//! yt-dlp adapter — argv-only process execution for experimental Links.
//! Phase 1: metadata inspection only (`--dump-single-json --skip-download`).

use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::process::Command;

use crate::errors::AppError;
use crate::media::link_errors::map_ytdlp_message;
use crate::media::link_url::validate_media_url;
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
    pub entries: Vec<LinkPlaylistEntry>,
    pub warnings: Vec<String>,
    pub video_options: Vec<VideoOption>,
    /// Best available audio codec hint from yt-dlp formats (e.g. `opus`, `aac`).
    pub best_audio_codec: Option<String>,
    /// Container/extension for that audio stream when known.
    pub best_audio_ext: Option<String>,
    /// True when the best available source audio appears lossy (for honesty warnings).
    pub source_audio_likely_lossy: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkPlaylistEntry {
    pub id: Option<String>,
    pub title: Option<String>,
    pub url: String,
    pub duration_seconds: Option<f64>,
    pub is_live: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoOption {
    pub id: String,
    pub label: String,
    pub height: u32,
    pub width: Option<u32>,
    pub fps: Option<f64>,
    pub container: Option<String>,
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
    entries: Option<Vec<YtdlpEntry>>,
    #[serde(default)]
    formats: Vec<YtdlpFormat>,
}

#[derive(Debug, Deserialize)]
struct YtdlpEntry {
    id: Option<String>,
    title: Option<String>,
    url: Option<String>,
    webpage_url: Option<String>,
    original_url: Option<String>,
    duration: Option<f64>,
    is_live: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct YtdlpFormat {
    format_id: Option<String>,
    height: Option<u32>,
    width: Option<u32>,
    fps: Option<f64>,
    ext: Option<String>,
    vcodec: Option<String>,
    acodec: Option<String>,
    abr: Option<f64>,
    tbr: Option<f64>,
}

/// Inspect a remote media URL with yt-dlp. Does not download media.
pub fn inspect(url: &str) -> Result<LinkMediaInfo, AppError> {
    inspect_with_options(url, None)
}

/// Inspect optional user-supplied Netscape cookies.txt. Browser cookie stores are never read.
pub fn inspect_with_options(
    url: &str,
    cookies_path: Option<&Path>,
) -> Result<LinkMediaInfo, AppError> {
    let safe = validate_media_url(url)?;
    let ytdlp = resolve_ytdlp().map_err(|detail| AppError::MediaToolMissing { detail })?;
    let raw = run_dump_json(&ytdlp, safe.as_str(), cookies_path)?;
    normalize(safe.as_str(), &raw)
}

fn run_dump_json(
    ytdlp: &std::path::Path,
    url: &str,
    cookies_path: Option<&Path>,
) -> Result<YtdlpDump, AppError> {
    let mut command = Command::new(ytdlp);
    command.args([
        "--dump-single-json",
        "--flat-playlist",
        "--skip-download",
        "--no-warnings",
        "--no-call-home",
    ]);
    if let Some(path) = cookies_path.filter(|path| !path.as_os_str().is_empty()) {
        command.arg("--cookies").arg(path);
    } else {
        command.arg("--no-cookies");
    }
    command.arg(url);

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
        let (_category, mapped) = map_ytdlp_message(&stderr, false);
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
    let item_count = dump.playlist_count.or(dump.n_entries).or_else(|| {
        dump.entries
            .as_ref()
            .map(|entries| entries.len() as u32)
    });

    let mut warnings = Vec::new();
    if is_playlist && item_count.unwrap_or(0) == 0 {
        warnings.push("This playlist contains no downloadable entries.".to_string());
    }
    if dump.is_live == Some(true) {
        warnings.push(
            "This media is currently live. Choose a recording duration before downloading."
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

    let (best_audio_codec, best_audio_ext) = best_audio_hint(&dump.formats);
    let source_audio_likely_lossy =
        audio_source_is_lossy(best_audio_codec.as_deref(), best_audio_ext.as_deref());

    let (entries, skipped_entries) = playlist_entries(dump.entries.as_deref());
    if skipped_entries > 0 {
        warnings.push(format!(
            "Skipped {skipped_entries} playlist item{} without a valid http(s) URL.",
            if skipped_entries == 1 { "" } else { "s" }
        ));
    }

    Ok(LinkMediaInfo {
        original_url: original_url.to_string(),
        webpage_url: dump
            .webpage_url
            .clone()
            .or_else(|| dump.original_url.clone()),
        extractor: dump
            .extractor
            .clone()
            .or_else(|| dump.extractor_key.clone()),
        service,
        id: dump.id.clone(),
        title: dump.title.clone(),
        creator,
        duration_seconds: dump.duration,
        is_live: dump.is_live.unwrap_or(false),
        is_playlist,
        item_count,
        entries,
        warnings,
        video_options: video_options(&dump.formats),
        best_audio_codec,
        best_audio_ext,
        source_audio_likely_lossy,
    })
}

fn playlist_entries(entries: Option<&[YtdlpEntry]>) -> (Vec<LinkPlaylistEntry>, usize) {
    let Some(entries) = entries else {
        return (Vec::new(), 0);
    };
    let mut kept = Vec::new();
    let mut skipped = 0usize;
    for entry in entries {
        let Some(raw_url) = entry
            .webpage_url
            .as_deref()
            .or(entry.original_url.as_deref())
            .or(entry.url.as_deref())
        else {
            skipped += 1;
            continue;
        };
        match validate_media_url(raw_url) {
            Ok(url) => kept.push(LinkPlaylistEntry {
                id: entry.id.clone(),
                title: entry.title.clone(),
                url: url.as_str().to_string(),
                duration_seconds: entry.duration,
                is_live: entry.is_live.unwrap_or(false),
            }),
            Err(_) => skipped += 1,
        }
    }
    (kept, skipped)
}

pub fn ytdlp_version() -> Result<String, AppError> {
    let ytdlp = resolve_ytdlp().map_err(|detail| AppError::MediaToolMissing { detail })?;
    let output = Command::new(ytdlp)
        .arg("--version")
        .output()
        .map_err(|error| AppError::DecodeFailure {
            detail: format!("Could not start yt-dlp: {error}"),
        })?;
    if !output.status.success() {
        return Err(AppError::DecodeFailure {
            detail: "yt-dlp could not report its version.".to_string(),
        });
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if version.is_empty() {
        Err(AppError::DecodeFailure {
            detail: "yt-dlp returned an empty version.".to_string(),
        })
    } else {
        Ok(version)
    }
}

/// Mirrors audio preflight lossy detection for link metadata honesty warnings.
pub fn audio_source_is_lossy(codec: Option<&str>, format: Option<&str>) -> bool {
    let codec = codec.unwrap_or("").to_ascii_lowercase();
    let format = format.unwrap_or("").to_ascii_lowercase();

    const LOSSY_CODECS: &[&str] = &[
        "mp3",
        "mp3float",
        "mp2",
        "mp1",
        "aac",
        "aac_latm",
        "opus",
        "vorbis",
        "wmav1",
        "wmav2",
        "wmapro",
        "ac3",
        "eac3",
        "dts",
        "libopus",
        "libmp3lame",
        "libvorbis",
    ];
    if LOSSY_CODECS.iter().any(|c| codec == *c) {
        return true;
    }

    format.split(',').any(|part| {
        let p = part.trim();
        p == "mp3"
            || p == "mp2"
            || p == "aac"
            || p == "m4a" && codec.contains("aac")
            || p == "ogg" && (codec.contains("vorbis") || codec.contains("opus"))
            || p == "opus"
            || p == "wma"
            || p == "webm" && codec.contains("opus")
            || p == "m4a" && (codec.is_empty() || codec.contains("aac"))
    })
}

fn best_audio_hint(formats: &[YtdlpFormat]) -> (Option<String>, Option<String>) {
    let mut candidates: Vec<&YtdlpFormat> = formats
        .iter()
        .filter(|format| {
            format
                .acodec
                .as_deref()
                .is_some_and(|codec| codec != "none")
        })
        .collect();
    if candidates.is_empty() {
        return (None, None);
    }

    candidates.sort_by(|left, right| {
        let left_audio_only = left
            .vcodec
            .as_deref()
            .map(|codec| codec == "none")
            .unwrap_or(true);
        let right_audio_only = right
            .vcodec
            .as_deref()
            .map(|codec| codec == "none")
            .unwrap_or(true);
        right_audio_only
            .cmp(&left_audio_only)
            .then_with(|| {
                let left_br = left.abr.or(left.tbr).unwrap_or(0.0);
                let right_br = right.abr.or(right.tbr).unwrap_or(0.0);
                right_br
                    .partial_cmp(&left_br)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let best = candidates[0];
    (
        best.acodec
            .clone()
            .filter(|codec| codec != "none"),
        best.ext.clone(),
    )
}

fn video_options(formats: &[YtdlpFormat]) -> Vec<VideoOption> {
    let mut options = formats
        .iter()
        .filter(|format| {
            format.height.is_some()
                && format.format_id.is_some()
                && format
                    .vcodec
                    .as_deref()
                    .is_some_and(|codec| codec != "none")
        })
        .filter_map(|format| {
            let height = format.height?;
            let id = format.format_id.clone()?;
            let dimensions = format
                .width
                .map(|width| format!("{width}×{height}"))
                .unwrap_or_else(|| format!("{height}p"));
            let fps = format.fps.filter(|fps| fps.is_finite() && *fps > 0.0);
            let label = match (fps, format.ext.as_deref()) {
                (Some(fps), Some(container)) => {
                    format!("{dimensions} · {fps:.0} fps · {container}")
                }
                (Some(fps), None) => format!("{dimensions} · {fps:.0} fps"),
                (None, Some(container)) => format!("{dimensions} · {container}"),
                (None, None) => dimensions,
            };
            Some(VideoOption {
                id,
                label,
                height,
                width: format.width,
                fps,
                container: format.ext.clone(),
            })
        })
        .collect::<Vec<_>>();
    options.sort_by(|left, right| right.height.cmp(&left.height).then(left.id.cmp(&right.id)));
    options.dedup_by(|left, right| left.height == right.height);
    options
}

fn humanize_service(extractor: &str) -> String {
    match extractor.to_ascii_lowercase().as_str() {
        "youtube" | "youtubenew" => "YouTube".to_string(),
        "youtubetab" => "YouTube".to_string(),
        "soundcloud" => "SoundCloud".to_string(),
        "bandcamp" => "Bandcamp".to_string(),
        "twitch" | "twitchvod" | "twitchclips" => "Twitch".to_string(),
        "bilibili" => "Bilibili".to_string(),
        "dailymotion" => "Dailymotion".to_string(),
        "rumble" => "Rumble".to_string(),
        "vimeo" => "Vimeo".to_string(),
        "twitter" | "x" => "X".to_string(),
        "tiktok" => "TikTok".to_string(),
        "reddit" => "Reddit".to_string(),
        "instagram" => "Instagram".to_string(),
        "facebook" => "Facebook".to_string(),
        other => other.to_string(),
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
            formats: Vec::new(),
        };
        let info = normalize("https://example.com", &dump).unwrap();
        assert!(info.is_playlist);
        assert_eq!(info.item_count, Some(12));
        assert!(!info.warnings.is_empty());
        assert_eq!(info.service.as_deref(), Some("YouTube"));
        assert!(!info.source_audio_likely_lossy);
        assert!(info.entries.is_empty());
    }

    #[test]
    fn best_audio_prefers_audio_only_and_marks_lossy() {
        let formats = vec![
            YtdlpFormat {
                format_id: Some("18".into()),
                height: Some(360),
                width: Some(640),
                fps: None,
                ext: Some("mp4".into()),
                vcodec: Some("avc1".into()),
                acodec: Some("aac".into()),
                abr: Some(96.0),
                tbr: Some(500.0),
            },
            YtdlpFormat {
                format_id: Some("251".into()),
                height: None,
                width: None,
                fps: None,
                ext: Some("webm".into()),
                vcodec: Some("none".into()),
                acodec: Some("opus".into()),
                abr: Some(160.0),
                tbr: None,
            },
        ];
        let (codec, ext) = best_audio_hint(&formats);
        assert_eq!(codec.as_deref(), Some("opus"));
        assert_eq!(ext.as_deref(), Some("webm"));
        assert!(audio_source_is_lossy(codec.as_deref(), ext.as_deref()));
        assert!(!audio_source_is_lossy(Some("flac"), Some("flac")));
    }

    #[test]
    fn map_unsupported_message() {
        let msg = crate::media::link_errors::map_ytdlp_message(
            "ERROR: Unsupported URL: https://example.com",
            false,
        )
        .1;
        assert!(msg.contains("does not currently recognize"));
    }

    /// Manual/network proof — not run in normal CI.
    #[test]
    #[ignore = "live network + yt-dlp sidecar"]
    fn live_inspect_public_video() {
        let info =
            inspect("https://www.youtube.com/watch?v=jNQXAC9IVRw").expect("inspect public video");
        assert_eq!(info.id.as_deref(), Some("jNQXAC9IVRw"));
        assert!(info.title.as_ref().is_some_and(|t| !t.is_empty()));
        assert!(!info.is_playlist);
    }
}
