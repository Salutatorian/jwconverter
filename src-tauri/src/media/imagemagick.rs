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
        .arg("-auto-orient")
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
            detail: friendly_image_error(path, &err),
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
    // Apply EXIF Orientation so phone photos don't land sideways.
    command.arg("-auto-orient");

    if let Some(geometry) = resize.magick_geometry() {
        command.arg("-resize").arg(geometry);
    }

    let quality = quality.normalize_for(format);
    if format == ImageOutputFormat::Webp && quality.is_lossless() {
        command.arg("-define").arg("webp:lossless=true");
    } else if let Some(q) = quality.magick_quality_for(format) {
        command.arg("-quality").arg(q.to_string());
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

/// Camera RAW extensions we accept as inputs (Magick/LibRaw when available).
fn is_likely_raw_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            matches!(
                e.to_ascii_lowercase().as_str(),
                "cr2" | "cr3" | "nef" | "arw" | "dng" | "orf" | "rw2" | "raf" | "pef" | "srw"
            )
        })
        .unwrap_or(false)
}

fn first_stderr_line(stderr: &str) -> Option<&str> {
    stderr
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
}

/// User-facing decode/convert failure text (honest about RAW / LibRaw limits).
pub fn friendly_image_error(path: &str, stderr: &str) -> String {
    let tip = first_stderr_line(stderr).unwrap_or("");
    let lower = stderr.to_ascii_lowercase();
    let looks_raw = is_likely_raw_path(path)
        || lower.contains("libraw")
        || lower.contains("dng:")
        || lower.contains("cr2:")
        || lower.contains("nef:");
    let no_delegate = lower.contains("no decode delegate")
        || lower.contains("unsupported file format");

    if looks_raw {
        let mut msg = "Couldn't decode this camera RAW file. The bundled ImageMagick/LibRaw may not support this camera model, or the file may be corrupt.".to_string();
        if !tip.is_empty() {
            msg.push_str(" (");
            msg.push_str(tip);
            msg.push(')');
        }
        return msg;
    }

    if no_delegate {
        return if tip.is_empty() {
            "Couldn't read this image — format not supported by the bundled ImageMagick."
                .to_string()
        } else {
            format!("Couldn't read this image: {tip}")
        };
    }

    if tip.is_empty() {
        "ImageMagick could not process this image.".to_string()
    } else {
        format!("Could not process image: {tip}")
    }
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
    fn raw_failure_message_is_honest() {
        let msg = friendly_image_error(
            r"C:\photos\IMG_0001.CR2",
            "magick: no decode delegate for this image format `CR2' @ error/constitute.c/ReadImage/1000",
        );
        assert!(msg.contains("camera RAW"));
        assert!(msg.contains("LibRaw"));
    }

    #[test]
    fn non_raw_keeps_short_tip() {
        let msg = friendly_image_error(
            r"C:\photos\broken.png",
            "magick: improper image header `broken.png' @ error/png.c/ReadPNGImage/100",
        );
        assert!(msg.contains("improper image header"));
        assert!(!msg.contains("camera RAW"));
        assert!(msg.starts_with("Could not process image:"));
    }

    #[test]
    fn auto_orient_swaps_phone_jpeg_dimensions() {
        let Some(magick_path) = resolve_magick() else {
            eprintln!("skip: magick not available");
            return;
        };

        let dir = std::env::temp_dir().join(format!("jw-orient-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("tmpdir");
        // 40×20 with Orientation=RightTop; after -auto-orient pixels should be 20×40.
        // TIFF stores orientation reliably (this Magick build strips it on JPEG write).
        let src = dir.join("phone.tif");
        let out = dir.join("out.jpg");

        let status = std::process::Command::new(&magick_path)
            .args(["-size", "40x20", "xc:red", "-orient", "RightTop"])
            .arg(&src)
            .status()
            .expect("create oriented tiff");
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

        let info = analyze(out.to_string_lossy().as_ref()).expect("identify");
        assert_eq!(info.width, Some(20));
        assert_eq!(info.height, Some(40));
    }

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

    #[test]
    fn converts_png_to_webp_lossless() {
        let Some(magick_path) = resolve_magick() else {
            eprintln!("skip: magick not available");
            return;
        };

        let dir = std::env::temp_dir().join(format!("jw-webp-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("tmpdir");
        let src = dir.join("src.png");
        let out = dir.join("out.webp");

        let status = std::process::Command::new(&magick_path)
            .args(["-size", "8x8", "xc:green"])
            .arg(&src)
            .status()
            .expect("create png");
        assert!(status.success());

        let child = start_conversion(
            &src,
            &out,
            ImageOutputFormat::Webp,
            ImageQualityPreset::Lossless,
            ImageResizePreset::Original,
        )
        .expect("start");
        let child = Arc::new(Mutex::new(Some(child)));
        let cancel = Arc::new(AtomicBool::new(false));
        let result = wait_with_cancel(child, cancel).expect("wait");
        assert!(result.success);
        assert!(out.is_file());

        let info = analyze(out.to_string_lossy().as_ref()).expect("identify");
        assert_eq!(info.format.as_deref(), Some("WEBP"));
    }

    #[test]
    fn converts_png_to_bmp_gif_avif() {
        let Some(magick_path) = resolve_magick() else {
            eprintln!("skip: magick not available");
            return;
        };

        let dir = std::env::temp_dir().join(format!("jw-extra-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("tmpdir");
        let src = dir.join("src.png");

        let status = std::process::Command::new(&magick_path)
            .args(["-size", "8x8", "xc:orange"])
            .arg(&src)
            .status()
            .expect("create png");
        assert!(status.success());

        for (format, name) in [
            (ImageOutputFormat::Bmp, "out.bmp"),
            (ImageOutputFormat::Gif, "out.gif"),
            (ImageOutputFormat::Avif, "out.avif"),
        ] {
            let out = dir.join(name);
            let child = start_conversion(
                &src,
                &out,
                format,
                ImageQualityPreset::Medium,
                ImageResizePreset::Original,
            )
            .expect("start");
            let child = Arc::new(Mutex::new(Some(child)));
            let cancel = Arc::new(AtomicBool::new(false));
            let result = wait_with_cancel(child, cancel).expect("wait");
            assert!(result.success, "{name}: {}", result.stderr_tail);
            assert!(out.is_file());
            let info = analyze(out.to_string_lossy().as_ref()).expect("identify");
            assert!(
                format.matches_identified(info.format.as_deref().unwrap_or("")),
                "{name} got {:?}",
                info.format
            );
        }
    }
}
