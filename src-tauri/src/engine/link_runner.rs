//! Single public-link download lifecycle using the local yt-dlp executable.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::engine::job::{JobStatus, OverwritePolicy};
use crate::engine::link_job::{format_selector, LinkDownloadJob, LinkMediaMode};
use crate::engine::runner::ActiveProcess;
use crate::errors::AppError;
use crate::fs_safety::{finalize, temp};
use crate::media::ffmpeg;
use crate::media::ffprobe;
use crate::media::link_filename::sanitize_link_stem;
use crate::media::link_url::validate_media_url;
use crate::media::paths::{resolve_ffmpeg, resolve_ytdlp};

#[derive(Clone)]
pub struct LinkRunCallbacks {
    pub on_status: Arc<dyn Fn(JobStatus, &str) + Send + Sync>,
    pub on_progress: Arc<dyn Fn(Option<f64>) + Send + Sync>,
}

pub struct LinkDownloadResult {
    pub output_path: String,
    pub status: JobStatus,
}

pub fn run_job(
    job: &LinkDownloadJob,
    active: &ActiveProcess,
    callbacks: &LinkRunCallbacks,
) -> Result<LinkDownloadResult, AppError> {
    let url = validate_media_url(job.url.trim())?;
    if job.is_playlist {
        return Err(AppError::UnsupportedFormat {
            detail: "Playlist downloads are not available in the experimental Links feature."
                .to_string(),
        });
    }
    if job.is_live {
        return Err(AppError::UnsupportedFormat {
            detail: "Live recording is not available in the experimental Links feature."
                .to_string(),
        });
    }

    let destination_dir = PathBuf::from(job.destination_dir.trim());
    ensure_destination_dir(&destination_dir)?;
    let stem = sanitize_link_stem(job.title.as_deref().unwrap_or("download"));
    let temp_stem = temp::link_temp_stem(&stem, &job.id);
    let template = destination_dir.join(format!("{temp_stem}.%(ext)s"));

    (callbacks.on_status)(JobStatus::Converting, "Downloading media");
    (callbacks.on_progress)(Some(0.0));
    let child = start_download(job, url.as_str(), &template)?;
    {
        let mut guard = active.child.lock().map_err(|_| AppError::FfmpegFailure {
            detail: "Internal process lock error.".to_string(),
        })?;
        *guard = Some(child);
    }

    let result = wait_with_progress(active, callbacks)?;
    if active.cancel_flag.load(Ordering::SeqCst) || result.cancelled {
        cleanup_download_temps(&destination_dir, &temp_stem);
        return Err(AppError::ConversionCancelled);
    }
    if !result.success {
        cleanup_download_temps(&destination_dir, &temp_stem);
        return Err(AppError::DecodeFailure {
            detail: ytdlp_error(&result.stderr),
        });
    }

    let temp_path = find_download_output(&destination_dir, &temp_stem)?;
    (callbacks.on_status)(JobStatus::Verifying, "Verifying downloaded media");
    (callbacks.on_progress)(Some(99.0));
    verify_output(&temp_path, job.mode)?;

    let extension = temp_path
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| !extension.is_empty())
        .ok_or_else(|| AppError::VerificationFailure {
            detail: "Downloaded output has no file extension.".to_string(),
        })?;
    let primary_path = finalize::primary_final_path(&destination_dir, &stem, extension);
    let final_path = match job.overwrite_policy {
        OverwritePolicy::Rename => finalize::unique_final_path(&destination_dir, &stem, extension),
        OverwritePolicy::Skip if primary_path.exists() => {
            temp::cleanup_temp(&temp_path);
            (callbacks.on_status)(JobStatus::Skipped, "Existing output left unchanged");
            return Ok(LinkDownloadResult {
                output_path: primary_path.to_string_lossy().into_owned(),
                status: JobStatus::Skipped,
            });
        }
        OverwritePolicy::Skip | OverwritePolicy::Replace => primary_path,
    };

    let allow_replace = matches!(job.overwrite_policy, OverwritePolicy::Replace);
    finalize::finalize_output_with_policy(&temp_path, &final_path, allow_replace).map_err(
        |error| {
            temp::cleanup_temp(&temp_path);
            error
        },
    )?;
    cleanup_download_temps(&destination_dir, &temp_stem);
    (callbacks.on_status)(JobStatus::Completed, "Download completed");
    (callbacks.on_progress)(Some(100.0));
    Ok(LinkDownloadResult {
        output_path: final_path.to_string_lossy().into_owned(),
        status: JobStatus::Completed,
    })
}

