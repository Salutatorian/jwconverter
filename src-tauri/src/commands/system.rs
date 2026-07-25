use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultPaths {
    pub downloads_dir: Option<String>,
}

/// Windows-first default output location: the user's Downloads folder.
#[tauri::command]
pub fn get_default_paths() -> DefaultPaths {
    DefaultPaths {
        downloads_dir: downloads_dir().map(|p| p.to_string_lossy().into_owned()),
    }
}

fn downloads_dir() -> Option<PathBuf> {
    // Prefer the standard USERPROFILE\Downloads layout on Windows.
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let candidate = PathBuf::from(profile).join("Downloads");
        if candidate.is_dir() {
            return Some(candidate);
        }
    }

    // Fallback used by some environments.
    if let Ok(home) = std::env::var("HOME") {
        let candidate = PathBuf::from(home).join("Downloads");
        if candidate.is_dir() {
            return Some(candidate);
        }
    }

    None
}
