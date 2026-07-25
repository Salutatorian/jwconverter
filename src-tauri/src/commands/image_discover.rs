//! Discover image files from paths and folders (optional recursion).

use std::path::{Path, PathBuf};

use serde::Serialize;

const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "tif", "tiff", "bmp", "gif", "heic", "heif", "avif",
    // Common camera RAW (Magick/LibRaw when available in the portable build)
    "cr2", "cr3", "nef", "arw", "dng", "orf", "rw2", "raf", "pef", "srw",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredImage {
    pub path: String,
    pub filename: String,
    pub relative_subdir: Option<String>,
}

#[tauri::command]
pub fn discover_image_paths(
    paths: Vec<String>,
    recursive: bool,
) -> Result<Vec<DiscoveredImage>, String> {
    let mut discovered = Vec::new();

    for raw in paths {
        let path = PathBuf::from(raw.trim());
        if !path.exists() {
            continue;
        }
        if path.is_file() {
            if is_image_file(&path) {
                discovered.push(file_entry(&path, None));
            }
            continue;
        }
        if path.is_dir() {
            scan_directory(&path, &path, recursive, &mut discovered)?;
        }
    }

    discovered.sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));
    discovered.dedup_by(|a, b| a.path.eq_ignore_ascii_case(&b.path));
    Ok(discovered)
}

fn scan_directory(
    root: &Path,
    current: &Path,
    recursive: bool,
    out: &mut Vec<DiscoveredImage>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(current)
        .map_err(|error| format!("Could not read folder {}: {error}", current.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if recursive {
                scan_directory(root, &path, true, out)?;
            }
            continue;
        }
        if path.is_file() && is_image_file(&path) {
            out.push(file_entry(&path, relative_subdir(root, &path)));
        }
    }
    Ok(())
}

fn file_entry(path: &Path, relative_subdir: Option<String>) -> DiscoveredImage {
    DiscoveredImage {
        path: path.to_string_lossy().into_owned(),
        filename: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string(),
        relative_subdir,
    }
}

fn relative_subdir(root: &Path, file: &Path) -> Option<String> {
    let parent = file.parent()?;
    let rel = parent.strip_prefix(root).ok()?;
    if rel.as_os_str().is_empty() {
        return None;
    }
    let text = rel
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_ascii_lowercase();
            IMAGE_EXTENSIONS.iter().any(|allowed| *allowed == lower)
        })
        .unwrap_or(false)
}
