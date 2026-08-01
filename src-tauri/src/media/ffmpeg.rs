//! Safe FFmpeg process execution (argv arrays only — never shell strings).

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::engine::planner::EncoderPlan;
use crate::errors::AppError;
use crate::media::progress::{parse_progress_line, percent_complete};

use super::paths::resolve_ffmpeg;

pub struct FfmpegRunResult {
    pub success: bool,
    pub cancelled: bool,
    pub stderr_tail: String,
}

pub fn resolve_ffmpeg_required() -> Result<PathBuf, AppError> {
    resolve_ffmpeg().ok_or_else(|| AppError::MediaToolMissing {
        detail:
            "FFmpeg was not found. Place ffmpeg.exe in src-tauri/binaries/ or set CONVERTER_FFMPEG."
                .to_string(),
    })
}

/// Spawn FFmpeg writing only to `temp_output`. Source is read-only input.
pub fn start_conversion(
    source: &Path,
    temp_output: &Path,
    plan: &EncoderPlan,
    preserve_tags: bool,
    preserve_cover: bool,
) -> Result<Child, AppError> {
    let ffmpeg = resolve_ffmpeg_required()?;

    let mut command = Command::new(&ffmpeg);
    command
        .arg("-hide_banner")
        .arg("-nostdin")
        .arg("-y")
        .arg("-protocol_whitelist")
        .arg("file,pipe,fd")
        .arg("-i")
        .arg(source)
        .arg("-map")
        .arg("0:a:0");

    let map_cover = preserve_cover && plan.format.supports_embedded_cover();
    if map_cover {
        // Optional attached-picture / cover stream (no-op when source has none).
        command.arg("-map").arg("0:V:0?");
        command.arg("-c:v").arg("copy");
        command.arg("-disposition:v:0").arg("attached_pic");
    } else {
        command.arg("-vn");
    }

    for arg in plan.ffmpeg_audio_args() {
        command.arg(arg);
    }

    if preserve_tags {
        command.arg("-map_metadata").arg("0");
        command.arg("-map_chapters").arg("0");
    } else {
        command.arg("-map_metadata").arg("-1");
    }

    if matches!(plan.format, crate::engine::job::OutputFormat::Mp3) {
        command.arg("-id3v2_version").arg("3");
    }

    command
        .arg("-progress")
        .arg("pipe:1")
        .arg("-nostats")
        .arg(temp_output);

    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command.spawn().map_err(|error| AppError::MediaToolMissing {
        detail: format!("Failed to start FFmpeg: {error}"),
    })
}

