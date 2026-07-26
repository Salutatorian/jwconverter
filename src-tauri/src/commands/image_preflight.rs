use serde::Deserialize;
use tauri::command;

use crate::engine::image_job::{ImageOutputFormat, ImageQualityPreset, ImageResizePreset};
use crate::engine::image_preflight::{
    self, ImagePreflightItem, ImagePreflightRequest as EngineRequest,
};
use crate::engine::job::OverwritePolicy;
use crate::engine::preflight::PreflightReport;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagePreflightItemDto {
    pub source_path: String,
    #[serde(default)]
    pub relative_subdir: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub file_size_bytes: Option<u64>,
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagePreflightBatchRequest {
    pub destination_dir: String,
    pub output_format: ImageOutputFormat,
    #[serde(default)]
    pub quality_preset: ImageQualityPreset,
    #[serde(default)]
    pub resize_preset: ImageResizePreset,
    #[serde(default)]
    pub overwrite_policy: OverwritePolicy,
    pub items: Vec<ImagePreflightItemDto>,
}

#[command]
pub fn preflight_image_batch(
    request: ImagePreflightBatchRequest,
) -> Result<PreflightReport, String> {
    let engine_request = EngineRequest {
        destination_dir: request.destination_dir,
        output_format: request.output_format,
        quality_preset: request.quality_preset.normalize_for(request.output_format),
        resize_preset: request.resize_preset,
        overwrite_policy: request.overwrite_policy,
        items: request
            .items
            .into_iter()
            .map(|item| ImagePreflightItem {
                source_path: item.source_path,
                relative_subdir: item.relative_subdir,
                width: item.width,
                height: item.height,
                file_size_bytes: item.file_size_bytes,
                format: item.format,
            })
            .collect(),
    };

    image_preflight::run_image_preflight(&engine_request).map_err(|e| e.to_string())
}
