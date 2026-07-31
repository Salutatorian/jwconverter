//! yt-dlp adapter — argv-only process execution for experimental Links.
//! Phase 1: metadata inspection only (`--dump-single-json --skip-download`).

use serde::Deserialize;
use serde::Serialize;
use std::process::Command;

use crate::errors::AppError;
use crate::media::link_url::{redact_url_for_log, validate_media_url};
use crate::media::paths::resolve_ytdlp;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkMediaInfo {
    pub original_url: String,
    pub webpage_url: Option<String>,
    pub extractor: Option<String>,
    pub service: Option<String>,
    pub id: Option<String>,
    pub title: Option<String>,
    pub creator: Option<String>,
    pub duration_seconds: Option<f64>,
    pub is_live: bool,
    pub is_playlist: bool,
    pub item_count: Option<u32>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct YtdlpDump {
    id: Option<String>,
    title: Option<String>,
    extractor: Option<String>,
    extractor_key: Option<String>,
    webpage_url: Option<String>,
    original_url: Option<String>,
    uploader: Option<String>,
    channel: Option<String>,
    creator: Option<String>,
    duration: Option<f64>,
    is_live: Option<bool>,
    was_live: Option<bool>,
    #[serde(default)]
    _type: Option<String>,
    playlist_count: Option<u32>,
    n_entries: Option<u32>,
    entries: Option<serde_json::Value>,
}

/// Inspect a remote media URL with yt-dlp. Does not download media.
pub fn inspect(url: &str) -> Result<LinkMediaInfo, AppError> {
    let safe = validate_media_url(url)?;
    let ytdlp = resolve_ytdlp().map_err(|detail| AppError::MediaToolMissing { detail })?;
    let raw = run_dump_json(&ytdlp, safe.as_str())?;
    normalize(safe.as_str(), &raw)
}

fn run_dump_json(ytdlp: &std::path::Path, url: &str) -> Result<YtdlpDump, AppError> {
    let mut command = Command::new(ytdlp);
    command.args([
        "--dump-single-json",
        "--no-playlist",
        "--skip-download",
        "--no-warnings",
        "--no-call-home",
        url,
    ]);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = command.output().map_err(|error| AppError::DecodeFailure {
        detail: format!("Could not start yt-dlp: {error}"),
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let mapped = map_ytdlp_stderr(&stderr);
        return Err(AppError::DecodeFailure { detail: mapped });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // yt-dlp may print progress lines; take the last JSON object line.
    let json_line = stdout
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with('{'))
        .ok_or_else(|| AppError::DecodeFailure {
            detail: "yt-dlp returned no JSON metadata.".to_string(),
        })?;

    serde_json::from_str(json_line).map_err(|error| AppError::DecodeFailure {
        detail: format!("Could not parse yt-dlp metadata: {error}"),
    })
}

fn normalize(original_url: &str, dump: &YtdlpDump) -> Result<LinkMediaInfo, AppError> {
    let type_name = dump._type.as_deref().unwrap_or("video");
    let is_playlist = type_name == "playlist" || dump.entries.is_some();
    let item_count = dump
        .playlist_count
        .or(dump.n_entries)
        .or_else(|| {
            dump.entries
                .as_ref()
                .and_then(|v| v.as_array().map(|a| a.len() as u32))
        });

    let mut warnings = Vec::new();
    if is_playlist {
        warnings.push(
            "This link looks like a playlist. Playlist downloads are not available in the experimental version."
                .to_string(),
        );
    }
    if dump.is_live == Some(true) {
        warnings.push(
            "This media is currently live. Live recording is not available in this experimental version."
                .to_string(),
        );
    }

    let service = dump
        .extractor_key
        .clone()
        .or_else(|| dump.extractor.clone())
        .map(|s| humanize_service(&s));

    let creator = dump
        .creator
        .clone()
        .or_else(|| dump.uploader.clone())
        .or_else(|| dump.channel.clone());

    Ok(LinkMediaInfo {
        original_url: original_url.to_string(),
        webpage_url: dump.webpage_url.clone().or_else(|| dump.original_url.clone()),
        extractor: dump.extractor.clone().or_else(|| dump.extractor_key.clone()),
        service,
        id: dump.id.clone(),
        title: dump.title.clone(),
        creator,
        duration_seconds: dump.duration,
        is_live: dump.is_live.unwrap_or(false),
        is_playlist,
        item_count,
        warnings,
    })
}

fn humanize_service(extractor: &str) -> String {
    match extractor.to_ascii_lowercase().as_str() {
        "youtube" | "youtubenew" => "YouTube".to_string(),
        "youtubetab" => "YouTube".to_string(),
        "soundcloud" => "SoundCloud".to_string(),
        "vimeo" => "Vimeo".to_string(),
        "twitter" | "x" => "X".to_string(),
        "tiktok" => "TikTok".to_string(),
        "reddit" => "Reddit".to_string(),
        "instagram" => "Instagram".to_string(),
        "facebook" => "Facebook".to_string(),
        other => other.to_string(),
    }
}

fn map_ytdlp_stderr(stderr: &str) -> String {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("unsupported url") || lower.contains("no suitable extractor") {
        return "JWConverter does not currently recognize this link.".to_string();
    }
    if lower.contains("private video")
        || lower.contains("login required")
        || lower.contains("sign in")
        || lower.contains("403")
    {
        return "This media is not publicly accessible. JWConverter does not bypass private access restrictions.".to_string();
    }
    if lower.contains("video unavailable")
        || lower.contains("has been removed")
        || lower.contains("not available")
    {
        return "The media may have been removed, made private, or restricted.".to_string();
    }
    if lower.contains("timed out") || lower.contains("network") || lower.contains("connection") {
        return "The connection was interrupted before metadata could be loaded.".to_string();
    }

    let tail = stderr
        .lines()
        .rev()
        .take(3)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(" ");
    if tail.trim().is_empty() {
        "Could not inspect this link.".to_string()
    } else {
        format!("Could not inspect this link. ({})", redact_url_for_log(&tail))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_marks_playlist() {
        let dump = YtdlpDump {
            id: Some("pl1".into()),
            title: Some("Demo".into()),
            extractor: Some("youtube".into()),
            extractor_key: Some("Youtube".into()),
            webpage_url: Some("https://example.com".into()),
            original_url: None,
            uploader: Some("Creator".into()),
            channel: None,
            creator: None,
            duration: None,
            is_live: Some(false),
            was_live: None,
            _type: Some("playlist".into()),
            playlist_count: Some(12),
            n_entries: None,
            entries: None,
        };
        let info = normalize("https://example.com", &dump).unwrap();
        assert!(info.is_playlist);
        assert_eq!(info.item_count, Some(12));
        assert!(!info.warnings.is_empty());
        assert_eq!(info.service.as_deref(), Some("YouTube"));
    }

    #[test]
    fn map_unsupported_message() {
        let msg = map_ytdlp_stderr("ERROR: Unsupported URL: https://example.com");
        assert!(msg.contains("does not currently recognize"));
    }

    /// Manual/network proof — not run in normal CI.
    #[test]
    #[ignore = "live network + yt-dlp sidecar"]
    fn live_inspect_public_video() {
        let info = inspect("https://www.youtube.com/watch?v=jNQXAC9IVRw")
            .expect("inspect public video");
        assert_eq!(info.id.as_deref(), Some("jNQXAC9IVRw"));
        assert!(info.title.as_ref().is_some_and(|t| !t.is_empty()));
        assert!(!info.is_playlist);
    }
}
