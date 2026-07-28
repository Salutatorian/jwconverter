//! Resolve final destination paths and atomically finalize temp → final.

use std::path::{Path, PathBuf};

use crate::errors::AppError;

use super::temp::{cleanup_temp, is_our_temp_file};

/// Choose a final path. Never silently overwrite — auto-rename instead.
pub fn unique_final_path(destination_dir: &Path, stem: &str, extension: &str) -> PathBuf {
    let primary = primary_final_path(destination_dir, stem, extension);
    if !primary.exists() {
        return primary;
    }

    for index in 1..10_000 {
        let candidate = destination_dir.join(format!("{stem} ({index}).{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }

    destination_dir.join(format!("{stem} ({}).{extension}", uuid_like()))
}

fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_else(|_| "final".to_string())
}

/// Primary destination path before rename/skip/replace policy resolution.
pub fn primary_final_path(destination_dir: &Path, stem: &str, extension: &str) -> PathBuf {
    destination_dir.join(format!("{stem}.{extension}"))
}

/// Move verified temp into the final destination path.
pub fn finalize_output_with_policy(
    temp_path: &Path,
    final_path: &Path,
    allow_replace: bool,
) -> Result<(), AppError> {
    if !is_our_temp_file(temp_path) {
        return Err(AppError::VerificationFailure {
            detail: "Refusing to finalize a file that is not our temporary output.".to_string(),
        });
    }

    if final_path.exists() && !allow_replace {
        return Err(AppError::OutputExists {
            detail: format!(
                "Destination already exists unexpectedly: {}",
                final_path.display()
            ),
        });
    }

    if let Some(parent) = final_path.parent() {
        if !parent.is_dir() {
            return Err(AppError::DestinationUnavailable {
                detail: format!("Destination folder unavailable: {}", parent.display()),
            });
        }
    }

    // Safe replace: move existing aside first, then promote temp. Restore on failure.
    let backup = if final_path.exists() {
        let backup = final_path.with_file_name(format!(
            "{}.jwbak-{}",
            final_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("output"),
            uuid::Uuid::new_v4()
        ));
        std::fs::rename(final_path, &backup).map_err(|error| AppError::DestinationUnavailable {
            detail: format!(
                "Could not move existing output aside {}: {error}",
                final_path.display()
            ),
        })?;
        Some(backup)
    } else {
        None
    };

    let promote = || -> Result<(), AppError> {
        match std::fs::rename(temp_path, final_path) {
            Ok(()) => Ok(()),
            Err(rename_error) => {
                // Cross-volume fallback. Clean partial destination if copy fails.
                match std::fs::copy(temp_path, final_path) {
                    Ok(_) => {
                        cleanup_temp(temp_path);
                        Ok(())
                    }
                    Err(copy_error) => {
                        if final_path.exists() {
                            let _ = std::fs::remove_file(final_path);
                        }
                        Err(AppError::DestinationUnavailable {
                            detail: format!(
                                "Could not write output file (rename: {rename_error}; copy: {copy_error})"
                            ),
                        })
                    }
                }
            }
        }
    };

    match promote() {
        Ok(()) => {
            if let Some(backup) = backup {
                let _ = std::fs::remove_file(backup);
            }
            Ok(())
        }
        Err(error) => {
            if let Some(backup) = backup {
                if final_path.exists() {
                    let _ = std::fs::remove_file(final_path);
                }
                if let Err(restore_error) = std::fs::rename(&backup, final_path) {
                    return Err(AppError::DestinationUnavailable {
                        detail: format!(
                            "{error} Original kept at {}. Restore failed: {restore_error}",
                            backup.display()
                        ),
                    });
                }
            }
            Err(error)
        }
    }
}

pub fn finalize_output(temp_path: &Path, final_path: &Path) -> Result<(), AppError> {
    finalize_output_with_policy(temp_path, final_path, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "jwconverter-finalize-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    #[test]
    fn primary_final_path_joins_stem_and_extension() {
        let dir = PathBuf::from("/music/out");
        assert_eq!(
            primary_final_path(&dir, "song", "flac"),
            dir.join("song.flac")
        );
    }

    #[test]
    fn unique_final_path_renames_when_exists() {
        let dir = test_dir();
        let existing = dir.join("song.flac");
        std::fs::write(&existing, b"existing").expect("write existing");

        let unique = unique_final_path(&dir, "song", "flac");
        assert_eq!(unique, dir.join("song (1).flac"));

        let _ = std::fs::remove_file(&existing);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn finalize_output_replace_overwrites_existing() {
        let dir = test_dir();
        let final_path = dir.join("song.flac");
        std::fs::write(&final_path, b"old content").expect("write old final");

        let temp_path = dir.join(format!("song.jwconverting-{}.flac", uuid::Uuid::new_v4()));
        std::fs::write(&temp_path, b"new content").expect("write temp");

        finalize_output_with_policy(&temp_path, &final_path, true).expect("finalize replace");

        let content = std::fs::read(&final_path).expect("read final");
        assert_eq!(content, b"new content");
        assert!(!temp_path.exists());

        let _ = std::fs::remove_file(&final_path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn finalize_refuses_unmarked_temp() {
        let dir = test_dir();
        let final_path = dir.join("song.flac");
        let impostor = dir.join("not-our-temp.flac");
        std::fs::write(&impostor, b"data").expect("write impostor");

        let result = finalize_output_with_policy(&impostor, &final_path, false);
        assert!(result.is_err());
        // Impostor must be left alone — cleanup only handles our temp files.
        assert!(impostor.is_file());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn finalize_refuses_existing_destination_without_replace() {
        let dir = test_dir();
        let final_path = dir.join("song.flac");
        std::fs::write(&final_path, b"old").expect("write final");

        let temp_path = dir.join(format!("song.jwconverting-{}.flac", uuid::Uuid::new_v4()));
        std::fs::write(&temp_path, b"new").expect("write temp");

        let result = finalize_output_with_policy(&temp_path, &final_path, false);
        assert!(result.is_err());
        // Destination untouched; temp still present for caller cleanup.
        assert_eq!(std::fs::read(&final_path).expect("read final"), b"old");
        assert!(temp_path.is_file());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unique_final_path_increments_index() {
        let dir = test_dir();
        std::fs::write(dir.join("song.flac"), b"a").expect("write 0");
        std::fs::write(dir.join("song (1).flac"), b"b").expect("write 1");

        let unique = unique_final_path(&dir, "song", "flac");
        assert_eq!(unique, dir.join("song (2).flac"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unique_final_path_fresh_name_when_free() {
        let dir = test_dir();
        let unique = unique_final_path(&dir, "fresh", "mp3");
        assert_eq!(unique, dir.join("fresh.mp3"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn finalize_restore_after_failed_promote_removes_partial() {
        let dir = test_dir();
        let final_path = dir.join("song.flac");
        std::fs::write(&final_path, b"old content").expect("write old final");

        // Marker name passes is_our_temp_file, but the path does not exist → promote fails.
        let missing_temp =
            dir.join(format!("song.jwconverting-{}.flac", uuid::Uuid::new_v4()));

        let err = finalize_output_with_policy(&missing_temp, &final_path, true)
            .expect_err("promote should fail");
        assert!(!err.to_string().is_empty());
        let content = std::fs::read(&final_path).expect("read restored");
        assert_eq!(content, b"old content");

        let _ = std::fs::remove_file(&final_path);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
