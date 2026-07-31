//! Single public-link download lifecycle using the local yt-dlp executable.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::engine::job::{ConversionJob, JobStatus, LoudnessPreset, NormalizeMode, OverwritePolicy};
use crate::engine::link_job::{
    format_selector, ytdlp_cookie_args, ytdlp_live_args, ytdlp_mode_args, ytdlp_subtitle_args,
    ytdlp_thumbnail_args, LinkDownloadJob, LinkMediaMode, LinkProcessingMode,
};
use crate::engine::runner::{self, ActiveProcess, RunCallbacks};
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
    if job.is_live && job.live_max_minutes.filter(|minutes| *minutes > 0).is_none() {
        return Err(AppError::UnsupportedFormat {
            detail: "Choose a live recording duration before downloading live media.".to_string(),
        });
    }

    let destination_dir = PathBuf::from(job.destination_dir.trim());
    ensure_destination_dir(&destination_dir)?;
    ensure_disk_space(&destination_dir)?;
    let _ = temp::cleanup_orphaned_link_temps(&destination_dir);
    let stem = sanitize_link_stem(job.title.as_deref().unwrap_or("download"));
    let temp_stem = temp::link_temp_stem(&stem, &job.id);
    let template = destination_dir.join(format!("{temp_stem}.%(ext)s"));

    let download_callbacks = LinkRunCallbacks {
        on_status: Arc::clone(&callbacks.on_status),
        on_progress: Arc::new({
            let on_progress = Arc::clone(&callbacks.on_progress);
            move |percent| {
                if let Some(percent) = percent {
                    on_progress(Some((percent * 0.7).clamp(0.0, 70.0)));
                } else {
                    on_progress(None);
                }
            }
        }),
    };

    (download_callbacks.on_status)(JobStatus::Converting, "Downloading media");
    (download_callbacks.on_progress)(Some(0.0));
    let child = start_download(job, url.as_str(), &template)?;
    {
        let mut guard = active.child.lock().map_err(|_| AppError::FfmpegFailure {
            detail: "Internal process lock error.".to_string(),
        })?;
        *guard = Some(child);
    }

    let result = wait_with_progress(active, &download_callbacks)?;
    if active.cancel_flag.load(Ordering::SeqCst) || result.cancelled {
        cleanup_download_temps(&destination_dir, &temp_stem);
        return Err(AppError::ConversionCancelled);
    }
    if !result.success {
        cleanup_download_temps(&destination_dir, &temp_stem);
        let (_category, mapped) = crate::media::link_errors::map_ytdlp_message(&result.stderr, true);
        return Err(if mapped.contains("free space") {
            AppError::DiskFull { detail: mapped }
        } else {
            AppError::DecodeFailure { detail: mapped }
        });
    }

    let temp_path = find_download_output(&destination_dir, &temp_stem)?;
    match job.processing_mode() {
        LinkProcessingMode::Remux => {
            finalize_remux(job, &destination_dir, &stem, &temp_stem, &temp_path, callbacks)
        }
        LinkProcessingMode::Transcode => {
            transcode_acquired(job, &destination_dir, &stem, &temp_stem, &temp_path, active, callbacks)
        }
    }
}

fn finalize_remux(
    job: &LinkDownloadJob,
    destination_dir: &Path,
    stem: &str,
    temp_stem: &str,
    temp_path: &Path,
    callbacks: &LinkRunCallbacks,
) -> Result<LinkDownloadResult, AppError> {
    (callbacks.on_status)(JobStatus::Verifying, "Verifying downloaded media");
    (callbacks.on_progress)(Some(95.0));
    verify_output(temp_path, job.mode)?;

    let extension = temp_path
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| !extension.is_empty())
        .ok_or_else(|| AppError::VerificationFailure {
            detail: "Downloaded output has no file extension.".to_string(),
        })?;
    let primary_path = finalize::primary_final_path(destination_dir, stem, extension);
    let final_path = match job.overwrite_policy {
        OverwritePolicy::Rename => finalize::unique_final_path(destination_dir, stem, extension),
        OverwritePolicy::Skip if primary_path.exists() => {
            temp::cleanup_temp(temp_path);
            cleanup_download_temps(destination_dir, temp_stem);
            (callbacks.on_status)(JobStatus::Skipped, "Existing output left unchanged");
            return Ok(LinkDownloadResult {
                output_path: primary_path.to_string_lossy().into_owned(),
                status: JobStatus::Skipped,
            });
        }
        OverwritePolicy::Skip | OverwritePolicy::Replace => primary_path,
    };

    let allow_replace = matches!(job.overwrite_policy, OverwritePolicy::Replace);
    finalize::finalize_output_with_policy(temp_path, &final_path, allow_replace).map_err(
        |error| {
            temp::cleanup_temp(temp_path);
            error
        },
    )?;
    move_thumbnail(destination_dir, temp_stem, &final_path, job.save_thumbnail)?;
    cleanup_download_temps(destination_dir, temp_stem);
    (callbacks.on_status)(JobStatus::Completed, "Download completed");
    (callbacks.on_progress)(Some(100.0));
    Ok(LinkDownloadResult {
        output_path: final_path.to_string_lossy().into_owned(),
        status: JobStatus::Completed,
    })
}

