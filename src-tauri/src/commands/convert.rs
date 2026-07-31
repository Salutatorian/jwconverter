use serde::Deserialize;
use tauri::{AppHandle, State};

use crate::engine::job::{
    BitDepthPreset, ConversionJob, JobStatus, LoudnessPreset, Mp3EncodingMode, NormalizeMode,
    OutputFormat, OverwritePolicy, QualityPreset,
};
use crate::engine::queue::{self, QueueItem};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionRequest {
    pub source_path: String,
    pub destination_dir: String,
    pub output_format: OutputFormat,
    pub source_duration_seconds: Option<f64>,
    #[serde(default)]
    pub relative_subdir: Option<String>,
    #[serde(default)]
    pub overwrite_policy: OverwritePolicy,
    #[serde(default)]
    pub quality_preset: QualityPreset,
    #[serde(default)]
    pub mp3_encoding_mode: Mp3EncodingMode,
    #[serde(default)]
    pub bit_depth_preset: BitDepthPreset,
    #[serde(default = "default_true")]
    pub preserve_tags: bool,
    #[serde(default = "default_true")]
    pub preserve_cover: bool,
    #[serde(default)]
    pub normalize: NormalizeMode,
    #[serde(default)]
    pub loudness_preset: LoudnessPreset,
    #[serde(default)]
    pub trim_silence: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchStartResult {
    pub batch_id: String,
    pub job_ids: Vec<String>,
}

fn build_queue_item(request: ConversionRequest) -> Result<QueueItem, String> {
    let source_path = request.source_path.trim().to_string();
    let destination_dir = request.destination_dir.trim().to_string();

    if source_path.is_empty() {
        return Err("No source file was provided.".to_string());
    }
    if destination_dir.is_empty() {
        return Err("No destination folder was provided.".to_string());
    }

    let relative_subdir = request
        .relative_subdir
        .map(|value| value.trim().trim_matches(['/', '\\']).to_string())
        .filter(|value| !value.is_empty());

    Ok(QueueItem {
        job: ConversionJob {
            id: uuid::Uuid::new_v4().to_string(),
            source_path,
            destination_dir,
            relative_subdir,
            output_format: request.output_format,
            overwrite_policy: request.overwrite_policy,
            quality_preset: request.quality_preset,
            mp3_encoding_mode: request.mp3_encoding_mode,
            bit_depth_preset: request.bit_depth_preset,
            preserve_tags: request.preserve_tags,
            preserve_cover: request.preserve_cover,
            normalize: request.normalize,
            loudness_preset: request.loudness_preset,
            trim_silence: request.trim_silence,
            output_stem: None,
            status: JobStatus::Queued,
        },
        source_duration_seconds: request.source_duration_seconds,
    })
}

/// Start a single-file conversion (batch of one). Returns job id.
#[tauri::command]
pub fn start_conversion(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ConversionRequest,
) -> Result<String, String> {
    let item = build_queue_item(request)?;
    let job_id = item.job.id.clone();
    let (_batch_id, _ids) = queue::enqueue_batch(app, &state, vec![item])?;
    Ok(job_id)
}

/// Start a sequential batch. One FFmpeg process at a time.
#[tauri::command]
pub fn start_batch(
    app: AppHandle,
    state: State<'_, AppState>,
    requests: Vec<ConversionRequest>,
) -> Result<BatchStartResult, String> {
    let mut items = Vec::with_capacity(requests.len());
    for request in requests {
        items.push(build_queue_item(request)?);
    }
    let (batch_id, job_ids) = queue::enqueue_batch(app, &state, items)?;
    Ok(BatchStartResult { batch_id, job_ids })
}

/// Cancel the whole batch (active job + remaining queue).
#[tauri::command]
pub fn cancel_batch(state: State<'_, AppState>) -> Result<(), String> {
    queue::cancel_queue(&state)
}

/// Cancel the currently running job; remaining queued jobs continue.
#[tauri::command]
pub fn cancel_conversion(state: State<'_, AppState>, job_id: String) -> Result<(), String> {
    if state.request_cancel(&job_id) {
        return Ok(());
    }
    Err(format!("No active conversion found for job {job_id}."))
}

#[tauri::command]
pub fn is_batch_running(state: State<'_, AppState>) -> bool {
    queue::is_batch_running(&state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(source: &str, destination: &str) -> ConversionRequest {
        ConversionRequest {
            source_path: source.to_string(),
            destination_dir: destination.to_string(),
            output_format: OutputFormat::Flac,
            source_duration_seconds: Some(1.0),
            relative_subdir: None,
            overwrite_policy: OverwritePolicy::Rename,
            quality_preset: QualityPreset::Medium,
            mp3_encoding_mode: Mp3EncodingMode::Cbr,
            bit_depth_preset: BitDepthPreset::Original,
            preserve_tags: true,
            preserve_cover: true,
            normalize: NormalizeMode::Off,
            loudness_preset: LoudnessPreset::Streaming,
            trim_silence: false,
        }
    }

    #[test]
    fn empty_source_or_destination_rejected() {
        assert!(build_queue_item(request("", "out")).is_err());
        assert!(build_queue_item(request("in.wav", "")).is_err());
        assert!(build_queue_item(request("   ", "out")).is_err());
        assert!(build_queue_item(request("in.wav", "  ")).is_err());
    }

    #[test]
    fn paths_are_trimmed() {
        let item = build_queue_item(request("  in.wav  ", "  out  ")).expect("item");
        assert_eq!(item.job.source_path, "in.wav");
        assert_eq!(item.job.destination_dir, "out");
    }

    #[test]
    fn subdir_is_normalized_and_empty_becomes_none() {
        let mut req = request("in.wav", "out");
        req.relative_subdir = Some(r"/Album A/\".to_string());
        let item = build_queue_item(req).expect("item");
        assert_eq!(item.job.relative_subdir.as_deref(), Some("Album A"));

        let mut req = request("in.wav", "out");
        req.relative_subdir = Some(r"\\".to_string());
        let item = build_queue_item(req).expect("item");
        assert_eq!(item.job.relative_subdir, None);

        let mut req = request("in.wav", "out");
        req.relative_subdir = Some("   ".to_string());
        let item = build_queue_item(req).expect("item");
        assert_eq!(item.job.relative_subdir, None);
    }

    #[test]
    fn job_starts_queued_with_unique_ids() {
        let first = build_queue_item(request("a.wav", "out")).expect("item");
        let second = build_queue_item(request("a.wav", "out")).expect("item");
        assert_eq!(first.job.status, JobStatus::Queued);
        assert_ne!(first.job.id, second.job.id);
        assert!(!first.job.id.is_empty());
    }

    #[test]
    fn processing_options_pass_through() {
        let mut req = request("in.wav", "out");
        req.normalize = NormalizeMode::TwoPass;
        req.loudness_preset = LoudnessPreset::EbuR128;
        req.trim_silence = true;
        let item = build_queue_item(req).expect("item");
        assert_eq!(item.job.normalize, NormalizeMode::TwoPass);
        assert_eq!(item.job.loudness_preset, LoudnessPreset::EbuR128);
        assert!(item.job.trim_silence);
    }
}
