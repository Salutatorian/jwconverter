//! Job model for a single conversion unit.
//! Batch conversion will reuse this — one file is one job.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobStatus {
    Idle,
    Analyzing,
    Ready,
    Queued,
    Converting,
    Verifying,
    Completed,
    Failed,
    Cancelled,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OverwritePolicy {
    #[default]
    Rename,
    Skip,
    Replace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionJob {
    pub id: String,
    pub source_path: String,
    pub destination_dir: String,
    /// Optional subdirectory under destination, preserving folder import structure.
    /// Example: `Album A` or `Music/Album A`.
    #[serde(default)]
    pub relative_subdir: Option<String>,
    pub output_format: OutputFormat,
    #[serde(default)]
    pub overwrite_policy: OverwritePolicy,
    pub status: JobStatus,
}

/// Supported output formats. Quality presets stay fixed for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Wav,
    Flac,
    Mp3,
    Aac,
    Opus,
    Ogg,
    Alac,
    Aiff,
}
