//! Unique temporary output paths on the destination volume.

use std::path::{Path, PathBuf};

use crate::errors::AppError;

const TEMP_MARKER: &str = ".jwconverting-";
const LINK_TEMP_MARKER: &str = ".jwdownload-";

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
        .is_some_and(|name| name.contains(TEMP_MARKER) || name.contains(LINK_TEMP_MARKER))
}

/// Output template stem for yt-dlp downloads on the destination volume.
pub fn link_temp_stem(stem: &str, job_id: &str) -> String {
    let short_id = job_id.chars().take(8).collect::<String>();
    format!("{stem}{LINK_TEMP_MARKER}{short_id}")
}

/// Delete a temp file only if it matches our naming marker.
pub fn cleanup_temp(path: &Path) {
    if !is_our_temp_file(path) {
        return;
    }
    let _ = std::fs::remove_file(path);
}

/// Best-effort removal of leftover `.jwdownload-` temps in a destination folder.
pub fn cleanup_orphaned_link_temps(destination_dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(destination_dir) else {
        return 0;
    };
    let mut removed = 0;
    for path in entries.filter_map(Result::ok).map(|entry| entry.path()) {
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.contains(LINK_TEMP_MARKER))
        {
            let _ = std::fs::remove_file(&path);
            if !path.exists() {
                removed += 1;
            }
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_detection() {
        assert!(is_our_temp_file(Path::new(
            "song.jwconverting-ab12cd34.flac"
        )));
        assert!(is_our_temp_file(Path::new("song.jwdownload-ab12cd34.webm")));
        assert!(is_our_temp_file(Path::new(r"C:\out\x.jwconverting-q.mp3")));
        assert!(!is_our_temp_file(Path::new("song.flac")));
        assert!(!is_our_temp_file(Path::new("song.jwconverting.flac")));
        // The bare marker string does contain the marker — document that.
        assert!(is_our_temp_file(Path::new(".jwconverting-")));
        assert!(is_our_temp_file(Path::new(".jwdownload-")));
        assert!(!is_our_temp_file(Path::new("")));
    }

    #[test]
    fn link_temp_stem_includes_short_job_marker() {
        assert_eq!(
            link_temp_stem("song", "job-abcdef123456"),
            "song.jwdownload-job-abcd"
        );
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

        let path =
            temp_output_path(&dir, "song", "flac", "job-abcdef1234567890").expect("temp path");
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
    fn cleanup_orphaned_link_temps_only_touches_markers() {
        let dir = std::env::temp_dir().join(format!("jw-orphan-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("tmpdir");
        let orphan = dir.join("song.jwdownload-deadbeef.webm");
        let keep = dir.join("song.webm");
        std::fs::write(&orphan, b"tmp").unwrap();
        std::fs::write(&keep, b"keep").unwrap();
        assert_eq!(cleanup_orphaned_link_temps(&dir), 1);
        assert!(!orphan.exists());
        assert!(keep.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
