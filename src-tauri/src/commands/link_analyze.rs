//! Experimental Links — local metadata inspection (debug builds).

use crate::logging;
use crate::media::link_errors::classify_app_error_message;
use crate::media::ytdlp::{self, LinkMediaInfo};

/// Inspect a public media URL with the local yt-dlp sidecar. Does not download.
#[tauri::command]
pub fn analyze_link(url: String) -> Result<LinkMediaInfo, String> {
    match ytdlp::inspect(url.trim()) {
        Ok(info) => {
            logging::log_link_event(
                "link_analyze_ok",
                &format!(
                    "service={} live={} playlist={}",
                    info.service.as_deref().unwrap_or("unknown"),
                    info.is_live,
                    info.is_playlist
                ),
            );
            Ok(info)
        }
        Err(error) => {
            let message = error.to_string();
            let category = classify_app_error_message(&message);
            logging::log_link_event(
                "link_analyze_failed",
                &format!("category={}", category.as_str()),
            );
            Err(message)
        }
    }
}
