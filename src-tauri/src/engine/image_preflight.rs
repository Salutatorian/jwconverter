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

        if source_is_lossy(item.format.as_deref()) && !request.output_format.is_lossy() {
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
                "{lossy_to_lossless} image(s): converting a lossy photo to PNG/TIFF won't restore discarded detail — output is often larger."
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
    match format {
        ImageOutputFormat::Jpeg => {
            let bpp = match quality {
                ImageQualityPreset::Low => 0.12,
                ImageQualityPreset::Medium => 0.20,
                ImageQualityPreset::High => 0.35,
            };
            (pixels as f64 * bpp) as u64
        }
        ImageOutputFormat::Webp => {
            let bpp = match quality {
                ImageQualityPreset::Low => 0.08,
                ImageQualityPreset::Medium => 0.14,
                ImageQualityPreset::High => 0.25,
            };
            (pixels as f64 * bpp) as u64
        }
        ImageOutputFormat::Png => pixels.saturating_mul(2),
        ImageOutputFormat::Tiff => pixels.saturating_mul(3),
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
}
