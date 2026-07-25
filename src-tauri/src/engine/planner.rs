//! Turns a conversion request into an encoder plan.
//! Owns format → codec / container decisions. No process spawning here.

use super::job::OutputFormat;

#[derive(Debug, Clone)]
pub struct EncoderPlan {
    pub format: OutputFormat,
    pub container: &'static str,
    pub audio_codec: &'static str,
    pub extension: &'static str,
}

impl EncoderPlan {
    pub fn ffmpeg_audio_args(&self) -> Vec<&'static str> {
        match self.format {
            OutputFormat::Wav => vec!["-c:a", "pcm_s16le"],
            OutputFormat::Flac => vec!["-c:a", "flac"],
            // Sensible default quality for V0.1 — presets come later.
            OutputFormat::Mp3 => vec!["-c:a", "libmp3lame", "-b:a", "192k"],
        }
    }
}

pub fn plan_for(format: OutputFormat) -> EncoderPlan {
    match format {
        OutputFormat::Wav => EncoderPlan {
            format,
            container: "wav",
            audio_codec: "pcm_s16le",
            extension: "wav",
        },
        OutputFormat::Flac => EncoderPlan {
            format,
            container: "flac",
            audio_codec: "flac",
            extension: "flac",
        },
        OutputFormat::Mp3 => EncoderPlan {
            format,
            container: "mp3",
            audio_codec: "libmp3lame",
            extension: "mp3",
        },
    }
}