fn transcode_acquired(
    job: &LinkDownloadJob,
    destination_dir: &Path,
    stem: &str,
    temp_stem: &str,
    temp_path: &Path,
    active: &ActiveProcess,
    callbacks: &LinkRunCallbacks,
) -> Result<LinkDownloadResult, AppError> {
    let output_format = job.audio_format.output_format().ok_or_else(|| {
        AppError::UnsupportedFormat {
            detail: "Selected audio format cannot be transcoded.".to_string(),
        }
    })?;

    (callbacks.on_status)(JobStatus::Converting, "Transcoding audio");
    (callbacks.on_progress)(Some(70.0));

    let convert_callbacks = RunCallbacks {
        on_status: Arc::new({
            let on_status = Arc::clone(&callbacks.on_status);
            move |status| {
                let message = match status {
                    JobStatus::Converting => "Transcoding audio",
                    JobStatus::Verifying => "Verifying converted audio",
                    JobStatus::Skipped => "Existing output left unchanged",
                    JobStatus::Completed => "Download completed",
                    _ => "Processing audio",
                };
                on_status(status, message);
            }
        }),
        on_progress: Arc::new({
            let on_progress = Arc::clone(&callbacks.on_progress);
            move |percent| {
                if let Some(percent) = percent {
                    on_progress(Some((70.0 + percent * 0.3).clamp(70.0, 100.0)));
                }
            }
        }),
    };

    let conversion = ConversionJob {
        id: job.id.clone(),
        source_path: temp_path.to_string_lossy().into_owned(),
        destination_dir: destination_dir.to_string_lossy().into_owned(),
        relative_subdir: None,
        output_format,
        overwrite_policy: job.overwrite_policy,
        quality_preset: job.quality_preset,
        mp3_encoding_mode: job.mp3_encoding_mode,
        bit_depth_preset: job.bit_depth_preset,
        preserve_tags: true,
        preserve_cover: true,
        normalize: NormalizeMode::Off,
        loudness_preset: LoudnessPreset::Streaming,
        trim_silence: false,
        output_stem: Some(stem.to_string()),
        status: JobStatus::Queued,
    };

    let outcome = runner::run_job(
        &conversion,
        job.duration_seconds,
        active,
        &convert_callbacks,
    );
    cleanup_download_temps(destination_dir, temp_stem);
    let result = outcome?;
    Ok(LinkDownloadResult {
        output_path: result.output_path,
        status: result.status,
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

    for arg in ytdlp_mode_args(job) {
        command.arg(arg);
    }
    for arg in ytdlp_cookie_args(job) {
        command.arg(arg);
    }
    for arg in ytdlp_subtitle_args(job) {
        command.arg(arg);
    }
    for arg in ytdlp_thumbnail_args(job) {
        command.arg(arg);
    }
    for arg in ytdlp_live_args(job) {
        command.arg(arg);
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
    let last_emit = Arc::new(Mutex::new((std::time::Instant::now(), -1.0_f64)));
    let stdout_thread = std::thread::spawn({
        let last_emit = Arc::clone(&last_emit);
        move || read_ytdlp_lines(stdout, &progress, None, &last_emit)
    });
    let stderr_writer = Arc::clone(&stderr_text);
    let progress = Arc::clone(&callbacks.on_progress);
    let stderr_thread = std::thread::spawn({
        let last_emit = Arc::clone(&last_emit);
        move || read_ytdlp_lines(stderr, &progress, Some(stderr_writer), &last_emit)
    });

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
    last_emit: &Arc<Mutex<(std::time::Instant, f64)>>,
) {
    let Some(stream) = stream else {
        return;
    };
    for line in BufReader::new(stream).lines().map_while(Result::ok) {
        if let Some(percent) = parse_download_percent(&line) {
            let should_emit = last_emit
                .lock()
                .map(|mut state| {
                    let elapsed = state.0.elapsed() >= Duration::from_millis(250);
                    let jumped = (percent - state.1).abs() >= 1.0;
                    if elapsed || jumped || percent >= 100.0 {
                        state.0 = std::time::Instant::now();
                        state.1 = percent;
                        true
                    } else {
                        false
                    }
                })
                .unwrap_or(true);
            if should_emit {
                on_progress(Some(percent));
            }
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
                            && !is_thumbnail_extension(path)
                            && !is_subtitle_extension(path)
                    })
        })
        .ok_or_else(|| AppError::VerificationFailure {
            detail: "yt-dlp completed but did not create a usable output file.".to_string(),
        })
}

fn is_thumbnail_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("jpg" | "jpeg" | "png" | "webp")
    )
}

