use std::io::Read;

use serde::Deserialize;

use crate::media::paths::resolve_ytdlp;
use crate::media::ytdlp;

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";

#[derive(Deserialize)]
struct GithubRelease {
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[tauri::command]
pub fn get_ytdlp_version() -> Result<String, String> {
    ytdlp::ytdlp_version().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_ytdlp() -> Result<String, String> {
    let release_body = get_bytes(LATEST_RELEASE_URL)
        .map_err(|error| format!("Could not check for yt-dlp updates. Check your internet connection: {error}"))?;
    let release: GithubRelease = serde_json::from_slice(&release_body)
        .map_err(|error| format!("Could not parse the yt-dlp update response: {error}"))?;
    let asset_name = if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    };
    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name.eq_ignore_ascii_case(asset_name))
        .ok_or_else(|| {
            format!("The latest yt-dlp release did not include `{asset_name}`.")
        })?;
    let executable = get_bytes(&asset.browser_download_url)
        .map_err(|error| format!("Could not download the yt-dlp update. Check your internet connection: {error}"))?;
    if executable.is_empty() {
        return Err("The yt-dlp update download was empty.".to_string());
    }

    let target = resolve_ytdlp()?;
    let temporary = {
        let mut name = target
            .file_name()
            .map(|name| name.to_os_string())
            .unwrap_or_else(|| std::ffi::OsString::from("yt-dlp"));
        name.push(".new");
        target.with_file_name(name)
    };
    std::fs::write(&temporary, executable)
        .map_err(|error| format!("Could not save the yt-dlp update: {error}"))?;
    replace_executable(&temporary, &target)?;
    ytdlp::ytdlp_version().map_err(|error| error.to_string())
}

fn get_bytes(url: &str) -> Result<Vec<u8>, String> {
    let mut response = ureq::get(url)
        .call()
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

fn replace_executable(temporary: &std::path::Path, target: &std::path::Path) -> Result<(), String> {
    let backup = {
        let mut name = target
            .file_name()
            .map(|name| name.to_os_string())
            .unwrap_or_else(|| std::ffi::OsString::from("yt-dlp"));
        name.push(".previous");
        target.with_file_name(name)
    };
    let _ = std::fs::remove_file(&backup);
    std::fs::rename(target, &backup)
        .map_err(|error| format!("Could not replace the local yt-dlp executable: {error}"))?;
    match std::fs::rename(temporary, target) {
        Ok(()) => {
            let _ = std::fs::remove_file(backup);
            Ok(())
        }
        Err(error) => {
            let _ = std::fs::rename(&backup, target);
            let _ = std::fs::remove_file(temporary);
            Err(format!("Could not activate the yt-dlp update: {error}"))
        }
    }
}
