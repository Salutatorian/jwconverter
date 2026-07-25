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
        crate::engine::job::OutputFormat::Aac => codec == "aac",
        crate::engine::job::OutputFormat::Opus => codec == "opus",
        crate::engine::job::OutputFormat::Ogg => codec == "vorbis",
        crate::engine::job::OutputFormat::Alac => codec == "alac",
        crate::engine::job::OutputFormat::Aiff => codec == "pcm_s16be" || codec.starts_with("pcm_"),
    }
}
