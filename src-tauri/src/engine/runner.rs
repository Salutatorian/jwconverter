//! Orchestrates: validate → temp → convert → verify → finalize → cleanup.
//! Source files are never modified.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::errors::AppError;
use crate::fs_safety::{finalize, temp};
use crate::media::ffmpeg;

use super::job::{ConversionJob, JobStatus, OutputFormat, OverwritePolicy};
use super::planner::{self, EncoderPlan};
use super::verify::{self, VerificationContext};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionResult {
    pub job_id: String,
    pub output_path: String,
    pub status: JobStatus,
}

#[derive(Clone)]
pub struct RunCallbacks {
    pub on_status: Arc<dyn Fn(JobStatus) + Send + Sync>,
    pub on_progress: Arc<dyn Fn(Option<f64>) + Send + Sync>,
}

pub struct ActiveProcess {
    pub child: Arc<Mutex<Option<std::process::Child>>>,
    pub cancel_flag: Arc<AtomicBool>,
}

/// Run the safe conversion lifecycle for one job.
pub fn run_job(
    job: &ConversionJob,
    source_duration_seconds: Option<f64>,
    active: &ActiveProcess,
    callbacks: &RunCallbacks,
) -> Result<ConversionResult, AppError> {
    let source = PathBuf::from(&job.source_path);
    let destination_root = PathBuf::from(&job.destination_dir);
    let destination_dir = resolve_destination_dir(&destination_root, job.relative_subdir.as_deref())?;

    validate_source(&source)?;
    ensure_destination_dir(&destination_dir)?;

    let plan = planner::plan_for(job.output_format, job.quality_preset);
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string();

    let primary_path = finalize::primary_final_path(&destination_dir, &stem, plan.extension);

    let final_path = match job.overwrite_policy {
        OverwritePolicy::Rename => {
            finalize::unique_final_path(&destination_dir, &stem, plan.extension)
        }
        OverwritePolicy::Skip => {
            if primary_path.exists() {
                (callbacks.on_status)(JobStatus::Skipped);
                return Ok(ConversionResult {
                    job_id: job.id.clone(),
                    output_path: primary_path.to_string_lossy().into_owned(),
                    status: JobStatus::Skipped,
                });
            }
            primary_path.clone()
        }
        OverwritePolicy::Replace => primary_path.clone(),
    };

    // Never allow writing over the source file (especially Replace same-folder same-format).
    if paths_equal_file(&source, &final_path) {
        return Err(AppError::DestinationUnavailable {
            detail: "Output path matches the source file. Choose a different destination or format."
                .to_string(),
        });
    }

    let allow_replace = matches!(job.overwrite_policy, OverwritePolicy::Replace);

    let temp_path = temp::temp_output_path(&destination_dir, &stem, plan.extension, &job.id)?;

    // Ensure we never write the final name during encoding.
    if final_path == temp_path {
        return Err(AppError::DestinationUnavailable {
            detail: "Temporary and final paths collided.".to_string(),
        });
    }

    (callbacks.on_status)(JobStatus::Converting);

    let child = ffmpeg::start_conversion(&source, &temp_path, &plan)?;
    {
        let mut guard = active.child.lock().map_err(|_| AppError::FfmpegFailure {
            detail: "Internal process lock error.".to_string(),
        })?;
        *guard = Some(child);
    }

    let progress_cb = Arc::clone(&callbacks.on_progress);
    let wait_result = ffmpeg::wait_with_progress(
        Arc::clone(&active.child),
        Arc::clone(&active.cancel_flag),
        source_duration_seconds,
        move |percent| progress_cb(percent),
    );

    // Detach finished child handle.
    {
        if let Ok(mut guard) = active.child.lock() {
            *guard = None;
        }
    }

    if active.cancel_flag.load(Ordering::SeqCst) {
        temp::cleanup_temp(&temp_path);
        return Err(AppError::ConversionCancelled);
    }

    match wait_result {
        Ok(result) if result.cancelled => {
            temp::cleanup_temp(&temp_path);
            return Err(AppError::ConversionCancelled);
        }
        Ok(_) => {}
        Err(error) => {
            temp::cleanup_temp(&temp_path);
            return Err(error);
        }
    }

    if active.cancel_flag.load(Ordering::SeqCst) {
        temp::cleanup_temp(&temp_path);
        return Err(AppError::ConversionCancelled);
    }

    (callbacks.on_status)(JobStatus::Verifying);
    (callbacks.on_progress)(Some(99.0));

    let verify_result = verify::verify_output(
        &temp_path,
        &plan,
        &VerificationContext {
            source_duration_seconds,
        },
    );

    if let Err(error) = verify_result {
        temp::cleanup_temp(&temp_path);
        return Err(error);
    }

    finalize::finalize_output_with_policy(&temp_path, &final_path, allow_replace).map_err(
        |error| {
            temp::cleanup_temp(&temp_path);
            error
        },
    )?;

    // Source must remain untouched — we never open it for write.
    assert_source_still_exists(&source)?;

    (callbacks.on_status)(JobStatus::Completed);
    (callbacks.on_progress)(Some(100.0));

    Ok(ConversionResult {
        job_id: job.id.clone(),
        output_path: final_path.to_string_lossy().into_owned(),
        status: JobStatus::Completed,
    })
}

