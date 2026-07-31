//! Package multi-item Links downloads into a single zip in the destination folder.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

use crate::fs_safety::finalize;
use crate::media::link_filename::sanitize_link_stem;

/// Build a Windows-safe zip stem for a multi-item Links batch.
pub fn batch_zip_stem(batch_title: Option<&str>, fallback_titles: &[Option<String>]) -> String {
    if let Some(title) = batch_title.map(str::trim).filter(|title| !title.is_empty()) {
        return sanitize_link_stem(title);
    }
    for title in fallback_titles {
        if let Some(title) = title.as_deref().map(str::trim).filter(|title| !title.is_empty()) {
            return sanitize_link_stem(title);
        }
    }
    "links-batch".to_string()
}

/// Zip every regular file in `source_dir` (non-recursive) into `zip_path`.
/// Returns the number of files added.
pub fn zip_directory_files(source_dir: &Path, zip_path: &Path) -> Result<usize, String> {
    let file = File::create(zip_path)
        .map_err(|error| format!("Could not create zip file: {error}"))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut count = 0usize;

    let entries = std::fs::read_dir(source_dir)
        .map_err(|error| format!("Could not read download staging folder: {error}"))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("Could not read staging entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Could not inspect staging entry: {error}"))?;
        if !file_type.is_file() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.is_empty() {
            continue;
        }
        zip.start_file(name.as_ref(), options)
            .map_err(|error| format!("Could not add `{name}` to zip: {error}"))?;
        let mut input = File::open(entry.path())
            .map_err(|error| format!("Could not read `{name}` for zip: {error}"))?;
        io::copy(&mut input, &mut zip)
            .map_err(|error| format!("Could not write `{name}` into zip: {error}"))?;
        count += 1;
    }

    zip.finish()
        .map_err(|error| format!("Could not finish zip archive: {error}"))?;
    if count == 0 {
        let _ = std::fs::remove_file(zip_path);
        return Err("No completed downloads were available to zip.".to_string());
    }
    Ok(count)
}

/// Create a unique `.zip` under `destination_dir`, fill it from `staging_dir`, return the zip path.
pub fn package_staging_dir(
    staging_dir: &Path,
    destination_dir: &Path,
    stem: &str,
) -> Result<PathBuf, String> {
    let zip_path = finalize::unique_final_path(destination_dir, stem, "zip");
    let temporary = destination_dir.join(format!(
        "{}.jwconverting-{}.zip",
        sanitize_link_stem(stem),
        uuid::Uuid::new_v4()
    ));
    match zip_directory_files(staging_dir, &temporary) {
        Ok(_count) => {
            std::fs::rename(&temporary, &zip_path).map_err(|error| {
                let _ = std::fs::remove_file(&temporary);
                format!("Could not move zip into destination: {error}")
            })?;
            Ok(zip_path)
        }
        Err(error) => {
            let _ = std::fs::remove_file(&temporary);
            Err(error)
        }
    }
}

pub fn remove_dir_best_effort(path: &Path) {
    let _ = std::fs::remove_dir_all(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_zip_stem_prefers_explicit_title() {
        assert_eq!(
            batch_zip_stem(Some("My Playlist: Hits"), &[Some("Track".into())]),
            "My Playlist-Hits"
        );
        assert_eq!(
            batch_zip_stem(None, &[None, Some("Song A".into())]),
            "Song A"
        );
        assert_eq!(batch_zip_stem(None, &[]), "links-batch");
    }

    #[test]
    fn zips_files_from_staging_directory() {
        let root = std::env::temp_dir().join(format!(
            "jwconverter-link-zip-{}",
            uuid::Uuid::new_v4()
        ));
        let staging = root.join("stage");
        let dest = root.join("out");
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(staging.join("a.mp3"), b"aaa").unwrap();
        std::fs::write(staging.join("b.mp3"), b"bbb").unwrap();

        let zip_path = package_staging_dir(&staging, &dest, "Demo Mix").unwrap();
        assert!(zip_path.exists());
        assert_eq!(zip_path.extension().and_then(|e| e.to_str()), Some("zip"));

        let file = File::open(&zip_path).unwrap();
        let archive = zip::ZipArchive::new(file).unwrap();
        assert_eq!(archive.len(), 2);

        remove_dir_best_effort(&root);
    }
}
