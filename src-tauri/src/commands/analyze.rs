use serde::Serialize;

use crate::media::ffprobe::{self, AudioInfo};
use crate::media::paths;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaToolsInfo {
    pub ffmpeg_path: Option<String>,
    pub ffprobe_path: Option<String>,
    pub source: String,
}

/// Analyze one local audio file with FFprobe.
#[tauri::command]
pub fn analyze_file(path: String) -> Result<AudioInfo, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("No file path was provided.".to_string());
    }
    ffprobe::analyze(trimmed).map_err(|error| error.to_string())
}

/// Report where media tools were resolved from (debug / About later).
#[tauri::command]
pub fn get_media_tools_info() -> MediaToolsInfo {
    let status = paths::media_tool_status();
    MediaToolsInfo {
        ffmpeg_path: status
            .ffmpeg
            .map(|path| path.to_string_lossy().into_owned()),
        ffprobe_path: status
            .ffprobe
            .map(|path| path.to_string_lossy().into_owned()),
        source: status.source,
    }
}
