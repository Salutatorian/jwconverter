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

/// MP3 only. Ignored for other formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mp3EncodingMode {
    #[default]
    Cbr,
    Vbr,
}

/// Loudness normalization mode for the audio conversion pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum NormalizeMode {
    /// No loudness processing.
    #[default]
    Off,
    /// Single-pass dynamic loudnorm (fast, less precise).
    OnePass,
    /// Measure-then-convert linear loudnorm (slower, sample-accurate).
    TwoPass,
}

/// Loudness target preset for normalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum LoudnessPreset {
    /// -14 LUFS / -1 dBTP (streaming platforms).
    #[default]
    Streaming,
    /// -23 LUFS / -1 dBTP (broadcast EBU R128).
    EbuR128,
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
    /// MP3 CBR vs VBR. Ignored unless `output_format` is MP3.
    #[serde(default)]
    pub mp3_encoding_mode: Mp3EncodingMode,
    #[serde(default)]
    pub bit_depth_preset: BitDepthPreset,
    /// Keep container tags / chapters from the source when possible.
    #[serde(default = "default_true")]
    pub preserve_tags: bool,
    /// Keep embedded cover art when the destination format supports it.
    #[serde(default = "default_true")]
    pub preserve_cover: bool,
    /// Loudness normalization applied during conversion.
    #[serde(default)]
    pub normalize: NormalizeMode,
    /// Target preset used when `normalize` is not Off.
    #[serde(default)]
    pub loudness_preset: LoudnessPreset,
    /// Remove silent regions detected by an analysis pre-pass.
    #[serde(default)]
    pub trim_silence: bool,
    /// Optional final filename stem. When unset, derived from the source path.
    #[serde(default)]
    pub output_stem: Option<String>,
    pub status: JobStatus,
}

fn default_true() -> bool {
    true
}

/// Supported output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Wav,
    Flac,
    Mp3,
    /// AAC in an M4A/MP4 container (`.m4a`).
    M4a,
    /// Raw AAC ADTS bitstream (`.aac`).
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
            OutputFormat::Mp3
                | OutputFormat::M4a
                | OutputFormat::Aac
                | OutputFormat::Opus
                | OutputFormat::Ogg
        )
    }

    /// Destinations that can usefully carry an embedded cover / attached picture.
    pub fn supports_embedded_cover(self) -> bool {
        matches!(
            self,
            OutputFormat::Mp3
                | OutputFormat::Flac
                | OutputFormat::M4a
                | OutputFormat::Alac
                | OutputFormat::Ogg
                | OutputFormat::Opus
        )
    }
}