fn validate_source(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Err(AppError::SourceMissing {
            detail: format!("Source file not found: {}", path.display()),
        });
    }
    if !path.is_file() {
        return Err(AppError::UnsupportedFormat {
            detail: "Source path is not a file.".to_string(),
        });
    }
    Ok(())
}

/// Build destination folder from root + relative subdir without allowing path escape.
fn resolve_destination_dir(
    destination_root: &Path,
    relative_subdir: Option<&str>,
) -> Result<PathBuf, AppError> {
    let Some(subdir) = relative_subdir.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(destination_root.to_path_buf());
    };

    let mut dir = destination_root.to_path_buf();
    for part in subdir.split(['/', '\\']) {
        let part = part.trim();
        if part.is_empty() || part == "." || part == ".." {
            continue;
        }
        // Reject drive prefixes / absolute segments (e.g. "C:" resets PathBuf on Windows).
        if part.contains(':') || part.starts_with('\\') || Path::new(part).is_absolute() {
            return Err(AppError::DestinationUnavailable {
                detail: format!("Invalid relative folder segment: {part}"),
            });
        }
        dir.push(part);
    }

    // Soft containment check without requiring the path to exist yet.
    let root = destination_root
        .components()
        .collect::<Vec<_>>();
    let resolved = dir.components().collect::<Vec<_>>();
    if resolved.len() < root.len() || resolved[..root.len()] != root[..] {
        return Err(AppError::DestinationUnavailable {
            detail: "Resolved output folder escaped the destination root.".to_string(),
        });
    }

    Ok(dir)
}

fn paths_equal_file(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => {
            // Compare normalized string forms when either path does not exist yet.
            let na = a.to_string_lossy().replace('/', "\\").to_lowercase();
            let nb = b.to_string_lossy().replace('/', "\\").to_lowercase();
            na == nb
        }
    }
}

fn validate_destination_dir(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Err(AppError::DestinationUnavailable {
            detail: format!("Destination folder does not exist: {}", path.display()),
        });
    }
    if !path.is_dir() {
        return Err(AppError::DestinationUnavailable {
            detail: "Destination must be a folder.".to_string(),
        });
    }

    // Quick writability probe.
    let probe = path.join(format!(".jwconverter-write-test-{}", std::process::id()));
    match std::fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            Ok(())
        }
        Err(error) => Err(AppError::PermissionDenied {
            detail: format!("Cannot write to destination folder: {error}"),
        }),
    }
}

fn ensure_destination_dir(path: &Path) -> Result<(), AppError> {
    if path.exists() {
        return validate_destination_dir(path);
    }

    std::fs::create_dir_all(path).map_err(|error| AppError::DestinationUnavailable {
        detail: format!("Could not create output folder {}: {error}", path.display()),
    })?;
    validate_destination_dir(path)
}

fn assert_source_still_exists(path: &Path) -> Result<(), AppError> {
    if path.is_file() {
        Ok(())
    } else {
        Err(AppError::SourceMissing {
            detail:
                "Source file disappeared during conversion. Output was not trusted as complete."
                    .to_string(),
        })
    }
}

