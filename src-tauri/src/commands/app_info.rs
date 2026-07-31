use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub phase: String,
    /// Dev builds only — experimental Links nav (Phase 1 metadata).
    pub links_experimental: bool,
}

/// Returns basic application identity. Used to verify frontend ↔ Rust IPC.
#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        name: "JW Converter".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        phase: "images".to_string(),
        links_experimental: cfg!(debug_assertions),
    }
}