fn is_subtitle_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("ass" | "lrc" | "srt" | "ssa" | "ttml" | "vtt")
    )
}

fn move_thumbnail(
    destination_dir: &Path,
    temp_stem: &str,
    final_path: &Path,
    save_thumbnail: bool,
) -> Result<(), AppError> {
    if !save_thumbnail {
        return Ok(());
    }
    let Some(thumbnail) = std::fs::read_dir(destination_dir)
        .ok()
        .and_then(|entries| {
            entries.filter_map(Result::ok).map(|entry| entry.path()).find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(temp_stem) && is_thumbnail_extension(path))
            })
        })
    else {
        return Ok(());
    };
    let extension = thumbnail.extension().and_then(|value| value.to_str()).unwrap_or("jpg");
    let target = final_path.with_extension(extension);
    if thumbnail != target {
        std::fs::rename(&thumbnail, &target).map_err(|error| AppError::DestinationUnavailable {
            detail: format!("Could not save downloaded thumbnail: {error}"),
        })?;
    }
    Ok(())
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
    std::fs::create_dir_all(path).map_err(|error| {
        if is_disk_full_io(&error) {
            AppError::DiskFull {
                detail: "The destination drive does not have enough free space.".to_string(),
            }
        } else {
            AppError::DestinationUnavailable {
                detail: format!("Cannot create destination folder: {error}"),
            }
        }
    })
}

fn ensure_disk_space(destination_dir: &Path) -> Result<(), AppError> {
    let free = crate::engine::preflight::free_space_bytes(destination_dir)?;
    let required = crate::engine::preflight::disk_margin(0);
    if free < required {
        return Err(AppError::DiskFull {
            detail: format!(
                "The destination drive does not have enough free space (need about {} MB free).",
                required / (1024 * 1024)
            ),
        });
    }
    Ok(())
}

fn is_disk_full_io(error: &std::io::Error) -> bool {
    matches!(
        error.raw_os_error(),
        Some(28) | Some(112) // ENOSPC / ERROR_DISK_FULL
    ) || error.to_string().to_ascii_lowercase().contains("no space")
}

#[cfg(test)]
mod tests {
    use super::parse_download_percent;
    use crate::engine::job::{
        BitDepthPreset, JobStatus, Mp3EncodingMode, OverwritePolicy, QualityPreset,
    };
    use crate::engine::link_job::{
        format_selector, ytdlp_mode_args, LinkAudioFormat, LinkDownloadJob, LinkMediaMode,
        LinkProcessingMode, LinkVideoQuality,
    };

    fn audio_job(format: LinkAudioFormat) -> LinkDownloadJob {
        LinkDownloadJob {
            id: "job-123".to_string(),
            url: "https://example.com/video".to_string(),
            title: Some("Demo".into()),
            duration_seconds: Some(12.0),
            is_live: false,
            is_playlist: false,
            destination_dir: ".".to_string(),
            overwrite_policy: OverwritePolicy::Rename,
            mode: LinkMediaMode::Audio,
            video_quality: LinkVideoQuality::Best,
            audio_format: format,
            quality_preset: QualityPreset::High,
            mp3_encoding_mode: Mp3EncodingMode::Cbr,
            bit_depth_preset: BitDepthPreset::Original,
            cookies_path: None,
            download_subtitles: false,
            save_thumbnail: false,
            embed_thumbnail: false,
            live_max_minutes: None,
            status: JobStatus::Queued,
        }
    }

    #[test]
    fn parses_ytdlp_download_progress() {
        assert_eq!(
            parse_download_percent("[download]  45.2% of 3.00MiB"),
            Some(45.2)
        );
        assert_eq!(parse_download_percent("[download]  45.2%"), Some(45.2));
        assert_eq!(parse_download_percent("other output"), None);
    }

    #[test]
    fn original_audio_uses_remux_without_extract_flags() {
        let job = audio_job(LinkAudioFormat::Original);
        assert_eq!(job.processing_mode(), LinkProcessingMode::Remux);
        assert_eq!(format_selector(&job), "ba/b");
        assert!(ytdlp_mode_args(&job).is_empty());
    }

    #[test]
    fn transcode_audio_acquires_without_ytdlp_audio_format() {
        let job = audio_job(LinkAudioFormat::Mp3);
        assert_eq!(job.processing_mode(), LinkProcessingMode::Transcode);
        assert_eq!(format_selector(&job), "ba/b");
        assert!(ytdlp_mode_args(&job).is_empty());
    }
}
