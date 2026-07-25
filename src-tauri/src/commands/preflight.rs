use serde::Deserialize;
use tauri::command;

use crate::engine::job::{BitDepthPreset, OutputFormat, OverwritePolicy, QualityPreset};
use crate::engine::preflight::{
    self, PreflightItem, PreflightReport, PreflightRequest as EnginePreflightRequest,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightItemDto {
    pub source_path: String,
    #[serde(default)]
    pub relative_subdir: Option<String>,
    pub duration_seconds: Option<f64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub file_size_bytes: Option<u64>,
    pub codec: Option<String>,
    pub format: Option<String>,
    pub bit_depth: Option<u32>,
    pub bits_per_raw_sample: Option<u32>,
    pub sample_format: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightBatchRequest {
    pub destination_dir: String,
    pub output_format: OutputFormat,
    #[serde(default)]
    pub quality_preset: QualityPreset,
    #[serde(default)]
    pub bit_depth_preset: BitDepthPreset,
    #[serde(default)]
    pub overwrite_policy: OverwritePolicy,
    pub items: Vec<PreflightItemDto>,
}

#[command]
pub fn preflight_batch(request: PreflightBatchRequest) -> Result<PreflightReport, String> {
    let engine_request = EnginePreflightRequest {
        destination_dir: request.destination_dir,
        output_format: request.output_format,
        quality_preset: request.quality_preset,
        bit_depth_preset: request.bit_depth_preset,
        overwrite_policy: request.overwrite_policy,
        items: request
            .items
            .into_iter()
            .map(|item| PreflightItem {
                source_path: item.source_path,
                relative_subdir: item.relative_subdir,
                duration_seconds: item.duration_seconds,
                sample_rate: item.sample_rate,
                channels: item.channels,
                file_size_bytes: item.file_size_bytes,
                codec: item.codec,
                format: item.format,
                bit_depth: item.bit_depth,
                bits_per_raw_sample: item.bits_per_raw_sample,
                sample_format: item.sample_format,
            })
            .collect(),
    };

    preflight::run_preflight(&engine_request).map_err(|e| e.to_string())
}
