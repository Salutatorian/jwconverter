//! Image conversion lifecycle: validate → temp → Magick → verify → finalize.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use serde::Serialize;

use crate::engine::image_job::{ImageConversionJob, ImageOutputFormat};
use crate::engine::job::{JobStatus, OverwritePolicy};
use crate::engine::runner::{ActiveProcess, RunCallbacks};
use crate::errors::AppError;
use crate::fs_safety::{finalize, temp};
use crate::media::imagemagick;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageConversionResult {
    pub job_id: String,
    pub output_path: String,
    pub status: JobStatus,
}

pub fn run_job(
    job: &ImageConversionJob,
    active: &ActiveProcess,
    callbacks: &RunCallbacks,
) -> Result<ImageConversionResult, AppError> {
    let source = PathBuf::from(&job.source_path);
    let destination_root = PathBuf::from(&job.destination_dir);
    let destination_dir =
        resolve_destination_dir(&destination_root, job.relative_subdir.as_deref())?;

    validate_source(&source)?;
    ensure_destination_dir(&destination_dir)?;

    let extension = job.output_format.extension();
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string();

    let primary_path = finalize::primary_final_path(&destination_dir, &stem, extension);
    let final_path = match job.overwrite_policy {
        OverwritePolicy::Rename => {
            finalize::unique_final_path(&destination_dir, &stem, extension)
        }
        OverwritePolicy::Skip => {
            if primary_path.exists() {
                (callbacks.on_status)(JobStatus::Skipped);
                return Ok(ImageConversionResult {
                    job_id: job.id.clone(),
                    output_path: primary_path.to_string_lossy().into_owned(),
                    status: JobStatus::Skipped,
                });
            }
            primary_path.clone()
        }
        OverwritePolicy::Replace => primary_path.clone(),
    };

    if paths_equal_file(&source, &final_path)? {
        return Err(AppError::DestinationUnavailable {
            detail: "Output path matches the source file. Choose a different destination or format."
                .to_string(),
        });
    }

    let temp_path = temp::temp_output_path(&destination_dir, &stem, extension, &job.id)?;

    (callbacks.on_status)(JobStatus::Converting);
    (callbacks.on_progress)(Some(5.0));

    let child = imagemagick::start_conversion(
        &source,
        &temp_path,
        job.output_format,
        job.quality_preset,
        job.resize_preset,
    )?;
    {
        let mut guard = active.child.lock().map_err(|_| AppError::FfmpegFailure {
            detail: "Internal process lock error.".to_string(),
        })?;
        *guard = Some(child);
    }

    let run = imagemagick::wait_with_cancel(
        Arc::clone(&active.child),
        Arc::clone(&active.cancel_flag),
    )?;

    if run.cancelled || active.cancel_flag.load(Ordering::SeqCst) {
        let _ = std::fs::remove_file(&temp_path);
        (callbacks.on_status)(JobStatus::Cancelled);
        return Ok(ImageConversionResult {
            job_id: job.id.clone(),
            output_path: String::new(),
            status: JobStatus::Cancelled,
        });
    }

    if !run.success {
        let _ = std::fs::remove_file(&temp_path);
        return Err(AppError::DecodeFailure {
            detail: imagemagick::friendly_image_error(&job.source_path, &run.stderr_tail),
        });
    }

    (callbacks.on_status)(JobStatus::Verifying);
    (callbacks.on_progress)(Some(90.0));
    verify_image_output(&temp_path, job.output_format)?;

    let allow_replace = matches!(job.overwrite_policy, OverwritePolicy::Replace);
    finalize::finalize_output_with_policy(&temp_path, &final_path, allow_replace)?;
    let _ = std::fs::remove_file(&temp_path);

    (callbacks.on_status)(JobStatus::Completed);
    (callbacks.on_progress)(Some(100.0));

    Ok(ImageConversionResult {
        job_id: job.id.clone(),
        output_path: final_path.to_string_lossy().into_owned(),
        status: JobStatus::Completed,
    })
}

fn verify_image_output(path: &Path, format: ImageOutputFormat) -> Result<(), AppError> {
    if !path.is_file() {
        return Err(AppError::VerificationFailure {
            detail: "Output file was not created.".to_string(),
        });
    }
    let meta = std::fs::metadata(path).map_err(|error| AppError::VerificationFailure {
        detail: format!("Could not read output file: {error}"),
    })?;
    if meta.len() == 0 {
        return Err(AppError::VerificationFailure {
            detail: "Output file is empty.".to_string(),
        });
    }

    let info = imagemagick::analyze(path.to_string_lossy().as_ref())?;
    if let Some(actual) = info.format.as_deref() {
        if !format.matches_identified(actual) {
            return Err(AppError::VerificationFailure {
                detail: format!(
                    "Output format mismatch. Expected {}, got {actual}.",
                    format.magick_format()
                ),
            });
        }
    }
    Ok(())
}

fn validate_source(path: &Path) -> Result<(), AppError> {
    if !path.is_file() {
        return Err(AppError::SourceMissing {
            detail: format!("Source file not found: {}", path.display()),
        });
    }
    Ok(())
}

fn ensure_destination_dir(path: &Path) -> Result<(), AppError> {
    if path.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(path).map_err(|error| AppError::DestinationUnavailable {
        detail: format!("Cannot create destination folder: {error}"),
    })
}

fn resolve_destination_dir(
    destination_root: &Path,
    relative: Option<&str>,
) -> Result<PathBuf, AppError> {
    let Some(relative) = relative.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(destination_root.to_path_buf());
    };
    let mut dir = destination_root.to_path_buf();
    for part in relative.split(['/', '\\']) {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." || part.contains(':') {
            return Err(AppError::DestinationUnavailable {
                detail: "Invalid relative output folder.".to_string(),
            });
        }
        dir.push(part);
    }
    let root = destination_root
        .canonicalize()
        .unwrap_or_else(|_| destination_root.to_path_buf());
    let resolved = dir.canonicalize().unwrap_or(dir.clone());
    if !resolved.starts_with(&root) && !dir.starts_with(destination_root) {
        return Err(AppError::DestinationUnavailable {
            detail: "Resolved output folder escaped the destination root.".to_string(),
        });
    }
    Ok(dir)
}

fn paths_equal_file(a: &Path, b: &Path) -> Result<bool, AppError> {
    if a == b {
        return Ok(true);
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(ca), Ok(cb)) => Ok(ca == cb),
        _ => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jpeg_extension() {
        assert_eq!(ImageOutputFormat::Jpeg.extension(), "jpg");
    }
}