#[allow(dead_code)]
pub fn plan_for_format(format: OutputFormat) -> EncoderPlan {
    planner::plan_for(format, crate::engine::job::QualityPreset::Medium)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn fixture_path() -> Option<PathBuf> {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("tests")
            .join("fixtures")
            .join("tone-440hz.wav");
        if fixture.is_file() {
            Some(fixture)
        } else {
            eprintln!("skipping: fixture missing");
            None
        }
    }

    fn test_out_dir() -> PathBuf {
        let out_dir =
            std::env::temp_dir().join(format!("jwconverter-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&out_dir).expect("create out dir");
        out_dir
    }

    fn test_job(
        fixture: &Path,
        out_dir: &Path,
        format: OutputFormat,
        policy: OverwritePolicy,
    ) -> ConversionJob {
        ConversionJob {
            id: uuid::Uuid::new_v4().to_string(),
            source_path: fixture.to_string_lossy().into_owned(),
            destination_dir: out_dir.to_string_lossy().into_owned(),
            relative_subdir: None,
            output_format: format,
            overwrite_policy: policy,
            quality_preset: crate::engine::job::QualityPreset::Medium,
            status: JobStatus::Queued,
        }
    }

    fn active_and_callbacks() -> (ActiveProcess, RunCallbacks) {
        let active = ActiveProcess {
            child: Arc::new(Mutex::new(None)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        };
        let callbacks = RunCallbacks {
            on_status: Arc::new(|_| {}),
            on_progress: Arc::new(|_| {}),
        };
        (active, callbacks)
    }

    fn count_files(dir: &Path) -> usize {
        std::fs::read_dir(dir)
            .expect("read out dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_file())
            .count()
    }

    #[test]
    fn resolve_destination_dir_rejects_drive_prefix() {
        let root = PathBuf::from(r"D:\Music\Out");
        let err = resolve_destination_dir(&root, Some(r"C:\Windows"))
            .expect_err("drive prefix must fail");
        assert!(err.to_string().contains("Invalid relative folder"));
    }

    #[test]
    fn resolve_destination_dir_keeps_nested_relative() {
        let root = PathBuf::from(r"D:\Music\Out");
        let resolved = resolve_destination_dir(&root, Some(r"Album\Disc 1")).expect("ok");
        assert_eq!(resolved, root.join("Album").join("Disc 1"));
    }

    #[test]
    fn paths_equal_file_matches_identical_paths() {
        let a = PathBuf::from(r"C:\music\song.wav");
        let b = PathBuf::from(r"C:\music\song.wav");
        assert!(paths_equal_file(&a, &b));
    }

    #[test]
    fn converts_wav_to_flac_without_touching_source() {
        let Some(fixture) = fixture_path() else {
            return;
        };

        let out_dir = test_out_dir();
        let source_bytes = std::fs::read(&fixture).expect("read source");
        let source_meta = std::fs::metadata(&fixture).expect("source meta");

        let job = test_job(
            &fixture,
            &out_dir,
            OutputFormat::Flac,
            OverwritePolicy::Rename,
        );
        let (active, callbacks) = active_and_callbacks();

        let result = run_job(&job, Some(2.0), &active, &callbacks).expect("convert");
        assert_eq!(result.status, JobStatus::Completed);

        let output = PathBuf::from(&result.output_path);
        assert!(output.is_file());
        assert!(output.metadata().expect("out meta").len() > 0);

        // Source untouched.
        let after = std::fs::read(&fixture).expect("reread source");
        assert_eq!(after, source_bytes);
        let after_meta = std::fs::metadata(&fixture).expect("source meta after");
        assert_eq!(after_meta.len(), source_meta.len());

        // Cleanup test artifacts.
        let _ = std::fs::remove_file(&output);
        let _ = std::fs::remove_dir_all(&out_dir);
        let _ = Duration::from_millis(1);
    }

    #[test]
    fn skip_when_primary_exists() {
        let Some(fixture) = fixture_path() else {
            return;
        };

        let out_dir = test_out_dir();
        let primary = out_dir.join("tone-440hz.flac");
        let existing_content = b"existing flac placeholder";
        std::fs::write(&primary, existing_content).expect("write existing primary");

        let job = test_job(
            &fixture,
            &out_dir,
            OutputFormat::Flac,
            OverwritePolicy::Skip,
        );
        let (active, callbacks) = active_and_callbacks();

        let result = run_job(&job, Some(2.0), &active, &callbacks).expect("skip");
        assert_eq!(result.status, JobStatus::Skipped);
        assert_eq!(result.output_path, primary.to_string_lossy());

        let content = std::fs::read(&primary).expect("read primary");
        assert_eq!(content, existing_content);
        assert_eq!(count_files(&out_dir), 1);

        let _ = std::fs::remove_file(&primary);
        let _ = std::fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn rename_when_primary_exists() {
        let Some(fixture) = fixture_path() else {
            return;
        };

        let out_dir = test_out_dir();
        let primary = out_dir.join("tone-440hz.flac");
        let existing_content = b"existing flac placeholder";
        std::fs::write(&primary, existing_content).expect("write existing primary");

        let job = test_job(
            &fixture,
            &out_dir,
            OutputFormat::Flac,
            OverwritePolicy::Rename,
        );
        let (active, callbacks) = active_and_callbacks();

        let result = run_job(&job, Some(2.0), &active, &callbacks).expect("rename");
        assert_eq!(result.status, JobStatus::Completed);
        assert_eq!(
            result.output_path,
            out_dir.join("tone-440hz (1).flac").to_string_lossy()
        );

        let primary_content = std::fs::read(&primary).expect("read primary");
        assert_eq!(primary_content, existing_content);

        let renamed = PathBuf::from(&result.output_path);
        assert!(renamed.is_file());
        assert!(renamed.metadata().expect("renamed meta").len() > 0);

        let _ = std::fs::remove_file(&primary);
        let _ = std::fs::remove_file(&renamed);
        let _ = std::fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn replace_when_primary_exists() {
        let Some(fixture) = fixture_path() else {
            return;
        };

        let out_dir = test_out_dir();
        let primary = out_dir.join("tone-440hz.flac");
        let existing_content = b"existing flac placeholder";
        std::fs::write(&primary, existing_content).expect("write existing primary");

        let source_bytes = std::fs::read(&fixture).expect("read source");
        let source_meta = std::fs::metadata(&fixture).expect("source meta");

        let job = test_job(
            &fixture,
            &out_dir,
            OutputFormat::Flac,
            OverwritePolicy::Replace,
        );
        let (active, callbacks) = active_and_callbacks();

        let result = run_job(&job, Some(2.0), &active, &callbacks).expect("replace");
        assert_eq!(result.status, JobStatus::Completed);
        assert_eq!(result.output_path, primary.to_string_lossy());

        let replaced_content = std::fs::read(&primary).expect("read replaced");
        assert_ne!(replaced_content, existing_content);
        assert!(replaced_content.len() > 0);

        let after = std::fs::read(&fixture).expect("reread source");
        assert_eq!(after, source_bytes);
        let after_meta = std::fs::metadata(&fixture).expect("source meta after");
        assert_eq!(after_meta.len(), source_meta.len());

        let _ = std::fs::remove_file(&primary);
        let _ = std::fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn converts_fixture_to_each_new_format() {
        let Some(fixture) = fixture_path() else {
            return;
        };

        let cases = [
            (OutputFormat::Aac, "m4a"),
            (OutputFormat::Opus, "opus"),
            (OutputFormat::Ogg, "ogg"),
            (OutputFormat::Alac, "m4a"),
            (OutputFormat::Aiff, "aiff"),
        ];

        for (format, extension) in cases {
            let out_dir = test_out_dir();
            let job = test_job(&fixture, &out_dir, format, OverwritePolicy::Rename);
            let (active, callbacks) = active_and_callbacks();
            let result = run_job(&job, Some(2.0), &active, &callbacks)
                .unwrap_or_else(|error| panic!("convert {format:?} failed: {error}"));
            assert_eq!(result.status, JobStatus::Completed, "{format:?}");
            let output = PathBuf::from(&result.output_path);
            assert!(output.is_file(), "{format:?}");
            assert!(
                output
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case(extension)),
                "{format:?} extension, got {}",
                output.display()
            );
            assert!(output.metadata().expect("meta").len() > 0, "{format:?}");
            let _ = std::fs::remove_dir_all(&out_dir);
        }
    }
}
