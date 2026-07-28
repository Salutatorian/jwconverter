//! Image conversion job model (separate from audio OutputFormat).

use serde::{Deserialize, Serialize};

use super::job::{JobStatus, OverwritePolicy};

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImageOutputFormat {
    #[default]
    Jpeg,
    Png,
    Webp,
    Tiff,
    Bmp,
    Gif,
    Avif,
}

impl ImageOutputFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ImageOutputFormat::Jpeg => "jpg",
            ImageOutputFormat::Png => "png",
            ImageOutputFormat::Webp => "webp",
            ImageOutputFormat::Tiff => "tiff",
            ImageOutputFormat::Bmp => "bmp",
            ImageOutputFormat::Gif => "gif",
            ImageOutputFormat::Avif => "avif",
        }
    }

    /// True when the chosen quality encodes lossy pixels (WebP lossless is not).
    pub fn is_lossy_with(self, quality: ImageQualityPreset) -> bool {
        match self {
            ImageOutputFormat::Jpeg | ImageOutputFormat::Gif | ImageOutputFormat::Avif => true,
            ImageOutputFormat::Webp => !quality.is_lossless(),
            ImageOutputFormat::Png | ImageOutputFormat::Tiff | ImageOutputFormat::Bmp => false,
        }
    }

    pub fn shows_quality_controls(self) -> bool {
        matches!(
            self,
            ImageOutputFormat::Jpeg
                | ImageOutputFormat::Png
                | ImageOutputFormat::Webp
                | ImageOutputFormat::Avif
        )
    }

    pub fn magick_format(self) -> &'static str {
        match self {
            ImageOutputFormat::Jpeg => "JPEG",
            ImageOutputFormat::Png => "PNG",
            ImageOutputFormat::Webp => "WEBP",
            ImageOutputFormat::Tiff => "TIFF",
            ImageOutputFormat::Bmp => "BMP",
            ImageOutputFormat::Gif => "GIF",
            ImageOutputFormat::Avif => "AVIF",
        }
    }

    /// Magick identify format aliases that still count as a match.
    pub fn matches_identified(self, actual: &str) -> bool {
        let expected = self.magick_format();
        if actual.eq_ignore_ascii_case(expected) {
            return true;
        }
        match self {
            ImageOutputFormat::Jpeg => actual.eq_ignore_ascii_case("JPG"),
            ImageOutputFormat::Gif => actual.eq_ignore_ascii_case("GIF87"),
            ImageOutputFormat::Bmp => {
                actual.eq_ignore_ascii_case("BMP2") || actual.eq_ignore_ascii_case("BMP3")
            }
            _ => false,
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
    Lossless,
}

impl ImageQualityPreset {
    pub fn is_lossless(self) -> bool {
        matches!(self, ImageQualityPreset::Lossless)
    }

    /// Coerce Lossless → Medium when the format does not support it.
    pub fn normalize_for(self, format: ImageOutputFormat) -> Self {
        if self.is_lossless() && !matches!(format, ImageOutputFormat::Webp) {
            ImageQualityPreset::Medium
        } else {
            self
        }
    }

    /// Magick `-quality` when applicable (None for WebP lossless / TIFF / BMP / GIF).
    pub fn magick_quality_for(self, format: ImageOutputFormat) -> Option<u8> {
        let quality = self.normalize_for(format);
        match format {
            ImageOutputFormat::Jpeg | ImageOutputFormat::Avif => Some(match quality {
                ImageQualityPreset::Low => 70,
                ImageQualityPreset::Medium => 85,
                ImageQualityPreset::High | ImageQualityPreset::Lossless => 95,
            }),
            ImageOutputFormat::Webp => {
                if quality.is_lossless() {
                    None
                } else {
                    Some(match quality {
                        ImageQualityPreset::Low => 70,
                        ImageQualityPreset::Medium => 85,
                        ImageQualityPreset::High | ImageQualityPreset::Lossless => 95,
                    })
                }
            }
            // PNG: higher Magick quality ≈ less zlib compression (faster / larger).
            ImageOutputFormat::Png => Some(match quality {
                ImageQualityPreset::Low => 90,
                ImageQualityPreset::Medium => 75,
                ImageQualityPreset::High | ImageQualityPreset::Lossless => 50,
            }),
            ImageOutputFormat::Tiff
            | ImageOutputFormat::Bmp
            | ImageOutputFormat::Gif => None,
        }
    }
}

/// Long-edge max; never upscales (Magick `>` geometry).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ImageResizePreset {
    #[default]
    Original,
    #[serde(rename = "2048")]
    Max2048,
    #[serde(rename = "1920")]
    Max1920,
    #[serde(rename = "1280")]
    Max1280,
    #[serde(rename = "1024")]
    Max1024,
}

