use serde::Deserialize;
use tauri::{AppHandle, State};

use crate::engine::job::{ConversionJob, JobStatus, OutputFormat, OverwritePolicy, QualityPreset};
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
    queue::cancel_current_job(&state)
}

#[tauri::command]
pub fn is_batch_running(state: State<'_, AppState>) -> bool {
    queue::is_batch_running(&state)
}
