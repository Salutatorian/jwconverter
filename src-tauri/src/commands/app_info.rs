use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub phase: String,
    /// Links mode (public URL download via bundled yt-dlp).
    pub links_experimental: bool,
}

/// Returns basic application identity. Used to verify frontend ↔ Rust IPC.
#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        name: "JW Converter".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        phase: "1.0".to_string(),
        links_experimental: true,
    }
}
