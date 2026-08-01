use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::media::paths::{resolve_ytdlp, ytdlp_update_allowed};
use crate::media::ytdlp;

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";
const USER_AGENT: &str = "JWConverter/1.0 (+https://github.com/Salutatorian/jwconverter)";

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
    let target = resolve_ytdlp()?;
    if !ytdlp_update_allowed(&target) {
        return Err(
            "In-app yt-dlp update is only allowed for the bundled app copy. Unset CONVERTER_YTDLP if you set it."
                .to_string(),
        );
    }

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
        .iter()
        .find(|asset| asset.name.eq_ignore_ascii_case(asset_name))
        .ok_or_else(|| format!("The latest yt-dlp release did not include `{asset_name}`."))?;

    ensure_github_download_host(&asset.browser_download_url)?;

    let checksums_url = release
        .assets
        .iter()
        .find(|asset| {
            let name = asset.name.to_ascii_lowercase();
            name == "sha2-256sums" || name == "sha256sums"
        })
        .map(|asset| asset.browser_download_url.as_str())
        .ok_or_else(|| {
            "The latest yt-dlp release did not include SHA2-256SUMS for verification.".to_string()
        })?;
    ensure_github_download_host(checksums_url)?;
    let checksums = get_bytes(checksums_url)
        .map_err(|error| format!("Could not download yt-dlp checksums: {error}"))?;
    let expected = expected_sha256(&checksums, asset_name)?;

    let executable = get_bytes(&asset.browser_download_url)
        .map_err(|error| format!("Could not download the yt-dlp update. Check your internet connection: {error}"))?;
    if executable.is_empty() {
        return Err("The yt-dlp update download was empty.".to_string());
    }
    let actual = hex_sha256(&executable);
    if !actual.eq_ignore_ascii_case(&expected) {
        return Err(format!(
            "yt-dlp update failed checksum verification (expected {expected}, got {actual})."
        ));
    }

    let temporary = sibling_with_suffix(&target, ".new");
    std::fs::write(&temporary, &executable)
        .map_err(|error| format!("Could not save the yt-dlp update: {error}"))?;
    replace_executable(&temporary, &target)?;
    ensure_executable(&target)?;
    ytdlp::ytdlp_version().map_err(|error| error.to_string())
}

fn ensure_github_download_host(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|_| "yt-dlp download URL was invalid.".to_string())?;
    if parsed.scheme() != "https" {
        return Err("yt-dlp downloads must use HTTPS.".to_string());
    }
    let host = parsed
        .host_str()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let allowed = host == "github.com"
        || host.ends_with(".github.com")
        || host == "objects.githubusercontent.com"
        || host == "release-assets.githubusercontent.com";
    if !allowed {
        return Err(format!("Refusing yt-dlp download from unexpected host: {host}"));
    }
    Ok(())
}

fn expected_sha256(checksums: &[u8], asset_name: &str) -> Result<String, String> {
    let text = String::from_utf8_lossy(checksums);
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(hash) = parts.next() else { continue };
        let Some(name) = parts.next() else { continue };
        let name = name.trim_start_matches('*');
        if name.eq_ignore_ascii_case(asset_name) {
            if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err("yt-dlp checksum file contained an invalid hash.".to_string());
            }
            return Ok(hash.to_ascii_lowercase());
        }
    }
    Err(format!(
        "yt-dlp checksum file did not include an entry for `{asset_name}`."
    ))
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn get_bytes(url: &str) -> Result<Vec<u8>, String> {
    let mut response = ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/octet-stream, application/json;q=0.9, */*;q=0.8")
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

fn sibling_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| std::ffi::OsString::from("yt-dlp"));
    name.push(suffix);
    path.with_file_name(name)
}

fn replace_executable(temporary: &Path, target: &Path) -> Result<(), String> {
    let backup = sibling_with_suffix(target, ".previous");
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

fn ensure_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(path)
            .map_err(|error| format!("Could not read yt-dlp permissions: {error}"))?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(permissions.mode() | 0o755);
        std::fs::set_permissions(path, permissions)
            .map_err(|error| format!("Could not mark yt-dlp executable: {error}"))?;
    }
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ensure_github_download_host, expected_sha256, hex_sha256};

    #[test]
    fn parses_sha256sums_entry() {
        let body = concat!(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  yt-dlp.exe\n",
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff  yt-dlp\n"
        );
        assert_eq!(
            expected_sha256(body.as_bytes(), "yt-dlp.exe").unwrap(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
    }

    #[test]
    fn rejects_non_github_hosts() {
        assert!(ensure_github_download_host("https://evil.example/yt-dlp.exe").is_err());
        assert!(ensure_github_download_host(
            "https://objects.githubusercontent.com/github-production-release-asset/x"
        )
        .is_ok());
    }

    #[test]
    fn hashes_bytes() {
        assert_eq!(
            hex_sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
