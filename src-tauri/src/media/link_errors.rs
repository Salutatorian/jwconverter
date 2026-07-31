//! Map yt-dlp stderr into clear, user-facing messages (no raw dumps in the UI).

use crate::media::link_url::redact_url_for_log;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkErrorCategory {
    UnsupportedUrl,
    PrivateOrRestricted,
    Unavailable,
    ServiceChanged,
    Network,
    NoDownloadableMedia,
    LiveUnsupported,
    PlaylistUnsupported,
    Cancelled,
    DiskFull,
    MissingTool,
    Other,
}

impl LinkErrorCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedUrl => "unsupported_url",
            Self::PrivateOrRestricted => "private_or_restricted",
            Self::Unavailable => "unavailable",
            Self::ServiceChanged => "service_changed",
            Self::Network => "network",
            Self::NoDownloadableMedia => "no_downloadable_media",
            Self::LiveUnsupported => "live_unsupported",
            Self::PlaylistUnsupported => "playlist_unsupported",
            Self::Cancelled => "cancelled",
            Self::DiskFull => "disk_full",
            Self::MissingTool => "missing_tool",
            Self::Other => "other",
        }
    }
}

/// Classify and map resolver/download stderr into a stable UI message.
pub fn map_ytdlp_message(stderr: &str, for_download: bool) -> (LinkErrorCategory, String) {
    let lower = stderr.to_ascii_lowercase();

    if looks_like_disk_full(&lower) {
        return (
            LinkErrorCategory::DiskFull,
            "The destination drive does not have enough free space.".to_string(),
        );
    }

    if lower.contains("unsupported url") || lower.contains("no suitable extractor") {
        return (
            LinkErrorCategory::UnsupportedUrl,
            "JWConverter does not currently recognize this link.".to_string(),
        );
    }

    if lower.contains("private video")
        || lower.contains("login required")
        || lower.contains("sign in")
        || lower.contains("members-only")
        || lower.contains("403")
        || lower.contains("http error 401")
    {
        return (
            LinkErrorCategory::PrivateOrRestricted,
            "This media is not publicly accessible. JWConverter does not bypass private access restrictions.".to_string(),
        );
    }

    if lower.contains("cookies file")
        || lower.contains("cookie file")
        || lower.contains("invalid netscape cookies")
        || lower.contains("could not load cookies")
    {
        return (
            LinkErrorCategory::PrivateOrRestricted,
            "The selected cookies.txt file could not be used. Choose a valid Netscape-format cookies.txt file or continue without cookies.".to_string(),
        );
    }

    if lower.contains("video unavailable")
        || lower.contains("has been removed")
        || lower.contains("not available")
        || lower.contains("404")
    {
        return (
            LinkErrorCategory::Unavailable,
            "The media may have been removed, made private, or restricted.".to_string(),
        );
    }

    if lower.contains("unable to extract")
        || lower.contains("failed to parse json")
        || lower.contains("extractorerror")
        || lower.contains("youtube said")
    {
        return (
            LinkErrorCategory::ServiceChanged,
            "This website may have changed. Updating the downloader engine may restore compatibility.".to_string(),
        );
    }

    if lower.contains("timed out")
        || lower.contains("timeout")
        || lower.contains("network is unreachable")
        || lower.contains("connection reset")
        || lower.contains("connection refused")
        || lower.contains("temporary failure in name resolution")
        || lower.contains("name or service not known")
        || lower.contains("ssl:")
        || lower.contains("eof occurred")
    {
        let message = if for_download {
            "The connection was interrupted before the download completed."
        } else {
            "The connection was interrupted before metadata could be loaded."
        };
        return (LinkErrorCategory::Network, message.to_string());
    }

    if lower.contains("requested format is not available")
        || lower.contains("no video formats")
        || lower.contains("no audio formats")
        || lower.contains("no suitable format")
        || lower.contains("ffmpeg not found") && for_download
    {
        return (
            LinkErrorCategory::NoDownloadableMedia,
            "The page was found, but no supported downloadable media was detected.".to_string(),
        );
    }

    if lower.contains("is a live") || lower.contains("live event will begin") {
        return (
            LinkErrorCategory::LiveUnsupported,
            "Live recording is not available in this experimental version.".to_string(),
        );
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
        let message = if for_download {
            "Could not download this link."
        } else {
            "Could not inspect this link."
        };
        return (LinkErrorCategory::Other, message.to_string());
    }

    let action = if for_download { "download" } else { "inspect" };
    (
        LinkErrorCategory::Other,
        format!(
            "Could not {action} this link. ({})",
            redact_url_for_log(&tail)
        ),
    )
}

