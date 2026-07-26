use serde::Deserialize;
use tauri::{AppHandle, State};

use crate::engine::image_job::{
    ImageConversionJob, ImageOutputFormat, ImageQualityPreset, ImageResizePreset,
};
use crate::engine::image_queue::{self, ImageQueueItem};
use crate::engine::job::{JobStatus, OverwritePolicy};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageConversionRequest {
    pub source_path: String,
    pub destination_dir: String,
    pub output_format: ImageOutputFormat,
    #[serde(default)]
    pub relative_subdir: Option<String>,
    #[serde(default)]
    pub overwrite_policy: OverwritePolicy,
    #[serde(default)]
    pub quality_preset: ImageQualityPreset,
    #[serde(default)]
    pub resize_preset: ImageResizePreset,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageBatchStartResult {
    pub batch_id: String,
    pub job_ids: Vec<String>,
}

fn build_item(request: ImageConversionRequest) -> Result<ImageQueueItem, String> {
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

    Ok(ImageQueueItem {
        job: ImageConversionJob {
            id: uuid::Uuid::new_v4().to_string(),
            source_path,
            destination_dir,
            relative_subdir,
            output_format: request.output_format,
            overwrite_policy: request.overwrite_policy,
            quality_preset: request.quality_preset.normalize_for(request.output_format),
            resize_preset: request.resize_preset,
            status: JobStatus::Queued,
        },
    })
}

#[tauri::command]
pub fn start_image_batch(
    app: AppHandle,
    state: State<'_, AppState>,
    requests: Vec<ImageConversionRequest>,
) -> Result<ImageBatchStartResult, String> {
    let mut items = Vec::with_capacity(requests.len());
    for request in requests {
        items.push(build_item(request)?);
    }
    let (batch_id, job_ids) = image_queue::enqueue_batch(app, &state, items)?;
    Ok(ImageBatchStartResult { batch_id, job_ids })
}

#[tauri::command]
pub fn cancel_image_batch(state: State<'_, AppState>) -> Result<(), String> {
    image_queue::cancel_queue(&state)
}

#[tauri::command]
pub fn is_image_batch_running(state: State<'_, AppState>) -> bool {
    image_queue::is_batch_running(&state)
}

#[tauri::command]
pub fn analyze_image(path: String) -> Result<crate::media::imagemagick::ImageInfo, String> {
    crate::media::imagemagick::analyze(path.trim()).map_err(|e| e.to_string())
}
