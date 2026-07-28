//! Image batch preflight: honesty warnings, size estimate, disk gate.

use std::path::{Path, PathBuf};

use crate::engine::image_job::{ImageOutputFormat, ImageQualityPreset, ImageResizePreset};
use crate::engine::job::OverwritePolicy;
use crate::engine::preflight::{self, PreflightReport, PreflightWarning, WarningKind};
use crate::errors::AppError;
use crate::fs_safety::finalize;

#[derive(Debug, Clone)]
pub struct ImagePreflightItem {
    pub source_path: String,
    pub relative_subdir: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub file_size_bytes: Option<u64>,
    pub format: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ImagePreflightRequest {
    pub destination_dir: String,
    pub output_format: ImageOutputFormat,
    pub quality_preset: ImageQualityPreset,
    pub resize_preset: ImageResizePreset,
    pub overwrite_policy: OverwritePolicy,
    pub items: Vec<ImagePreflightItem>,
}

pub fn run_image_preflight(request: &ImagePreflightRequest) -> Result<PreflightReport, AppError> {
    let destination_root = PathBuf::from(request.destination_dir.trim());
    if request.destination_dir.trim().is_empty() {
        return Err(AppError::DestinationUnavailable {
            detail: "No destination folder was provided.".to_string(),
        });
    }

    let free_bytes = preflight::free_space_bytes(&destination_root).ok();
    let extension = request.output_format.extension();

    let mut file_count = 0u32;
    let mut skipped_existing = 0u32;
    let mut source_bytes = 0u64;
    let mut estimated_output_bytes = 0u64;
    let mut lossy_to_lossless = 0u32;

    for item in &request.items {
        let source = Path::new(&item.source_path);
        let stem = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let dest_dir =
            resolve_dest_dir_best_effort(&destination_root, item.relative_subdir.as_deref());
        let primary = finalize::primary_final_path(&dest_dir, stem, extension);

        if request.overwrite_policy == OverwritePolicy::Skip && primary.exists() {
            skipped_existing += 1;
            continue;
        }

        file_count += 1;
        if let Some(size) = item.file_size_bytes {
            source_bytes = source_bytes.saturating_add(size);
        }

        if source_is_lossy(item.format.as_deref())
            && !request
                .output_format
                .is_lossy_with(request.quality_preset)
        {
            lossy_to_lossless += 1;
        }

        if let (Some(w), Some(h)) = (item.width, item.height) {
            let (tw, th) = target_dimensions(w, h, request.resize_preset);
            estimated_output_bytes = estimated_output_bytes.saturating_add(estimate_output_bytes(
                tw,
                th,
                request.output_format,
                request.quality_preset,
            ));
        } else if let Some(size) = item.file_size_bytes {
            // Fallback when dimensions missing.
            estimated_output_bytes = estimated_output_bytes.saturating_add(size);
        }
    }

    let margin = preflight::disk_margin(estimated_output_bytes);
    let required_bytes = estimated_output_bytes.saturating_add(margin);
    let disk_blocked = match free_bytes {
        Some(free) => estimated_output_bytes > 0 && required_bytes > free,
        None => false,
    };

    let mut warnings = Vec::new();
    if lossy_to_lossless > 0 {
        warnings.push(PreflightWarning {
            kind: WarningKind::LossyToLossless,
            count: lossy_to_lossless,
            message: format!(
                "{lossy_to_lossless} image(s): converting a lossy photo to a lossless format (PNG/TIFF/BMP/WebP lossless) won't restore discarded detail — output is often larger."
            ),
        });
    }

    Ok(PreflightReport {
        file_count,
        skipped_existing,
        source_bytes,
        estimated_output_bytes,
        free_bytes,
        required_bytes,
        disk_blocked,
        warnings,
    })
}

pub fn target_dimensions(width: u32, height: u32, resize: ImageResizePreset) -> (u32, u32) {
    let Some(max_edge) = resize.max_long_edge() else {
        return (width, height);
    };
    let long = width.max(height);
    if long <= max_edge || long == 0 {
        return (width, height);
    }
    let scale = f64::from(max_edge) / f64::from(long);
    let tw = ((f64::from(width) * scale).round() as u32).max(1);
    let th = ((f64::from(height) * scale).round() as u32).max(1);
    (tw, th)
}

fn estimate_output_bytes(
    width: u32,
    height: u32,
    format: ImageOutputFormat,
    quality: ImageQualityPreset,
) -> u64 {
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    let quality = quality.normalize_for(format);
    match format {
        ImageOutputFormat::Jpeg => {
            let bpp = match quality {
                ImageQualityPreset::Low => 0.12,
                ImageQualityPreset::Medium => 0.20,
                ImageQualityPreset::High | ImageQualityPreset::Lossless => 0.35,
            };
            (pixels as f64 * bpp) as u64
        }
        ImageOutputFormat::Webp => {
            if quality.is_lossless() {
                return pixels.saturating_mul(2);
            }
            let bpp = match quality {
                ImageQualityPreset::Low => 0.08,
                ImageQualityPreset::Medium => 0.14,
                ImageQualityPreset::High | ImageQualityPreset::Lossless => 0.25,
            };
            (pixels as f64 * bpp) as u64
        }
        ImageOutputFormat::Png => {
            let bpp = match quality {
                ImageQualityPreset::Low => 2.4,
                ImageQualityPreset::Medium => 2.0,
                ImageQualityPreset::High | ImageQualityPreset::Lossless => 1.6,
            };
            (pixels as f64 * bpp) as u64
        }
        ImageOutputFormat::Tiff => pixels.saturating_mul(3),
        ImageOutputFormat::Bmp => pixels.saturating_mul(3),
        ImageOutputFormat::Gif => (pixels as f64 * 0.5) as u64,
        ImageOutputFormat::Avif => {
            let bpp = match quality {
                ImageQualityPreset::Low => 0.06,
                ImageQualityPreset::Medium => 0.10,
                ImageQualityPreset::High | ImageQualityPreset::Lossless => 0.18,
            };
            (pixels as f64 * bpp) as u64
        }
    }
}

fn source_is_lossy(format: Option<&str>) -> bool {
    let f = format.unwrap_or("").to_ascii_uppercase();
    matches!(
        f.as_str(),
        "JPEG" | "JPG" | "WEBP" | "HEIC" | "HEIF" | "AVIF" | "JP2" | "JXL"
    )
}

fn resolve_dest_dir_best_effort(root: &Path, relative: Option<&str>) -> PathBuf {
    let Some(relative) = relative.map(str::trim).filter(|s| !s.is_empty()) else {
        return root.to_path_buf();
    };
    let mut dir = root.to_path_buf();
    for part in relative.split(['/', '\\']) {
        if part.is_empty() || part == "." || part == ".." || part.contains(':') {
            continue;
        }
        dir.push(part);
    }
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_only_shrinks() {
        assert_eq!(
            target_dimensions(4000, 3000, ImageResizePreset::Max1920),
            (1920, 1440)
        );
        assert_eq!(
            target_dimensions(800, 600, ImageResizePreset::Max1920),
            (800, 600)
        );
        assert_eq!(
            target_dimensions(4000, 3000, ImageResizePreset::Original),
            (4000, 3000)
        );
    }

    #[test]
    fn lossy_to_png_warns() {
        let report = run_image_preflight(&ImagePreflightRequest {
            destination_dir: std::env::temp_dir().to_string_lossy().into_owned(),
            output_format: ImageOutputFormat::Png,
            quality_preset: ImageQualityPreset::Medium,
            resize_preset: ImageResizePreset::Original,
            overwrite_policy: OverwritePolicy::Rename,
            items: vec![ImagePreflightItem {
                source_path: r"C:\photos\a.jpg".into(),
                relative_subdir: None,
                width: Some(2000),
                height: Some(1500),
                file_size_bytes: Some(400_000),
                format: Some("JPEG".into()),
            }],
        })
        .expect("ok");
        assert!(report
            .warnings
            .iter()
            .any(|w| w.kind == WarningKind::LossyToLossless));
        assert!(report.estimated_output_bytes > 0);
    }

    #[test]
    fn lossy_to_webp_lossless_warns() {
        let report = run_image_preflight(&ImagePreflightRequest {
            destination_dir: std::env::temp_dir().to_string_lossy().into_owned(),
            output_format: ImageOutputFormat::Webp,
            quality_preset: ImageQualityPreset::Lossless,
            resize_preset: ImageResizePreset::Original,
            overwrite_policy: OverwritePolicy::Rename,
            items: vec![ImagePreflightItem {
                source_path: r"C:\photos\a.jpg".into(),
                relative_subdir: None,
                width: Some(2000),
                height: Some(1500),
                file_size_bytes: Some(400_000),
                format: Some("JPEG".into()),
            }],
        })
        .expect("ok");
        assert!(report
            .warnings
            .iter()
            .any(|w| w.kind == WarningKind::LossyToLossless));
    }

    fn item_jpeg() -> ImagePreflightItem {
        ImagePreflightItem {
            source_path: r"C:\photos\a.jpg".into(),
            relative_subdir: None,
            width: Some(2000),
            height: Some(1500),
            file_size_bytes: Some(400_000),
            format: Some("JPEG".into()),
        }
    }

    fn run_basic(
        format: ImageOutputFormat,
        quality: ImageQualityPreset,
        policy: OverwritePolicy,
        items: Vec<ImagePreflightItem>,
    ) -> PreflightReport {
        run_image_preflight(&ImagePreflightRequest {
            destination_dir: std::env::temp_dir().to_string_lossy().into_owned(),
            output_format: format,
            quality_preset: quality,
            resize_preset: ImageResizePreset::Original,
            overwrite_policy: policy,
            items,
        })
        .expect("preflight")
    }

    #[test]
    fn lossy_to_lossy_has_no_warning() {
        let report = run_basic(
            ImageOutputFormat::Jpeg,
            ImageQualityPreset::Medium,
            OverwritePolicy::Rename,
            vec![item_jpeg()],
        );
        assert!(report.warnings.is_empty());
        assert_eq!(report.file_count, 1);
    }

    #[test]
    fn lossless_source_to_lossless_target_no_warning() {
        let mut item = item_jpeg();
        item.format = Some("PNG".into());
        let report = run_basic(
            ImageOutputFormat::Tiff,
            ImageQualityPreset::Medium,
            OverwritePolicy::Rename,
            vec![item],
        );
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn empty_destination_is_rejected() {
        let result = run_image_preflight(&ImagePreflightRequest {
            destination_dir: "   ".into(),
            output_format: ImageOutputFormat::Jpeg,
            quality_preset: ImageQualityPreset::Medium,
            resize_preset: ImageResizePreset::Original,
            overwrite_policy: OverwritePolicy::Rename,
            items: vec![item_jpeg()],
        });
        assert!(result.is_err());
    }

    #[test]
    fn missing_dimensions_fall_back_to_source_size() {
        let mut item = item_jpeg();
        item.width = None;
        item.height = None;
        let report = run_basic(
            ImageOutputFormat::Jpeg,
            ImageQualityPreset::Medium,
            OverwritePolicy::Rename,
            vec![item],
        );
        assert_eq!(report.estimated_output_bytes, 400_000);
    }

    #[test]
    fn resize_reduces_estimate() {
        let small = run_image_preflight(&ImagePreflightRequest {
            destination_dir: std::env::temp_dir().to_string_lossy().into_owned(),
            output_format: ImageOutputFormat::Jpeg,
            quality_preset: ImageQualityPreset::Medium,
            resize_preset: ImageResizePreset::Max1024,
            overwrite_policy: OverwritePolicy::Rename,
            items: vec![item_jpeg()],
        })
        .expect("preflight");
        let original = run_basic(
            ImageOutputFormat::Jpeg,
            ImageQualityPreset::Medium,
            OverwritePolicy::Rename,
            vec![item_jpeg()],
        );
        assert!(small.estimated_output_bytes < original.estimated_output_bytes);
    }

    #[test]
    fn target_dimensions_never_upscale() {
        assert_eq!(
            target_dimensions(640, 480, ImageResizePreset::Max2048),
            (640, 480)
        );
    }

    #[test]
    fn target_dimensions_portrait_uses_height_as_long_edge() {
        assert_eq!(
            target_dimensions(3000, 4000, ImageResizePreset::Max1920),
            (1440, 1920)
        );
    }

    #[test]
    fn target_dimensions_zero_long_edge_is_safe() {
        assert_eq!(target_dimensions(0, 0, ImageResizePreset::Max1024), (0, 0));
    }

    #[test]
    fn target_dimensions_tiny_source_clamps_to_one_pixel() {
        let (w, h) = target_dimensions(2, 1, ImageResizePreset::Max1024);
        assert_eq!((w, h), (2, 1));
        let (w, h) = target_dimensions(5000, 1, ImageResizePreset::Max1024);
        assert_eq!(w, 1024);
        assert!(h >= 1);
    }

    #[test]
    fn estimate_matrix_monotonic_in_quality() {
        let pixels = |(w, h): (u32, u32)| u64::from(w) * u64::from(h);
        let dims = (2000, 1500);
        for format in [
            ImageOutputFormat::Jpeg,
            ImageOutputFormat::Webp,
            ImageOutputFormat::Avif,
        ] {
            let low = estimate_output_bytes(dims.0, dims.1, format, ImageQualityPreset::Low);
            let high = estimate_output_bytes(dims.0, dims.1, format, ImageQualityPreset::High);
            assert!(high > low, "{format:?}");
        }
        let tiff = estimate_output_bytes(2000, 1500, ImageOutputFormat::Tiff, ImageQualityPreset::Medium);
        assert_eq!(tiff, pixels(dims) * 3);
        let webp_lossless =
            estimate_output_bytes(2000, 1500, ImageOutputFormat::Webp, ImageQualityPreset::Lossless);
        assert_eq!(webp_lossless, pixels(dims) * 2);
        let gif = estimate_output_bytes(2000, 1500, ImageOutputFormat::Gif, ImageQualityPreset::Medium);
        assert_eq!(gif, (pixels(dims) as f64 * 0.5) as u64);
    }

    #[test]
    fn source_is_lossy_recognizes_lossy_containers() {
        for lossy in ["JPEG", "JPG", "WEBP", "HEIC", "HEIF", "AVIF", "JP2", "JXL"] {
            assert!(source_is_lossy(Some(lossy)), "{lossy}");
        }
        for lossless in ["PNG", "TIFF", "BMP", "GIF", ""] {
            assert!(!source_is_lossy(Some(lossless)), "{lossless}");
        }
        assert!(!source_is_lossy(None));
    }

    #[test]
    fn dest_dir_resolution_strips_traversal_and_drives() {
        let root = PathBuf::from(r"D:\out");
        // `..` segments are dropped, the rest of the path is kept.
        assert_eq!(
            resolve_dest_dir_best_effort(&root, Some(r"..\..\evil")),
            root.join("evil")
        );
        // Drive prefix segment is dropped, folder segment kept.
        assert_eq!(
            resolve_dest_dir_best_effort(&root, Some(r"C:\abs")),
            root.join("abs")
        );
        assert_eq!(
            resolve_dest_dir_best_effort(&root, Some(r"album/sub")),
            root.join("album").join("sub")
        );
        assert_eq!(resolve_dest_dir_best_effort(&root, None), root);
        assert_eq!(resolve_dest_dir_best_effort(&root, Some("  ")), root);
    }

    #[test]
    fn skip_policy_counts_existing_outputs() {
        let dir = std::env::temp_dir().join(format!("jw-img-preflight-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("tmpdir");
        std::fs::write(dir.join("a.jpg"), b"existing").expect("write existing");

        let report = run_image_preflight(&ImagePreflightRequest {
            destination_dir: dir.to_string_lossy().into_owned(),
            output_format: ImageOutputFormat::Jpeg,
            quality_preset: ImageQualityPreset::Medium,
            resize_preset: ImageResizePreset::Original,
            overwrite_policy: OverwritePolicy::Skip,
            items: vec![ImagePreflightItem {
                source_path: dir.join("a.png").to_string_lossy().into_owned(),
                relative_subdir: None,
                width: Some(100),
                height: Some(100),
                file_size_bytes: Some(10_000),
                format: Some("PNG".into()),
            }],
        })
        .expect("preflight");

        assert_eq!(report.file_count, 0);
        assert_eq!(report.skipped_existing, 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn source_bytes_accumulate_with_saturation() {
        let mut big = item_jpeg();
        big.file_size_bytes = Some(u64::MAX);
        let report = run_basic(
            ImageOutputFormat::Jpeg,
            ImageQualityPreset::Medium,
            OverwritePolicy::Rename,
            vec![big, item_jpeg()],
        );
        assert_eq!(report.source_bytes, u64::MAX);
        assert_eq!(report.file_count, 2);
    }
}