fn start_download(
    job: &LinkDownloadJob,
    url: &str,
    output_template: &Path,
) -> Result<std::process::Child, AppError> {
    let ytdlp = resolve_ytdlp().map_err(|detail| AppError::MediaToolMissing { detail })?;
    let ffmpeg = resolve_ffmpeg().ok_or_else(|| AppError::MediaToolMissing {
        detail:
            "FFmpeg was not found. Links downloads need FFmpeg for media merging and extraction."
                .to_string(),
    })?;
    let ffmpeg_dir = ffmpeg.parent().ok_or_else(|| AppError::MediaToolMissing {
        detail: "Could not determine the FFmpeg folder.".to_string(),
    })?;

    let mut command = Command::new(ytdlp);
    command
        .arg("--no-playlist")
        .arg("--newline")
        .arg("--no-call-home")
        .arg("--ffmpeg-location")
        .arg(ffmpeg_dir)
        .arg("-f")
        .arg(format_selector(job))
        .arg("-o")
        .arg(output_template);

    match job.mode {
        LinkMediaMode::Video => {
            command.arg("--merge-output-format").arg("mp4");
        }
        LinkMediaMode::Audio => {
            command
                .arg("-x")
                .arg("--audio-format")
                .arg(job.audio_format.ytdlp_format());
        }
    }

    command
        .arg(url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.spawn().map_err(|error| AppError::MediaToolMissing {
        detail: format!("Could not start yt-dlp: {error}"),
    })
}

struct DownloadWaitResult {
    success: bool,
    cancelled: bool,
    stderr: String,
}

fn wait_with_progress(
    active: &ActiveProcess,
    callbacks: &LinkRunCallbacks,
) -> Result<DownloadWaitResult, AppError> {
    let (stdout, stderr) = {
        let mut guard = active.child.lock().map_err(|_| AppError::FfmpegFailure {
            detail: "Internal process lock error.".to_string(),
        })?;
        let child = guard.as_mut().ok_or_else(|| AppError::FfmpegFailure {
            detail: "Download process missing.".to_string(),
        })?;
        (child.stdout.take(), child.stderr.take())
    };
    let stderr_text = Arc::new(Mutex::new(String::new()));
    let progress = Arc::clone(&callbacks.on_progress);
    let stdout_thread = std::thread::spawn(move || read_ytdlp_lines(stdout, &progress, None));
    let stderr_writer = Arc::clone(&stderr_text);
    let progress = Arc::clone(&callbacks.on_progress);
    let stderr_thread =
        std::thread::spawn(move || read_ytdlp_lines(stderr, &progress, Some(stderr_writer)));

    let status = loop {
        if active.cancel_flag.load(Ordering::SeqCst) {
            ffmpeg::kill_child(&active.child);
            break None;
        }
        let status = {
            let mut guard = active.child.lock().map_err(|_| AppError::FfmpegFailure {
                detail: "Internal process lock error.".to_string(),
            })?;
            match guard.as_mut() {
                Some(child) => child.try_wait().map_err(|error| AppError::FfmpegFailure {
                    detail: format!("Could not wait for yt-dlp: {error}"),
                })?,
                None => None,
            }
        };
        if status.is_some() {
            break status;
        }
        std::thread::sleep(Duration::from_millis(75));
    };
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();
    let stderr = stderr_text
        .lock()
        .map(|value| value.clone())
        .unwrap_or_default();
    if let Ok(mut guard) = active.child.lock() {
        *guard = None;
    }
    Ok(DownloadWaitResult {
        success: status.is_some_and(|status| status.success()),
        cancelled: active.cancel_flag.load(Ordering::SeqCst),
        stderr,
    })
}

fn read_ytdlp_lines(
    stream: Option<impl std::io::Read>,
    on_progress: &Arc<dyn Fn(Option<f64>) + Send + Sync>,
    stderr: Option<Arc<Mutex<String>>>,
) {
    let Some(stream) = stream else {
        return;
    };
    for line in BufReader::new(stream).lines().map_while(Result::ok) {
        if let Some(percent) = parse_download_percent(&line) {
            on_progress(Some(percent));
        }
        if let Some(stderr) = &stderr {
            if let Ok(mut value) = stderr.lock() {
                if value.len() < 8_000 {
                    value.push_str(&line);
                    value.push('\n');
                }
            }
        }
    }
}

fn parse_download_percent(line: &str) -> Option<f64> {
    let marker = "[download]";
    let remainder = line.strip_prefix(marker)?.trim_start();
    let raw_percent = remainder.split_whitespace().next()?.strip_suffix('%')?;
    let percent = raw_percent.parse::<f64>().ok()?;
    (0.0..=100.0).contains(&percent).then_some(percent)
}

fn find_download_output(destination_dir: &Path, temp_stem: &str) -> Result<PathBuf, AppError> {
    std::fs::read_dir(destination_dir)
        .map_err(|error| AppError::DestinationUnavailable {
            detail: format!("Could not read destination folder: {error}"),
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.starts_with(temp_stem)
                            && !name.ends_with(".part")
                            && !name.ends_with(".ytdl")
                    })
        })
        .ok_or_else(|| AppError::VerificationFailure {
            detail: "yt-dlp completed but did not create a usable output file.".to_string(),
        })
}

