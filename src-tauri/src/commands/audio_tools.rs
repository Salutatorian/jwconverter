//! Loudness measurement and silence detection commands.
//! Analysis only — never modifies the source file.

use std::path::PathBuf;

use serde::Serialize;

use crate::engine::job::LoudnessPreset;
use crate::media::loudness::{
    self, KeepSegment, LoudnessMeasurement, LoudnessTarget, SilenceSpan,
};
use crate::media::{ffmpeg, ffprobe};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoudnessReport {
    pub path: String,
    pub measurement: LoudnessMeasurement,
    pub target: LoudnessTarget,
    /// Filter the conversion pass will use for a linear two-pass normalize.
    pub second_pass_filter: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SilenceReport {
    pub path: String,
    pub duration_seconds: Option<f64>,
    pub noise_db: f64,
    pub min_duration_seconds: f64,
    pub spans: Vec<SilenceSpan>,
    pub keep_segments: Vec<KeepSegment>,
    /// True when trimming would remove the entire file.
    pub all_silence: bool,
}

fn validated_source(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path.trim());
    if path.as_os_str().is_empty() {
        return Err("No source file was provided.".to_string());
    }
    if !path.is_file() {
        return Err(format!("File not found: {}", path.display()));
    }
    Ok(path)
}

/// Measure integrated loudness / true peak / LRA with a loudnorm analysis pass.
#[tauri::command]
pub fn measure_loudness(path: String, preset: Option<LoudnessPreset>) -> Result<LoudnessReport, String> {
    let source = validated_source(&path)?;
    let ffmpeg = ffmpeg::resolve_ffmpeg_required().map_err(|error| error.to_string())?;

    let target = match preset.unwrap_or_default() {
        LoudnessPreset::Streaming => LoudnessTarget::streaming(),
        LoudnessPreset::EbuR128 => LoudnessTarget::ebu_r128(),
    };

    let measurement =
        loudness::measure_loudness(&ffmpeg, &source, &target).map_err(|error| error.to_string())?;
    let second_pass_filter = loudness::build_loudnorm_filter(&target, Some(&measurement))
        .ok_or_else(|| "Could not build a normalization filter for this file.".to_string())?;

    Ok(LoudnessReport {
        path: source.to_string_lossy().into_owned(),
        measurement,
        target,
        second_pass_filter,
    })
}

/// Detect silent regions and compute the segments a trim would keep.
#[tauri::command]
pub fn detect_silence(
    path: String,
    noise_db: Option<f64>,
    min_duration_seconds: Option<f64>,
) -> Result<SilenceReport, String> {
    let source = validated_source(&path)?;
    let ffmpeg = ffmpeg::resolve_ffmpeg_required().map_err(|error| error.to_string())?;

    let noise_db = noise_db.unwrap_or(loudness::DEFAULT_SILENCE_NOISE_DB);
    let min_duration_seconds =
        min_duration_seconds.unwrap_or(loudness::DEFAULT_SILENCE_MIN_DURATION);

    let duration_seconds = ffprobe::analyze(&source.to_string_lossy())
        .ok()
        .and_then(|info| info.duration_seconds);

    let spans = loudness::detect_silence(&ffmpeg, &source, noise_db, min_duration_seconds)
        .map_err(|error| error.to_string())?;

    let keep_segments = match duration_seconds {
        Some(duration) => loudness::keep_segments(&spans, duration),
        None => Vec::new(),
    };
    let all_silence = duration_seconds.is_some() && keep_segments.is_empty();

    Ok(SilenceReport {
        path: source.to_string_lossy().into_owned(),
        duration_seconds,
        noise_db,
        min_duration_seconds,
        spans,
        keep_segments,
        all_silence,
    })
}
