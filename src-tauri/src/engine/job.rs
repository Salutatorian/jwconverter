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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum QualityPreset {
    Low,
    #[default]
    Medium,
    High,
}

/// PCM bit depth for WAV / AIFF. Ignored for other formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum BitDepthPreset {
    #[default]
    Original,
    #[serde(rename = "16")]
    Bit16,
    #[serde(rename = "24")]
    Bit24,
    #[serde(rename = "float32")]
    Float32,
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
    #[serde(default)]
    pub quality_preset: QualityPreset,
    #[serde(default)]
    pub bit_depth_preset: BitDepthPreset,
    pub status: JobStatus,
}

/// Supported output formats.
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

impl OutputFormat {
    pub fn is_lossy(self) -> bool {
        matches!(
            self,
            OutputFormat::Mp3 | OutputFormat::Aac | OutputFormat::Opus | OutputFormat::Ogg
        )
    }
}
