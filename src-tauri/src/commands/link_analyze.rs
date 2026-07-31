//! Experimental Links — metadata inspection only (Phase 1).

use crate::media::ytdlp::{self, LinkMediaInfo};

/// Inspect a public media URL with the local yt-dlp sidecar. Does not download.
#[tauri::command]
pub fn analyze_link(url: String) -> Result<LinkMediaInfo, String> {
    ytdlp::inspect(url.trim()).map_err(|error| error.to_string())
}
