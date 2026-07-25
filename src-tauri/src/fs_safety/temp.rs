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
