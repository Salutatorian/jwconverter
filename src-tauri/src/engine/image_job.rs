//! Image conversion job model (separate from audio OutputFormat).

use serde::{Deserialize, Serialize};

use super::job::{JobStatus, OverwritePolicy};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImageOutputFormat {
    #[default]
    Jpeg,
    Png,
    Webp,
    Tiff,
}

impl ImageOutputFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ImageOutputFormat::Jpeg => "jpg",
            ImageOutputFormat::Png => "png",
            ImageOutputFormat::Webp => "webp",
            ImageOutputFormat::Tiff => "tiff",
        }
    }

    pub fn is_lossy(self) -> bool {
        matches!(self, ImageOutputFormat::Jpeg | ImageOutputFormat::Webp)
    }

    pub fn magick_format(self) -> &'static str {
        match self {
            ImageOutputFormat::Jpeg => "JPEG",
            ImageOutputFormat::Png => "PNG",
            ImageOutputFormat::Webp => "WEBP",
            ImageOutputFormat::Tiff => "TIFF",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImageQualityPreset {
    Low,
    #[default]
    Medium,
    High,
}

impl ImageQualityPreset {
    /// ImageMagick `-quality` for JPEG / WebP.
    pub fn magick_quality(self) -> u8 {
        match self {
            ImageQualityPreset::Low => 70,
            ImageQualityPreset::Medium => 85,
            ImageQualityPreset::High => 95,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageConversionJob {
    pub id: String,
    pub source_path: String,
    pub destination_dir: String,
    #[serde(default)]
    pub relative_subdir: Option<String>,
    pub output_format: ImageOutputFormat,
    #[serde(default)]
    pub overwrite_policy: OverwritePolicy,
    #[serde(default)]
    pub quality_preset: ImageQualityPreset,
    pub status: JobStatus,
}