fn verify_output(path: &Path, mode: LinkMediaMode) -> Result<(), AppError> {
    let metadata = std::fs::metadata(path).map_err(|error| AppError::VerificationFailure {
        detail: format!("Could not inspect downloaded output: {error}"),
    })?;
    if metadata.len() == 0 {
        return Err(AppError::VerificationFailure {
            detail: "Downloaded output is empty.".to_string(),
        });
    }
    let streams = ffprobe::stream_types(path)?;
    let required = match mode {
        LinkMediaMode::Video => "video",
        LinkMediaMode::Audio => "audio",
    };
    if !streams.iter().any(|stream| stream == required) {
        return Err(AppError::VerificationFailure {
            detail: format!("Downloaded output has no {required} stream."),
        });
    }
    Ok(())
}

fn cleanup_download_temps(destination_dir: &Path, temp_stem: &str) {
    if let Ok(entries) = std::fs::read_dir(destination_dir) {
        for path in entries.filter_map(Result::ok).map(|entry| entry.path()) {
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(temp_stem))
            {
                temp::cleanup_temp(&path);
            }
        }
    }
}

fn ensure_destination_dir(path: &Path) -> Result<(), AppError> {
    if path.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(path).map_err(|error| AppError::DestinationUnavailable {
        detail: format!("Cannot create destination folder: {error}"),
    })
}

fn ytdlp_error(stderr: &str) -> String {
    let message = stderr
        .lines()
        .rev()
        .find(|line| line.contains("ERROR:"))
        .unwrap_or("yt-dlp could not download this media.");
    format!("Download failed. {message}")
}

#[cfg(test)]
mod tests {
    use super::parse_download_percent;

    #[test]
    fn parses_ytdlp_download_progress() {
        assert_eq!(
            parse_download_percent("[download]  45.2% of 3.00MiB"),
            Some(45.2)
        );
        assert_eq!(parse_download_percent("[download]  45.2%"), Some(45.2));
        assert_eq!(parse_download_percent("other output"), None);
    }
}
