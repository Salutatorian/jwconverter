//! Turns a conversion request into an encoder plan.
//! Owns format → codec / container / quality decisions. No process spawning here.

use super::job::{OutputFormat, QualityPreset};

#[derive(Debug, Clone)]
pub struct EncoderPlan {
    pub format: OutputFormat,
    pub quality: QualityPreset,
    pub container: &'static str,
    pub audio_codec: &'static str,
    pub extension: &'static str,
}

impl EncoderPlan {
    pub fn ffmpeg_audio_args(&self) -> Vec<&'static str> {
        match self.format {
            OutputFormat::Wav => vec!["-c:a", "pcm_s16le"],
            OutputFormat::Flac => vec!["-c:a", "flac"],
            OutputFormat::Alac => vec!["-c:a", "alac"],
            OutputFormat::Aiff => vec!["-c:a", "pcm_s16be"],
            OutputFormat::Mp3 => {
                let bitrate = match self.quality {
                    QualityPreset::Low => "128k",
                    QualityPreset::Medium => "192k",
                    QualityPreset::High => "320k",
                };
                vec!["-c:a", "libmp3lame", "-b:a", bitrate]
            }
            OutputFormat::Aac => {
                let bitrate = match self.quality {
                    QualityPreset::Low => "128k",
                    QualityPreset::Medium => "192k",
                    QualityPreset::High => "256k",
                };
                vec!["-c:a", "aac", "-b:a", bitrate]
            }
            OutputFormat::Opus => {
                let bitrate = match self.quality {
                    QualityPreset::Low => "96k",
                    QualityPreset::Medium => "160k",
                    QualityPreset::High => "192k",
                };
                vec!["-c:a", "libopus", "-b:a", bitrate]
            }
            OutputFormat::Ogg => {
                let q = match self.quality {
                    QualityPreset::Low => "3",
                    QualityPreset::Medium => "5",
                    QualityPreset::High => "7",
                };
                vec!["-c:a", "libvorbis", "-q:a", q]
            }
        }
    }
}

pub fn plan_for(format: OutputFormat, quality: QualityPreset) -> EncoderPlan {
    let quality = if format.is_lossy() {
        quality
    } else {
        QualityPreset::Medium
    };

    match format {
        OutputFormat::Wav => EncoderPlan {
            format,
            quality,
            container: "wav",
            audio_codec: "pcm_s16le",
            extension: "wav",
        },
        OutputFormat::Flac => EncoderPlan {
            format,
            quality,
            container: "flac",
            audio_codec: "flac",
            extension: "flac",
        },
        OutputFormat::Mp3 => EncoderPlan {
            format,
            quality,
            container: "mp3",
            audio_codec: "libmp3lame",
            extension: "mp3",
        },
        OutputFormat::Aac => EncoderPlan {
            format,
            quality,
            container: "m4a",
            audio_codec: "aac",
            extension: "m4a",
        },
        OutputFormat::Opus => EncoderPlan {
            format,
            quality,
            container: "opus",
            audio_codec: "libopus",
            extension: "opus",
        },
        OutputFormat::Ogg => EncoderPlan {
            format,
            quality,
            container: "ogg",
            audio_codec: "libvorbis",
            extension: "ogg",
        },
        OutputFormat::Alac => EncoderPlan {
            format,
            quality,
            container: "m4a",
            audio_codec: "alac",
            extension: "m4a",
        },
        OutputFormat::Aiff => EncoderPlan {
            format,
            quality,
            container: "aiff",
            audio_codec: "pcm_s16be",
            extension: "aiff",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medium_mp3_matches_legacy_default() {
        let plan = plan_for(OutputFormat::Mp3, QualityPreset::Medium);
        assert_eq!(
            plan.ffmpeg_audio_args(),
            vec!["-c:a", "libmp3lame", "-b:a", "192k"]
        );
    }

    #[test]
    fn low_and_high_mp3_change_bitrate() {
        let low = plan_for(OutputFormat::Mp3, QualityPreset::Low);
        let high = plan_for(OutputFormat::Mp3, QualityPreset::High);
        assert_eq!(low.ffmpeg_audio_args()[3], "128k");
        assert_eq!(high.ffmpeg_audio_args()[3], "320k");
    }

    #[test]
    fn lossless_ignores_quality_in_args() {
        let low = plan_for(OutputFormat::Flac, QualityPreset::Low);
        let high = plan_for(OutputFormat::Flac, QualityPreset::High);
        assert_eq!(low.ffmpeg_audio_args(), high.ffmpeg_audio_args());
        assert_eq!(low.ffmpeg_audio_args(), vec!["-c:a", "flac"]);
    }
}
