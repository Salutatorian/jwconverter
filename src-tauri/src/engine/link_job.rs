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
}
