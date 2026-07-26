//! ImageMagick process execution (argv arrays only).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::engine::image_job::{ImageOutputFormat, ImageQualityPreset, ImageResizePreset};
use crate::errors::AppError;
use crate::media::magick_policy;
use crate::media::paths::resolve_magick;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInfo {
    pub path: String,
    pub filename: String,
    pub format: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub file_size_bytes: Option<u64>,
}

pub struct MagickRunResult {
    pub success: bool,
    pub cancelled: bool,
    pub stderr_tail: String,
}

pub fn resolve_magick_required() -> Result<PathBuf, AppError> {
    resolve_magick().ok_or_else(|| AppError::MediaToolMissing {
        detail:
            "ImageMagick was not found. Place magick.exe in src-tauri/binaries/imagemagick/ or set CONVERTER_MAGICK."
                .to_string(),
    })
}

pub fn analyze(path: &str) -> Result<ImageInfo, AppError> {
    let magick = resolve_magick_required()?;
    let dir = magick_dir(&magick)?;
    magick_policy::ensure_policy_file(dir)?;

    let mut command = Command::new(&magick);
    command
        .env("MAGICK_CONFIGURE_PATH", dir)
        .arg("identify")
        .arg("-ping")
        .arg("-format")
        .arg("%w\\n%h\\n%m\\n%b")
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    apply_no_window(&mut command);

    let output = command.output().map_err(|error| AppError::MediaToolMissing {
        detail: format!("Failed to start ImageMagick identify: {error}"),
    })?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::DecodeFailure {
            detail: format!(
                "Could not read image: {}",
                err.lines().next().unwrap_or("unknown Magick error")
            ),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let width = lines.next().and_then(|s| s.trim().parse().ok());
    let height = lines.next().and_then(|s| s.trim().parse().ok());
    let format = lines
        .next()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let file_size_bytes = lines
        .next()
        .and_then(|s| parse_byte_size(s.trim()))
        .or_else(|| std::fs::metadata(path).ok().map(|m| m.len()));

    let path_buf = PathBuf::from(path);
    let filename = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    if width.is_none() || height.is_none() {
        return Err(AppError::DecodeFailure {
            detail: "Image has no readable dimensions.".to_string(),
        });
    }

    Ok(ImageInfo {
        path: path.to_string(),
        filename,
        format,
        width,
        height,
        file_size_bytes,
    })
}

pub fn start_conversion(
    source: &Path,
    temp_output: &Path,
    format: ImageOutputFormat,
    quality: ImageQualityPreset,
    resize: ImageResizePreset,
) -> Result<std::process::Child, AppError> {
    let magick = resolve_magick_required()?;
    let dir = magick_dir(&magick)?;
    magick_policy::ensure_policy_file(dir)?;

    let mut command = Command::new(&magick);
    command.env("MAGICK_CONFIGURE_PATH", dir);
    command.arg(source);

    if let Some(geometry) = resize.magick_geometry() {
        command.arg("-resize").arg(geometry);
    }

    if format.is_lossy() {
        command
            .arg("-quality")
            .arg(quality.magick_quality().to_string());
    }

    let out = format!("{}:{}", format.magick_format(), temp_output.display());
    command.arg(out);
    command.stdout(Stdio::null()).stderr(Stdio::piped());
    apply_no_window(&mut command);

    command.spawn().map_err(|error| AppError::MediaToolMissing {
        detail: format!("Failed to start ImageMagick: {error}"),
    })
}

pub fn wait_with_cancel(
    child: Arc<Mutex<Option<std::process::Child>>>,
    cancel_flag: Arc<AtomicBool>,
) -> Result<MagickRunResult, AppError> {
    loop {
        if cancel_flag.load(Ordering::SeqCst) {
            crate::media::ffmpeg::kill_child(&child);
            return Ok(MagickRunResult {
                success: false,
                cancelled: true,
                stderr_tail: String::new(),
            });
        }

        let finished = {
            let mut guard = child.lock().map_err(|_| AppError::FfmpegFailure {
                detail: "Internal process lock error.".to_string(),
            })?;
            let process = guard.as_mut().ok_or_else(|| AppError::FfmpegFailure {
                detail: "Conversion process missing.".to_string(),
            })?;
            match process.try_wait() {
                Ok(Some(status)) => Some(status.success()),
                Ok(None) => None,
                Err(error) => {
                    return Err(AppError::FfmpegFailure {
                        detail: format!("ImageMagick wait failed: {error}"),
                    });
                }
            }
        };

        if let Some(success) = finished {
            let stderr_tail = {
                let mut guard = child.lock().map_err(|_| AppError::FfmpegFailure {
                    detail: "Internal process lock error.".to_string(),
                })?;
                if let Some(mut process) = guard.take() {
                    let mut buf = String::new();
                    if let Some(mut stderr) = process.stderr.take() {
                        let _ = stderr.read_to_string(&mut buf);
                    }
                    if buf.len() > 2000 {
                        buf[buf.len() - 2000..].to_string()
                    } else {
                        buf
                    }
                } else {
                    String::new()
                }
            };
            return Ok(MagickRunResult {
                success,
                cancelled: false,
                stderr_tail,
            });
        }

        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn magick_dir(magick: &Path) -> Result<&Path, AppError> {
    magick.parent().ok_or_else(|| AppError::MediaToolMissing {
        detail: "ImageMagick path has no parent directory.".to_string(),
    })
}

fn parse_byte_size(raw: &str) -> Option<u64> {
    let trimmed = raw.trim();
    if let Ok(n) = trimmed.parse::<u64>() {
        return Some(n);
    }
    let digits: String = trimmed
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

fn apply_no_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};

    #[test]
    fn converts_png_fixture_to_jpeg() {
        let Some(magick_path) = resolve_magick() else {
            eprintln!("skip: magick not available");
            return;
        };

        let dir = std::env::temp_dir().join(format!("jw-img-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("tmpdir");
        let src = dir.join("src.png");
        let out = dir.join("out.jpg");

        let status = std::process::Command::new(&magick_path)
            .args(["-size", "16x16", "xc:blue"])
            .arg(&src)
            .status()
            .expect("create png");
        assert!(status.success());

        let child = start_conversion(
            &src,
            &out,
            ImageOutputFormat::Jpeg,
            ImageQualityPreset::Medium,
            ImageResizePreset::Original,
        )
        .expect("start");
        let child = Arc::new(Mutex::new(Some(child)));
        let cancel = Arc::new(AtomicBool::new(false));
        let result = wait_with_cancel(child, cancel).expect("wait");
        assert!(result.success);
        assert!(out.is_file());

        let info = analyze(out.to_string_lossy().as_ref()).expect("identify");
        assert_eq!(info.width, Some(16));
        assert_eq!(info.height, Some(16));
    }
}
