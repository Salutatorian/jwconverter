use serde::{Deserialize, Serialize};

use super::job::{JobStatus, OverwritePolicy};

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

impl LinkAudioFormat {
    pub fn ytdlp_format(self) -> &'static str {
        match self {
            Self::Original => "best",
            Self::Mp3 => "mp3",
            Self::M4a => "m4a",
            Self::Opus => "opus",
            Self::Flac => "flac",
            Self::Wav => "wav",
        }
    }

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
}

#[derive(Debug, Clone)]
pub struct LinkDownloadJob {
    pub id: String,
    pub url: String,
    pub title: Option<String>,
    pub is_live: bool,
    pub is_playlist: bool,
    pub destination_dir: String,
    pub overwrite_policy: OverwritePolicy,
    pub mode: LinkMediaMode,
    pub video_quality: LinkVideoQuality,
    pub audio_format: LinkAudioFormat,
    pub status: JobStatus,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn job(mode: LinkMediaMode) -> LinkDownloadJob {
        LinkDownloadJob {
            id: "job-123".to_string(),
            url: "https://example.com/video".to_string(),
            title: None,
            is_live: false,
            is_playlist: false,
            destination_dir: ".".to_string(),
            overwrite_policy: OverwritePolicy::Rename,
            mode,
            video_quality: LinkVideoQuality::Best,
            audio_format: LinkAudioFormat::Original,
            status: JobStatus::Queued,
        }
    }

    #[test]
    fn plans_video_selectors() {
        let mut download = job(LinkMediaMode::Video);
        assert_eq!(format_selector(&download), "bv*+ba/b");
        download.video_quality = LinkVideoQuality::Height(720);
        assert_eq!(format_selector(&download), "bv*[height<=720]+ba/b");
    }

    #[test]
    fn plans_audio_selector_and_extraction_format() {
        let mut download = job(LinkMediaMode::Audio);
        download.audio_format = LinkAudioFormat::Opus;
        assert_eq!(format_selector(&download), "ba/b");
        assert_eq!(download.audio_format.ytdlp_format(), "opus");
        assert_eq!(download.audio_format.extension(), Some("opus"));
    }
}
