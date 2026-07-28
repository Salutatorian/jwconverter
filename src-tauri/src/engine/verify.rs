//! Post-conversion FFprobe verification against the requested plan.

use std::path::Path;

use crate::engine::planner::EncoderPlan;
use crate::errors::AppError;
use crate::media::ffprobe;

pub struct VerificationContext {
    pub source_duration_seconds: Option<f64>,
}

pub fn verify_output(
    path: &Path,
    plan: &EncoderPlan,
    context: &VerificationContext,
) -> Result<(), AppError> {
    if !path.is_file() {
        return Err(AppError::VerificationFailure {
            detail: "Output file was not created.".to_string(),
        });
    }

    let meta = std::fs::metadata(path).map_err(|error| AppError::VerificationFailure {
        detail: format!("Could not read output file: {error}"),
    })?;
    if meta.len() == 0 {
        return Err(AppError::VerificationFailure {
            detail: "Output file is empty.".to_string(),
        });
    }

    let info = ffprobe::analyze(path.to_string_lossy().as_ref())?;

    if info.codec.is_none() {
        return Err(AppError::VerificationFailure {
            detail: "Output has no readable audio codec.".to_string(),
        });
    }

    if !codec_matches(plan, info.codec.as_deref()) {
        return Err(AppError::VerificationFailure {
            detail: format!(
                "Output codec mismatch. Expected {}, got {}.",
                plan.audio_codec,
                info.codec.unwrap_or_else(|| "unknown".to_string())
            ),
        });
    }

    if let (Some(expected), Some(actual)) = (context.source_duration_seconds, info.duration_seconds)
    {
        if expected > 0.05 {
            let delta = (expected - actual).abs();
            let tolerance = (expected * 0.08).max(0.35);
            if delta > tolerance {
                return Err(AppError::VerificationFailure {
                    detail: format!(
                        "Output duration looks wrong (source {expected:.2}s, output {actual:.2}s)."
                    ),
                });
            }
        }
    }

    if info.sample_rate.is_none() || info.channels.is_none() {
        return Err(AppError::VerificationFailure {
            detail: "Output audio stream is missing sample rate or channel information."
                .to_string(),
        });
    }

    Ok(())
}

fn codec_matches(plan: &EncoderPlan, codec: Option<&str>) -> bool {
    let Some(codec) = codec else {
        return false;
    };
    match plan.format {
        crate::engine::job::OutputFormat::Wav => codec == "pcm_s16le" || codec.starts_with("pcm_"),
        crate::engine::job::OutputFormat::Flac => codec == "flac",
        crate::engine::job::OutputFormat::Mp3 => codec == "mp3",
        crate::engine::job::OutputFormat::Aac | crate::engine::job::OutputFormat::M4a => {
            codec == "aac"
        }
        crate::engine::job::OutputFormat::Opus => codec == "opus",
        crate::engine::job::OutputFormat::Ogg => codec == "vorbis",
        crate::engine::job::OutputFormat::Alac => codec == "alac",
        crate::engine::job::OutputFormat::Aiff => codec == "pcm_s16be" || codec.starts_with("pcm_"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::job::{BitDepthPreset, Mp3EncodingMode, OutputFormat, QualityPreset};
    use crate::engine::planner;

    fn plan(format: OutputFormat) -> EncoderPlan {
        planner::plan_for(
            format,
            QualityPreset::Medium,
            BitDepthPreset::Original,
            None,
            Mp3EncodingMode::Cbr,
        )
    }

    #[test]
    fn codec_matches_accepts_expected_codecs() {
        assert!(codec_matches(&plan(OutputFormat::Flac), Some("flac")));
        assert!(codec_matches(&plan(OutputFormat::Mp3), Some("mp3")));
        assert!(codec_matches(&plan(OutputFormat::M4a), Some("aac")));
        assert!(codec_matches(&plan(OutputFormat::Aac), Some("aac")));
        assert!(codec_matches(&plan(OutputFormat::Opus), Some("opus")));
        assert!(codec_matches(&plan(OutputFormat::Ogg), Some("vorbis")));
        assert!(codec_matches(&plan(OutputFormat::Alac), Some("alac")));
        assert!(codec_matches(&plan(OutputFormat::Wav), Some("pcm_s16le")));
        assert!(codec_matches(&plan(OutputFormat::Wav), Some("pcm_s24le")));
        assert!(codec_matches(&plan(OutputFormat::Aiff), Some("pcm_s16be")));
        assert!(codec_matches(&plan(OutputFormat::Aiff), Some("pcm_f32be")));
    }

    #[test]
    fn codec_matches_rejects_wrong_codecs() {
        assert!(!codec_matches(&plan(OutputFormat::Flac), Some("mp3")));
        assert!(!codec_matches(&plan(OutputFormat::Mp3), Some("aac")));
        assert!(!codec_matches(&plan(OutputFormat::Opus), Some("vorbis")));
        assert!(!codec_matches(&plan(OutputFormat::Ogg), Some("opus")));
        assert!(!codec_matches(&plan(OutputFormat::Wav), Some("flac")));
        assert!(!codec_matches(&plan(OutputFormat::Alac), Some("aac")));
    }

    #[test]
    fn codec_matches_rejects_missing_codec() {
        for format in [
            OutputFormat::Wav,
            OutputFormat::Flac,
            OutputFormat::Mp3,
            OutputFormat::M4a,
            OutputFormat::Aac,
            OutputFormat::Opus,
            OutputFormat::Ogg,
            OutputFormat::Alac,
            OutputFormat::Aiff,
        ] {
            assert!(!codec_matches(&plan(format), None), "{format:?}");
        }
    }
}