pub fn classify_app_error_message(message: &str) -> LinkErrorCategory {
    let lower = message.to_ascii_lowercase();
    if lower.contains("yt-dlp was not found")
        || lower.contains("ffmpeg was not found")
        || lower.contains("ffprobe was not found")
        || lower.contains("could not start yt-dlp")
        || lower.contains("experimental links needs")
    {
        return LinkErrorCategory::MissingTool;
    }
    if lower.contains("enough free space") || (lower.contains("disk") && lower.contains("full")) {
        return LinkErrorCategory::DiskFull;
    }
    if lower.contains("live recording") {
        return LinkErrorCategory::LiveUnsupported;
    }
    if lower.contains("playlist") {
        return LinkErrorCategory::PlaylistUnsupported;
    }
    if lower.contains("cancelled") {
        return LinkErrorCategory::Cancelled;
    }
    if lower.contains("does not currently recognize") {
        return LinkErrorCategory::UnsupportedUrl;
    }
    if lower.contains("does not bypass") || lower.contains("not publicly accessible") {
        return LinkErrorCategory::PrivateOrRestricted;
    }
    if lower.contains("removed, made private") {
        return LinkErrorCategory::Unavailable;
    }
    if lower.contains("website may have changed") {
        return LinkErrorCategory::ServiceChanged;
    }
    if lower.contains("no supported downloadable") {
        return LinkErrorCategory::NoDownloadableMedia;
    }
    if lower.contains("interrupted") || lower.contains("connection") {
        return LinkErrorCategory::Network;
    }
    LinkErrorCategory::Other
}

fn looks_like_disk_full(lower: &str) -> bool {
    lower.contains("no space left")
        || lower.contains("not enough space")
        || lower.contains("disk full")
        || lower.contains("os error 28")
        || lower.contains("os error 112")
        || lower.contains("error 112")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_unsupported_and_private() {
        let (cat, msg) = map_ytdlp_message("ERROR: Unsupported URL: https://example.com", false);
        assert_eq!(cat, LinkErrorCategory::UnsupportedUrl);
        assert!(msg.contains("does not currently recognize"));

        let (cat, msg) = map_ytdlp_message("ERROR: Private video. Sign in", true);
        assert_eq!(cat, LinkErrorCategory::PrivateOrRestricted);
        assert!(msg.contains("does not bypass"));
    }

    #[test]
    fn maps_network_and_service_changed() {
        let (cat, msg) = map_ytdlp_message("ERROR: timed out", true);
        assert_eq!(cat, LinkErrorCategory::Network);
        assert!(msg.contains("download completed"));

        let (cat, _) = map_ytdlp_message("ERROR: Unable to extract initial player response", false);
        assert_eq!(cat, LinkErrorCategory::ServiceChanged);
    }

    #[test]
    fn maps_disk_full() {
        let (cat, msg) = map_ytdlp_message("ERROR: [Errno 28] No space left on device", true);
        assert_eq!(cat, LinkErrorCategory::DiskFull);
        assert!(msg.contains("free space"));
    }

    #[test]
    fn redacts_token_tails() {
        let (cat, msg) = map_ytdlp_message(
            "ERROR: failed https://example.com/x?access_token=secret",
            false,
        );
        assert_eq!(cat, LinkErrorCategory::Other);
        assert!(!msg.contains("secret"));
    }
}