pub fn wait_with_progress<F>(
    child: Arc<Mutex<Option<Child>>>,
    cancel_flag: Arc<AtomicBool>,
    duration_seconds: Option<f64>,
    mut on_progress: F,
) -> Result<FfmpegRunResult, AppError>
where
    F: FnMut(Option<f64>),
{
    let stdout = {
        let mut guard = child.lock().map_err(|_| AppError::FfmpegFailure {
            detail: "Internal process lock error.".to_string(),
        })?;
        let process = guard.as_mut().ok_or_else(|| AppError::FfmpegFailure {
            detail: "Conversion process missing.".to_string(),
        })?;
        process.stdout.take()
    };

    let stderr = {
        let mut guard = child.lock().map_err(|_| AppError::FfmpegFailure {
            detail: "Internal process lock error.".to_string(),
        })?;
        let process = guard.as_mut().ok_or_else(|| AppError::FfmpegFailure {
            detail: "Conversion process missing.".to_string(),
        })?;
        process.stderr.take()
    };

    let stderr_tail = Arc::new(Mutex::new(String::new()));
    let stderr_tail_writer = Arc::clone(&stderr_tail);

    let stderr_thread = std::thread::spawn(move || {
        if let Some(stderr) = stderr {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                if let Ok(mut buffer) = stderr_tail_writer.lock() {
                    if buffer.len() < 8_000 {
                        buffer.push_str(&line);
                        buffer.push('\n');
                    }
                }
            }
        }
    });

    let mut last_emit = Instant::now()
        .checked_sub(std::time::Duration::from_millis(200))
        .unwrap_or_else(Instant::now);
    let mut last_percent: Option<f64> = None;

    if let Some(stdout) = stdout {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if cancel_flag.load(Ordering::SeqCst) {
                kill_child(&child);
                break;
            }

            let update = parse_progress_line(&line);
            if let Some(ms) = update.out_time_ms {
                if let Some(percent) = percent_complete(ms, duration_seconds) {
                    let should_emit = last_percent.is_none_or(|prev| (percent - prev).abs() >= 0.5)
                        || last_emit.elapsed() >= std::time::Duration::from_millis(100);
                    if should_emit {
                        on_progress(Some(percent));
                        last_percent = Some(percent);
                        last_emit = Instant::now();
                    }
                } else {
                    on_progress(None);
                }
            }
        }
    }

    if cancel_flag.load(Ordering::SeqCst) {
        kill_child(&child);
    }

    let status = {
        let mut guard = child.lock().map_err(|_| AppError::FfmpegFailure {
            detail: "Internal process lock error.".to_string(),
        })?;
        match guard.as_mut() {
            Some(process) => process.wait().map_err(|error| AppError::FfmpegFailure {
                detail: format!("Failed while waiting for FFmpeg: {error}"),
            })?,
            None => {
                return Ok(FfmpegRunResult {
                    success: false,
                    cancelled: true,
                    stderr_tail: String::new(),
                });
            }
        }
    };

    let _ = stderr_thread.join();
    let stderr_text = stderr_tail.lock().map(|s| s.clone()).unwrap_or_default();

    if cancel_flag.load(Ordering::SeqCst) {
        return Ok(FfmpegRunResult {
            success: false,
            cancelled: true,
            stderr_tail: stderr_text,
        });
    }

    if !status.success() {
        let detail = if stderr_text.trim().is_empty() {
            "FFmpeg failed to convert this file.".to_string()
        } else {
            let tail = stderr_text.lines().rev().take(6).collect::<Vec<_>>();
            let joined = tail.into_iter().rev().collect::<Vec<_>>().join(" ");
            format!("Conversion failed. {joined}")
        };
        return Err(AppError::FfmpegFailure { detail });
    }

    Ok(FfmpegRunResult {
        success: true,
        cancelled: false,
        stderr_tail: stderr_text,
    })
}

pub fn kill_child(child: &Arc<Mutex<Option<Child>>>) {
    if let Ok(mut guard) = child.lock() {
        if let Some(process) = guard.as_mut() {
            let _ = process.kill();
            let _ = process.wait();
        }
        *guard = None;
    }
}

/// Embed a JPEG/PNG cover into an audio container that supports attached pictures.
/// No-op when the media format cannot carry cover art.
pub fn embed_cover_image(media: &Path, cover: &Path) -> Result<(), AppError> {
    if !path_supports_embedded_cover(media) || !cover.is_file() || !media.is_file() {
        return Ok(());
    }
    let ffmpeg = resolve_ffmpeg_required()?;
    let extension = media
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("m4a");
    let temp_out = media.with_extension(format!("jwcover.tmp.{extension}"));

    let mut command = Command::new(&ffmpeg);
    command
        .arg("-hide_banner")
        .arg("-nostdin")
        .arg("-y")
        .arg("-protocol_whitelist")
        .arg("file,pipe,fd")
        .arg("-i")
        .arg(media)
        .arg("-i")
        .arg(cover)
        .arg("-map")
        .arg("0:a:0")
        .arg("-map")
        .arg("1:0")
        .arg("-c")
        .arg("copy")
        .arg("-c:v")
        .arg("mjpeg")
        .arg("-disposition:v:0")
        .arg("attached_pic")
        .arg("-map_metadata")
        .arg("0");

    if extension.eq_ignore_ascii_case("mp3") {
        command.arg("-id3v2_version").arg("3");
    }

    command.arg(&temp_out);
    command.stdout(Stdio::null()).stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = command.output().map_err(|error| AppError::MediaToolMissing {
        detail: format!("Failed to start FFmpeg for cover embed: {error}"),
    })?;
    if !output.status.success() {
        let _ = std::fs::remove_file(&temp_out);
        return Ok(());
    }
    std::fs::rename(&temp_out, media).map_err(|error| {
        let _ = std::fs::remove_file(&temp_out);
        AppError::DestinationUnavailable {
            detail: format!("Could not write embedded cover art: {error}"),
        }
    })?;
    Ok(())
}

fn path_supports_embedded_cover(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .as_deref(),
        Some("mp3" | "m4a" | "flac" | "ogg" | "opus" | "alac")
    )
}
