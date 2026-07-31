use serde::{Deserialize, Serialize};

use super::job::{
    BitDepthPreset, JobStatus, Mp3EncodingMode, OutputFormat, OverwritePolicy, QualityPreset,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LinkMediaMode {
    #[default]
    Video,
    Audio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum LinkVideoQuality {
    #[default]
    Best,
    Height(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LinkAudioFormat {
    #[default]
    Original,
    Mp3,
    M4a,
    Opus,
    Flac,
    Wav,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkProcessingMode {
    Remux,
    Transcode,
}

impl LinkAudioFormat {
    pub fn extension(self) -> Option<&'static str> {
        match self {
            Self::Original => None,
            Self::Mp3 => Some("mp3"),
            Self::M4a => Some("m4a"),
            Self::Opus => Some("opus"),
            Self::Flac => Some("flac"),
            Self::Wav => Some("wav"),
        }
    }

    pub fn output_format(self) -> Option<OutputFormat> {
        match self {
            Self::Original => None,
            Self::Mp3 => Some(OutputFormat::Mp3),
            Self::M4a => Some(OutputFormat::M4a),
            Self::Opus => Some(OutputFormat::Opus),
            Self::Flac => Some(OutputFormat::Flac),
            Self::Wav => Some(OutputFormat::Wav),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LinkDownloadJob {
    pub id: String,
    pub url: String,
    pub title: Option<String>,
    pub duration_seconds: Option<f64>,
    pub is_live: bool,
    pub is_playlist: bool,
    pub destination_dir: String,
    pub overwrite_policy: OverwritePolicy,
    pub mode: LinkMediaMode,
    pub video_quality: LinkVideoQuality,
    pub audio_format: LinkAudioFormat,
    pub quality_preset: QualityPreset,
    pub mp3_encoding_mode: Mp3EncodingMode,
    pub bit_depth_preset: BitDepthPreset,
    pub cookies_path: Option<String>,
    pub download_subtitles: bool,
    pub save_thumbnail: bool,
    pub embed_thumbnail: bool,
    /// Live downloads are allowed only when this has a recording limit.
    pub live_max_minutes: Option<u32>,
    pub status: JobStatus,
}

impl LinkDownloadJob {
    pub fn processing_mode(&self) -> LinkProcessingMode {
        match self.mode {
            LinkMediaMode::Video => LinkProcessingMode::Remux,
            LinkMediaMode::Audio => match self.audio_format {
                LinkAudioFormat::Original => LinkProcessingMode::Remux,
                _ => LinkProcessingMode::Transcode,
            },
        }
    }
}

pub fn format_selector(job: &LinkDownloadJob) -> String {
    match job.mode {
        LinkMediaMode::Video => match job.video_quality {
            LinkVideoQuality::Best => "bv*+ba/b".to_string(),
            LinkVideoQuality::Height(height) => format!("bv*[height<={height}]+ba/b"),
        },
        LinkMediaMode::Audio => "ba/b".to_string(),
    }
}

/// Pure argv fragments after the yt-dlp executable (excludes URL and output template).
pub fn ytdlp_mode_args(job: &LinkDownloadJob) -> Vec<&'static str> {
    match job.mode {
        LinkMediaMode::Video => vec!["--merge-output-format", "mp4"],
        LinkMediaMode::Audio => Vec::new(),
    }
}

/// Cookie input is opt-in; the application never reads browser cookie stores.
pub fn ytdlp_cookie_args(job: &LinkDownloadJob) -> Vec<String> {
    job.cookies_path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
        .map(|path| vec!["--cookies".to_string(), path.to_string()])
        .unwrap_or_else(|| vec!["--no-cookies".to_string()])
}

pub fn ytdlp_subtitle_args(job: &LinkDownloadJob) -> Vec<&'static str> {
    if job.download_subtitles {
        vec!["--write-subs", "--write-auto-subs", "--sub-langs", "all"]
    } else {
        Vec::new()
    }
}

pub fn ytdlp_thumbnail_args(job: &LinkDownloadJob) -> Vec<&'static str> {
    let mut args = Vec::new();
    if job.save_thumbnail {
        args.push("--write-thumbnail");
    }
    if job.embed_thumbnail {
        args.push("--embed-thumbnail");
    }
    args
}

pub fn ytdlp_live_args(job: &LinkDownloadJob) -> Vec<String> {
    job.live_max_minutes
        .filter(|minutes| *minutes > 0)
        .map(|minutes| {
            vec![
                "--wait-for-video".to_string(),
                "0".to_string(),
                "--download-sections".to_string(),
                format!("*0-{}", minutes.saturating_mul(60)),
            ]
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(mode: LinkMediaMode) -> LinkDownloadJob {
        LinkDownloadJob {
            id: "job-123".to_string(),
            url: "https://example.com/video".to_string(),
            title: None,
            duration_seconds: None,
            is_live: false,
            is_playlist: false,
            destination_dir: ".".to_string(),
            overwrite_policy: OverwritePolicy::Rename,
            mode,
            video_quality: LinkVideoQuality::Best,
            audio_format: LinkAudioFormat::Original,
            quality_preset: QualityPreset::Medium,
            mp3_encoding_mode: Mp3EncodingMode::Cbr,
            bit_depth_preset: BitDepthPreset::Original,
            cookies_path: None,
            download_subtitles: false,
            save_thumbnail: false,
            embed_thumbnail: false,
            live_max_minutes: None,
            status: JobStatus::Queued,
        }
    }

    #[test]
    fn plans_video_selectors() {
        let mut download = job(LinkMediaMode::Video);
        assert_eq!(format_selector(&download), "bv*+ba/b");
        download.video_quality = LinkVideoQuality::Height(720);
        assert_eq!(format_selector(&download), "bv*[height<=720]+ba/b");
        assert_eq!(download.processing_mode(), LinkProcessingMode::Remux);
        assert_eq!(
            ytdlp_mode_args(&download),
            vec!["--merge-output-format", "mp4"]
        );
    }

    #[test]
    fn plans_audio_remux_without_ytdlp_extract() {
        let download = job(LinkMediaMode::Audio);
        assert_eq!(format_selector(&download), "ba/b");
        assert_eq!(download.processing_mode(), LinkProcessingMode::Remux);
        assert!(ytdlp_mode_args(&download).is_empty());
        assert_eq!(download.audio_format.output_format(), None);
    }

    #[test]
    fn plans_audio_transcode_without_ytdlp_extract() {
        let mut download = job(LinkMediaMode::Audio);
        download.audio_format = LinkAudioFormat::Opus;
        assert_eq!(format_selector(&download), "ba/b");
        assert_eq!(download.processing_mode(), LinkProcessingMode::Transcode);
        assert!(ytdlp_mode_args(&download).is_empty());
        assert_eq!(
            download.audio_format.output_format(),
            Some(OutputFormat::Opus)
        );
        assert_eq!(download.audio_format.extension(), Some("opus"));
    }

    #[test]
    fn plans_opt_in_cookie_subtitle_thumbnail_and_live_args() {
        let mut download = job(LinkMediaMode::Audio);
        download.cookies_path = Some("C:\\cookies.txt".to_string());
        download.download_subtitles = true;
        download.save_thumbnail = true;
        download.embed_thumbnail = true;
        download.live_max_minutes = Some(10);

        assert_eq!(
            ytdlp_cookie_args(&download),
            vec!["--cookies", "C:\\cookies.txt"]
        );
        assert_eq!(
            ytdlp_subtitle_args(&download),
            vec!["--write-subs", "--write-auto-subs", "--sub-langs", "all"]
        );
        assert_eq!(
            ytdlp_thumbnail_args(&download),
            vec!["--write-thumbnail", "--embed-thumbnail"]
        );
        assert_eq!(
            ytdlp_live_args(&download),
            vec!["--wait-for-video", "0", "--download-sections", "*0-600"]
        );
    }
}
