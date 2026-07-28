//! Discover audio files from paths and folders (optional recursion).

use std::path::{Path, PathBuf};

use serde::Serialize;

const AUDIO_EXTENSIONS: &[&str] = &[
    "wav", "flac", "mp3", "m4a", "m4b", "aac", "ogg", "opus", "aiff", "aif", "wma", "caf",
    "mp4", "m4v", "mov", "webm", "weba", "mka", "mkv", "wv", "ape", "tak", "ac3", "eac3",
    "dts", "mp2", "mp1", "amr", "3gp", "3g2", "ra", "ram", "mpc", "tta", "dsf", "dff",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredAudio {
    pub path: String,
    pub filename: String,
    /// Relative folder under the chosen root, using `/` separators.
    /// Empty/None means place directly in the destination folder.
    pub relative_subdir: Option<String>,
}

#[tauri::command]
pub fn discover_audio_paths(
    paths: Vec<String>,
    recursive: bool,
) -> Result<Vec<DiscoveredAudio>, String> {
    let mut discovered = Vec::new();

    for raw in paths {
        let path = PathBuf::from(raw.trim());
        if !path.exists() {
            continue;
        }

        if path.is_file() {
            if is_audio_file(&path) {
                discovered.push(file_entry(&path, None));
            }
            continue;
        }

        if path.is_dir() {
            scan_directory(&path, &path, recursive, &mut discovered)?;
        }
    }

    // Stable, deterministic order for batching.
    discovered.sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));
    discovered.dedup_by(|a, b| a.path.eq_ignore_ascii_case(&b.path));
    Ok(discovered)
}

fn scan_directory(
    root: &Path,
    current: &Path,
    recursive: bool,
    out: &mut Vec<DiscoveredAudio>,
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
        if path.is_file() && is_audio_file(&path) {
            let relative = relative_subdir(root, &path);
            out.push(file_entry(&path, relative));
        }
    }

    Ok(())
}

fn file_entry(path: &Path, relative_subdir: Option<String>) -> DiscoveredAudio {
    DiscoveredAudio {
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

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_ascii_lowercase();
            AUDIO_EXTENSIONS.iter().any(|allowed| *allowed == lower)
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_tree() -> PathBuf {
        let root = std::env::temp_dir().join(format!("jw-discover-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("Album A").join("Disc 1")).expect("tree");
        std::fs::create_dir_all(root.join("Album B")).expect("tree");
        std::fs::write(root.join("top.mp3"), b"x").expect("file");
        std::fs::write(root.join("notes.txt"), b"x").expect("file");
        std::fs::write(root.join("Album A").join("song.FLAC"), b"x").expect("file");
        std::fs::write(
            root.join("Album A").join("Disc 1").join("deep.wav"),
            b"x",
        )
        .expect("file");
        std::fs::write(root.join("Album B").join("cover.png"), b"x").expect("file");
        root
    }

    #[test]
    fn extension_matching_is_case_insensitive() {
        assert!(is_audio_file(Path::new("song.mp3")));
        assert!(is_audio_file(Path::new("song.FLAC")));
        assert!(is_audio_file(Path::new("song.M4A")));
        assert!(is_audio_file(Path::new("track.dsf")));
        assert!(!is_audio_file(Path::new("cover.png")));
        assert!(!is_audio_file(Path::new("notes.txt")));
        assert!(!is_audio_file(Path::new("no_extension")));
        assert!(!is_audio_file(Path::new("")));
    }

    #[test]
    fn every_listed_extension_is_accepted() {
        for ext in AUDIO_EXTENSIONS {
            let path = format!("file.{ext}");
            assert!(is_audio_file(Path::new(&path)), "{ext}");
        }
    }

    #[test]
    fn relative_subdir_uses_forward_slashes() {
        let root = PathBuf::from("root");
        let file = root.join("Album A").join("Disc 1").join("song.wav");
        assert_eq!(
            relative_subdir(&root, &file).as_deref(),
            Some("Album A/Disc 1")
        );

        let top = root.join("song.wav");
        assert_eq!(relative_subdir(&root, &top), None);
    }

    #[test]
    fn discover_single_file_directly() {
        let root = temp_tree();
        let file = root.join("top.mp3");
        let found = discover_audio_paths(vec![file.to_string_lossy().into_owned()], false)
            .expect("discover");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].filename, "top.mp3");
        assert_eq!(found[0].relative_subdir, None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn discover_ignores_missing_and_non_audio_paths() {
        let root = temp_tree();
        let found = discover_audio_paths(
            vec![
                root.join("does-not-exist.mp3").to_string_lossy().into_owned(),
                root.join("notes.txt").to_string_lossy().into_owned(),
                "   ".to_string(),
            ],
            false,
        )
        .expect("discover");
        assert!(found.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn non_recursive_scan_stays_at_top_level() {
        let root = temp_tree();
        let found = discover_audio_paths(vec![root.to_string_lossy().into_owned()], false)
            .expect("discover");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].filename, "top.mp3");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn recursive_scan_collects_nested_audio_with_subdirs() {
        let root = temp_tree();
        let found = discover_audio_paths(vec![root.to_string_lossy().into_owned()], true)
            .expect("discover");
        assert_eq!(found.len(), 3);

        let by_name: std::collections::HashMap<&str, &DiscoveredAudio> =
            found.iter().map(|d| (d.filename.as_str(), d)).collect();
        assert_eq!(by_name["top.mp3"].relative_subdir, None);
        assert_eq!(
            by_name["song.FLAC"].relative_subdir.as_deref(),
            Some("Album A")
        );
        assert_eq!(
            by_name["deep.wav"].relative_subdir.as_deref(),
            Some("Album A/Disc 1")
        );

        // Sorted case-insensitively for deterministic batching.
        let mut sorted = found.clone();
        sorted.sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));
        assert_eq!(
            found.iter().map(|d| &d.path).collect::<Vec<_>>(),
            sorted.iter().map(|d| &d.path).collect::<Vec<_>>()
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn duplicate_inputs_dedup_case_insensitively() {
        let root = temp_tree();
        let file = root.join("top.mp3");
        let upper = file.to_string_lossy().to_uppercase();
        let found = discover_audio_paths(
            vec![file.to_string_lossy().into_owned(), upper],
            false,
        )
        .expect("discover");
        assert_eq!(found.len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }
}
