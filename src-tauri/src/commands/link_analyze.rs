//! Experimental Links — local metadata inspection (debug builds).

use std::path::Path;

use crate::logging;
use crate::media::link_errors::classify_app_error_message;
use crate::media::ytdlp::{self, LinkMediaInfo};

/// Inspect a public media URL with the local yt-dlp sidecar. Does not download.
#[tauri::command]
pub fn analyze_link(
    url: String,
    cookies_path: Option<String>,
) -> Result<LinkMediaInfo, String> {
    let cookies = cookies_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty());
    if let Some(path) = cookies {
        if !Path::new(path).is_file() {
            return Err("The selected cookies.txt file could not be found.".to_string());
        }
    }

    match ytdlp::inspect_with_options(url.trim(), cookies.map(Path::new)) {
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
