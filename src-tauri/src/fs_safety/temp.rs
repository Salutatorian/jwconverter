//! Unique temporary output paths on the destination volume.

use std::path::{Path, PathBuf};

use crate::errors::AppError;

const TEMP_MARKER: &str = ".jwconverting-";

/// Build a unique temp path in `destination_dir`, same volume as the final file.
pub fn temp_output_path(
    destination_dir: &Path,
    stem: &str,
    extension: &str,
    job_id: &str,
) -> Result<PathBuf, AppError> {
    if !destination_dir.is_dir() {
        return Err(AppError::DestinationUnavailable {
            detail: format!(
                "Destination folder does not exist: {}",
                destination_dir.display()
            ),
        });
    }

    let short_id = job_id.chars().take(8).collect::<String>();
    let file_name = format!("{stem}{TEMP_MARKER}{short_id}.{extension}");
    let path = destination_dir.join(file_name);

    if path.exists() {
        return Err(AppError::DestinationUnavailable {
            detail: "Could not create a unique temporary output path.".to_string(),
        });
    }

    Ok(path)
}

pub fn is_our_temp_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.contains(TEMP_MARKER))
}

/// Delete a temp file only if it matches our naming marker.
pub fn cleanup_temp(path: &Path) {
    if !is_our_temp_file(path) {
        return;
    }
    let _ = std::fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_detection() {
        assert!(is_our_temp_file(Path::new("song.jwconverting-ab12cd34.flac")));
        assert!(is_our_temp_file(Path::new(r"C:\out\x.jwconverting-q.mp3")));
        assert!(!is_our_temp_file(Path::new("song.flac")));
        assert!(!is_our_temp_file(Path::new("song.jwconverting.flac")));
        // The bare marker string does contain the marker — document that.
        assert!(is_our_temp_file(Path::new(".jwconverting-")));
        assert!(!is_our_temp_file(Path::new("")));
    }

    #[test]
    fn temp_path_rejects_missing_destination() {
        let missing = Path::new("__jw_no_such_dir_9f3b__");
        let result = temp_output_path(missing, "stem", "flac", "job-12345678");
        assert!(result.is_err());
    }

    #[test]
    fn temp_path_builds_marked_unique_name() {
        let dir = std::env::temp_dir().join(format!("jw-temp-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("tmpdir");

        let path = temp_output_path(&dir, "song", "flac", "job-abcdef1234567890")
            .expect("temp path");
        assert!(is_our_temp_file(&path));
        let name = path.file_name().and_then(|n| n.to_str()).expect("name");
        assert!(name.starts_with("song.jwconverting-"));
        assert!(name.ends_with(".flac"));
        // Job id is truncated to 8 chars for the marker.
        assert!(name.contains("job-abcd"));
        assert!(!name.contains("job-abcde"));

        // Second call with same job id also succeeds (file not created yet).
        let again = temp_output_path(&dir, "song", "flac", "job-abcdef1234567890")
            .expect("temp path again");
        assert_eq!(path, again);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn temp_path_errors_on_collision() {
        let dir = std::env::temp_dir().join(format!("jw-temp-collide-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("tmpdir");

        let path = temp_output_path(&dir, "song", "flac", "job-collision").expect("temp path");
        std::fs::write(&path, b"taken").expect("occupy temp name");

        let result = temp_output_path(&dir, "song", "flac", "job-collision");
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cleanup_only_deletes_marked_files() {
        let dir = std::env::temp_dir().join(format!("jw-cleanup-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("tmpdir");

        let marked = dir.join("song.jwconverting-abcd1234.flac");
        let unmarked = dir.join("song.flac");
        std::fs::write(&marked, b"temp").expect("write marked");
        std::fs::write(&unmarked, b"final").expect("write unmarked");

        cleanup_temp(&unmarked);
        assert!(unmarked.is_file(), "unmarked file must survive cleanup");

        cleanup_temp(&marked);
        assert!(!marked.exists(), "marked temp must be deleted");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