impl ImageResizePreset {
    pub fn max_long_edge(self) -> Option<u32> {
        match self {
            ImageResizePreset::Original => None,
            ImageResizePreset::Max2048 => Some(2048),
            ImageResizePreset::Max1920 => Some(1920),
            ImageResizePreset::Max1280 => Some(1280),
            ImageResizePreset::Max1024 => Some(1024),
        }
    }

    /// Magick geometry like `1920x1920>` (shrink only).
    pub fn magick_geometry(self) -> Option<String> {
        self.max_long_edge()
            .map(|edge| format!("{edge}x{edge}>"))
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
    #[serde(default)]
    pub resize_preset: ImageResizePreset,
    /// Keep EXIF / ICC / comments when the destination format can carry them.
    #[serde(default = "default_true")]
    pub preserve_metadata: bool,
    pub status: JobStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_format_has_distinct_extension() {
        let formats = [
            ImageOutputFormat::Jpeg,
            ImageOutputFormat::Png,
            ImageOutputFormat::Webp,
            ImageOutputFormat::Tiff,
            ImageOutputFormat::Bmp,
            ImageOutputFormat::Gif,
            ImageOutputFormat::Avif,
        ];
        let mut extensions: Vec<&str> = formats.iter().map(|f| f.extension()).collect();
        extensions.sort_unstable();
        extensions.dedup();
        assert_eq!(extensions.len(), formats.len());
        assert_eq!(ImageOutputFormat::Jpeg.extension(), "jpg");
    }

    #[test]
    fn lossy_classification_depends_on_quality_for_webp() {
        assert!(ImageOutputFormat::Jpeg.is_lossy_with(ImageQualityPreset::Medium));
        assert!(ImageOutputFormat::Gif.is_lossy_with(ImageQualityPreset::Medium));
        assert!(ImageOutputFormat::Avif.is_lossy_with(ImageQualityPreset::Medium));
        assert!(ImageOutputFormat::Webp.is_lossy_with(ImageQualityPreset::Medium));
        assert!(!ImageOutputFormat::Webp.is_lossy_with(ImageQualityPreset::Lossless));
        assert!(!ImageOutputFormat::Png.is_lossy_with(ImageQualityPreset::Medium));
        assert!(!ImageOutputFormat::Tiff.is_lossy_with(ImageQualityPreset::Medium));
        assert!(!ImageOutputFormat::Bmp.is_lossy_with(ImageQualityPreset::Medium));
    }

    #[test]
    fn quality_controls_shown_for_formats_with_meaningful_knob() {
        assert!(ImageOutputFormat::Jpeg.shows_quality_controls());
        assert!(ImageOutputFormat::Png.shows_quality_controls());
        assert!(ImageOutputFormat::Webp.shows_quality_controls());
        assert!(ImageOutputFormat::Avif.shows_quality_controls());
        assert!(!ImageOutputFormat::Tiff.shows_quality_controls());
        assert!(!ImageOutputFormat::Bmp.shows_quality_controls());
        assert!(!ImageOutputFormat::Gif.shows_quality_controls());
    }

    #[test]
    fn magick_format_names_are_uppercase() {
        assert_eq!(ImageOutputFormat::Jpeg.magick_format(), "JPEG");
        assert_eq!(ImageOutputFormat::Webp.magick_format(), "WEBP");
        assert_eq!(ImageOutputFormat::Avif.magick_format(), "AVIF");
    }

    #[test]
    fn matches_identified_accepts_known_aliases() {
        assert!(ImageOutputFormat::Jpeg.matches_identified("JPG"));
        assert!(ImageOutputFormat::Jpeg.matches_identified("jpeg"));
        assert!(ImageOutputFormat::Gif.matches_identified("GIF87"));
        assert!(ImageOutputFormat::Bmp.matches_identified("BMP2"));
        assert!(ImageOutputFormat::Bmp.matches_identified("BMP3"));
        assert!(!ImageOutputFormat::Png.matches_identified("PNG24"));
        assert!(!ImageOutputFormat::Webp.matches_identified("JPEG"));
    }

    #[test]
    fn normalize_for_coerces_lossless_off_unsupported_formats() {
        assert_eq!(
            ImageQualityPreset::Lossless.normalize_for(ImageOutputFormat::Webp),
            ImageQualityPreset::Lossless
        );
        assert_eq!(
            ImageQualityPreset::Lossless.normalize_for(ImageOutputFormat::Jpeg),
            ImageQualityPreset::Medium
        );
        assert_eq!(
            ImageQualityPreset::High.normalize_for(ImageOutputFormat::Jpeg),
            ImageQualityPreset::High
        );
    }

    #[test]
    fn magick_quality_matrix() {
        assert_eq!(
            ImageQualityPreset::Low.magick_quality_for(ImageOutputFormat::Jpeg),
            Some(70)
        );
        assert_eq!(
            ImageQualityPreset::Medium.magick_quality_for(ImageOutputFormat::Jpeg),
            Some(85)
        );
        assert_eq!(
            ImageQualityPreset::High.magick_quality_for(ImageOutputFormat::Jpeg),
            Some(95)
        );
        // Lossless coerces to Medium outside WebP.
        assert_eq!(
            ImageQualityPreset::Lossless.magick_quality_for(ImageOutputFormat::Jpeg),
            Some(85)
        );
        assert_eq!(
            ImageQualityPreset::Lossless.magick_quality_for(ImageOutputFormat::Webp),
            None
        );
        assert_eq!(
            ImageQualityPreset::Medium.magick_quality_for(ImageOutputFormat::Webp),
            Some(85)
        );
        // PNG: higher preset means more compression effort (lower Magick number).
        assert_eq!(
            ImageQualityPreset::Low.magick_quality_for(ImageOutputFormat::Png),
            Some(90)
        );
        assert_eq!(
            ImageQualityPreset::High.magick_quality_for(ImageOutputFormat::Png),
            Some(50)
        );
        assert_eq!(
            ImageQualityPreset::Medium.magick_quality_for(ImageOutputFormat::Tiff),
            None
        );
        assert_eq!(
            ImageQualityPreset::Medium.magick_quality_for(ImageOutputFormat::Bmp),
            None
        );
        assert_eq!(
            ImageQualityPreset::Medium.magick_quality_for(ImageOutputFormat::Gif),
            None
        );
        assert_eq!(
            ImageQualityPreset::High.magick_quality_for(ImageOutputFormat::Avif),
            Some(95)
        );
    }

    #[test]
    fn resize_preset_edges() {
        assert_eq!(ImageResizePreset::Original.max_long_edge(), None);
        assert_eq!(ImageResizePreset::Max2048.max_long_edge(), Some(2048));
        assert_eq!(ImageResizePreset::Max1920.max_long_edge(), Some(1920));
        assert_eq!(ImageResizePreset::Max1280.max_long_edge(), Some(1280));
        assert_eq!(ImageResizePreset::Max1024.max_long_edge(), Some(1024));
    }

    #[test]
    fn magick_geometry_shrinks_only() {
        assert_eq!(ImageResizePreset::Original.magick_geometry(), None);
        assert_eq!(
            ImageResizePreset::Max1920.magick_geometry(),
            Some("1920x1920>".to_string())
        );
        assert_eq!(
            ImageResizePreset::Max1024.magick_geometry(),
            Some("1024x1024>".to_string())
        );
    }

    #[test]
    fn resize_preset_deserializes_from_numeric_strings() {
        let parsed: ImageResizePreset = serde_json::from_str("\"1920\"").expect("parse");
        assert_eq!(parsed, ImageResizePreset::Max1920);
        let parsed: ImageResizePreset = serde_json::from_str("\"original\"").expect("parse");
        assert_eq!(parsed, ImageResizePreset::Original);
    }

    #[test]
    fn job_defaults_are_sane() {
        let job: ImageConversionJob = serde_json::from_value(serde_json::json!({
            "id": "1",
            "sourcePath": "a.png",
            "destinationDir": "out",
            "outputFormat": "webp",
            "status": "idle"
        }))
        .expect("job parses");
        assert_eq!(job.quality_preset, ImageQualityPreset::Medium);
        assert_eq!(job.resize_preset, ImageResizePreset::Original);
        assert_eq!(job.overwrite_policy, OverwritePolicy::Rename);
        assert!(job.preserve_metadata);
        assert_eq!(job.relative_subdir, None);
    }
}
