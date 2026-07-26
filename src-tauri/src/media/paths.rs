//! Resolves ffmpeg.exe / ffprobe.exe for development and bundled production.
//! Production must not depend on PATH.

use std::path::{Path, PathBuf};

#[cfg(windows)]
const FFPROBE_NAME: &str = "ffprobe.exe";
#[cfg(not(windows))]
const FFPROBE_NAME: &str = "ffprobe";

#[cfg(windows)]
const FFMPEG_NAME: &str = "ffmpeg.exe";
#[cfg(not(windows))]
const FFMPEG_NAME: &str = "ffmpeg";

#[derive(Debug, Clone)]
pub struct MediaToolPaths {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

#[derive(Debug, Clone)]
pub struct MediaToolStatus {
    pub ffmpeg: Option<PathBuf>,
    pub ffprobe: Option<PathBuf>,
    pub source: String,
}

/// Resolve both tools. FFprobe is required for analysis; FFmpeg may be absent until conversion.
pub fn resolve_ffprobe() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("CONVERTER_FFPROBE") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!(
            "CONVERTER_FFPROBE is set but not a file: {}",
            path.display()
        ));
    }

    if let Some(dir) = candidate_binary_dirs()
        .into_iter()
        .find(|dir| dir.join(FFPROBE_NAME).is_file())
    {
        return Ok(dir.join(FFPROBE_NAME));
    }

    // Development convenience only — never rely on this for production packaging.
    #[cfg(debug_assertions)]
    if let Some(path) = which_on_path(FFPROBE_NAME) {
        return Ok(path);
    }

    Err(format!(
        "FFprobe was not found. Place {} in src-tauri/binaries/ or set CONVERTER_FFPROBE.",
        FFPROBE_NAME
    ))
}

pub fn resolve_ffmpeg() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("CONVERTER_FFMPEG") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
        return None;
    }

    for dir in candidate_binary_dirs() {
        let candidate = dir.join(FFMPEG_NAME);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    #[cfg(debug_assertions)]
    if let Some(path) = which_on_path(FFMPEG_NAME) {
        return Some(path);
    }

    None
}

pub fn media_tool_status() -> MediaToolStatus {
    let ffprobe = resolve_ffprobe().ok();
    let ffmpeg = resolve_ffmpeg();
    let source = if ffprobe
        .as_ref()
        .is_some_and(|p| p.starts_with(env!("CARGO_MANIFEST_DIR")))
    {
        "project-binaries".to_string()
    } else if ffprobe.is_some() {
        "resolved".to_string()
    } else {
        "missing".to_string()
    };

    MediaToolStatus {
        ffmpeg,
        ffprobe,
        source,
    }
}

#[cfg(windows)]
const MAGICK_NAME: &str = "magick.exe";
#[cfg(not(windows))]
const MAGICK_NAME: &str = "magick";

pub fn resolve_magick() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("CONVERTER_MAGICK") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
        return None;
    }

    for dir in candidate_binary_dirs() {
        let nested = dir.join("imagemagick").join(MAGICK_NAME);
        if nested.is_file() {
            return Some(nested);
        }
        let candidate = dir.join(MAGICK_NAME);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    // Windows ships a portable Magick tree. On macOS/Linux, fall back to a
    // system install (Homebrew / apt) when no portable tree is bundled.
    #[cfg(any(not(windows), debug_assertions))]
    if let Some(path) = which_on_path(MAGICK_NAME) {
        return Some(path);
    }

    None
}

fn candidate_binary_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // Dev / source tree: src-tauri/binaries
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries"));

    // Next to the running executable (NSIS / externalBin layout)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.to_path_buf());
            dirs.push(parent.join("binaries"));
            dirs.push(parent.join("resources"));
            dirs.push(parent.join("resources").join("binaries"));
            dirs.push(parent.join("resources").join("imagemagick"));
        }
    }

    dirs
}

#[cfg(debug_assertions)]
fn which_on_path(name: &str) -> Option<PathBuf> {
    which_on_path_impl(name)
}

#[cfg(all(not(windows), not(debug_assertions)))]
fn which_on_path(name: &str) -> Option<PathBuf> {
    which_on_path_impl(name)
}

fn which_on_path_impl(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[allow(dead_code)]
pub fn resolve_media_tools() -> Option<MediaToolPaths> {
    let ffprobe = resolve_ffprobe().ok()?;
    let ffmpeg = resolve_ffmpeg()?;
    Some(MediaToolPaths { ffmpeg, ffprobe })
}

#[allow(dead_code)]
pub fn tool_exists(path: &Path) -> bool {
    path.is_file()
}
