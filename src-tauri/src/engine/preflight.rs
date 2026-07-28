//! Batch preflight: quality honesty warnings, size estimate, disk space gate.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::engine::job::{
    BitDepthPreset, Mp3EncodingMode, OutputFormat, OverwritePolicy, QualityPreset,
};
use crate::engine::planner::{self, EncoderPlan, SourcePcmHints};
use crate::errors::AppError;
use crate::fs_safety::finalize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WarningKind {
    LossyToLossless,
    BitDepthUpsample,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightWarning {
    pub kind: WarningKind,
    pub count: u32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightReport {
    pub file_count: u32,
    pub skipped_existing: u32,
    pub source_bytes: u64,
    pub estimated_output_bytes: u64,
    pub free_bytes: Option<u64>,
    pub required_bytes: u64,
    pub disk_blocked: bool,
    pub warnings: Vec<PreflightWarning>,
}

#[derive(Debug, Clone)]
pub struct PreflightItem {
    pub source_path: String,
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

#[derive(Debug, Clone)]
pub struct PreflightRequest {
    pub destination_dir: String,
    pub output_format: OutputFormat,
    pub quality_preset: QualityPreset,
    pub mp3_encoding_mode: Mp3EncodingMode,
    pub bit_depth_preset: BitDepthPreset,
    pub overwrite_policy: OverwritePolicy,
    pub items: Vec<PreflightItem>,
}

const MARGIN_FLOOR: u64 = 500 * 1024 * 1024;

pub fn run_preflight(request: &PreflightRequest) -> Result<PreflightReport, AppError> {
    let destination_root = PathBuf::from(request.destination_dir.trim());
    if request.destination_dir.trim().is_empty() {
        return Err(AppError::DestinationUnavailable {
            detail: "No destination folder was provided.".to_string(),
        });
    }

    let free_bytes = free_space_bytes(&destination_root).ok();

    let mut file_count = 0u32;
    let mut skipped_existing = 0u32;
    let mut source_bytes = 0u64;
    let mut estimated_output_bytes = 0u64;
    let mut lossy_to_lossless = 0u32;
    let mut bit_depth_upsample = 0u32;

    for item in &request.items {
        let hints = SourcePcmHints {
            sample_format: item.sample_format.clone(),
            bits_per_raw_sample: item.bits_per_raw_sample,
            bit_depth: item.bit_depth,
        };
        let plan = planner::plan_for(
            request.output_format,
            request.quality_preset,
            request.bit_depth_preset,
            Some(&hints),
            request.mp3_encoding_mode,
        );

        let source = Path::new(&item.source_path);
        let stem = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");

        let dest_dir =
            resolve_dest_dir_best_effort(&destination_root, item.relative_subdir.as_deref());
        let primary = finalize::primary_final_path(&dest_dir, stem, plan.extension);

        if request.overwrite_policy == OverwritePolicy::Skip && primary.exists() {
            skipped_existing += 1;
            continue;
        }

        file_count += 1;
        if let Some(size) = item.file_size_bytes {
            source_bytes = source_bytes.saturating_add(size);
        }

        if source_is_lossy(item.codec.as_deref(), item.format.as_deref())
            && !request.output_format.is_lossy()
        {
            lossy_to_lossless += 1;
        }

        if is_bit_depth_upsample(request.bit_depth_preset, request.output_format, &hints) {
            bit_depth_upsample += 1;
        }

        if let Some(est) = estimate_output_bytes(
            &plan,
            item.duration_seconds,
            item.sample_rate,
            item.channels,
            &hints,
        ) {
            estimated_output_bytes = estimated_output_bytes.saturating_add(est);
        }
    }

    let margin = disk_margin(estimated_output_bytes);
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
                "{lossy_to_lossless} file(s): converting lossy audio to a lossless/PCM format won't restore discarded detail — output will usually be larger."
            ),
        });
    }
    if bit_depth_upsample > 0 {
        warnings.push(PreflightWarning {
            kind: WarningKind::BitDepthUpsample,
            count: bit_depth_upsample,
            message: format!(
                "{bit_depth_upsample} file(s): forcing a higher bit depth won't add information that wasn't in the source."
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

pub fn disk_margin(estimated_output_bytes: u64) -> u64 {
    let five_percent = estimated_output_bytes / 20;
    five_percent.max(MARGIN_FLOOR)
}

fn resolve_dest_dir_best_effort(root: &Path, relative: Option<&str>) -> PathBuf {
    let Some(relative) = relative.map(str::trim).filter(|s| !s.is_empty()) else {
        return root.to_path_buf();
    };
    let mut dir = root.to_path_buf();
    for part in relative.split(['/', '\\']) {
        if part.is_empty() || part == "." || part == ".." {
            continue;
        }
        if part.contains(':') {
            return root.to_path_buf();
        }
        dir.push(part);
    }
    dir
}

fn source_is_lossy(codec: Option<&str>, format: Option<&str>) -> bool {
    let codec = codec.unwrap_or("").to_ascii_lowercase();
    let format = format.unwrap_or("").to_ascii_lowercase();

    const LOSSY_CODECS: &[&str] = &[
        "mp3",
        "mp3float",
        "mp2",
        "mp1",
        "aac",
        "aac_latm",
        "opus",
        "vorbis",
        "wmav1",
        "wmav2",
        "wmapro",
        "ac3",
        "eac3",
        "dts",
        "libopus",
        "libmp3lame",
        "libvorbis",
    ];
    if LOSSY_CODECS.iter().any(|c| codec == *c) {
        return true;
    }

    format.split(',').any(|part| {
        let p = part.trim();
        p == "mp3"
            || p == "mp2"
            || p == "aac"
            || p == "m4a" && codec.contains("aac")
            || p == "ogg" && (codec.contains("vorbis") || codec.contains("opus"))
            || p == "opus"
            || p == "wma"
    })
}

fn is_bit_depth_upsample(
    preset: BitDepthPreset,
    format: OutputFormat,
    hints: &SourcePcmHints,
) -> bool {
    if !matches!(format, OutputFormat::Wav | OutputFormat::Aiff) {
        return false;
    }
    let Some(target) = forced_target_bits(preset) else {
        return false;
    };
    let Some(source) = hints.bits_per_raw_sample.or(hints.bit_depth) else {
        return false;
    };
    target > source
}

fn forced_target_bits(preset: BitDepthPreset) -> Option<u32> {
    match preset {
        BitDepthPreset::Original => None,
        BitDepthPreset::Bit16 => Some(16),
        BitDepthPreset::Bit24 => Some(24),
        BitDepthPreset::Float32 => Some(32),
    }
}

fn estimate_output_bytes(
    plan: &EncoderPlan,
    duration_seconds: Option<f64>,
    sample_rate: Option<u32>,
    channels: Option<u32>,
    hints: &SourcePcmHints,
) -> Option<u64> {
    let duration = duration_seconds.filter(|d| *d > 0.0)?;
    let rate = sample_rate.filter(|r| *r > 0).unwrap_or(44100);
    let ch = channels.filter(|c| *c > 0).unwrap_or(2);

    match plan.format {
        OutputFormat::Wav | OutputFormat::Aiff => {
            let bytes_per_sample = pcm_bytes_per_sample(plan.bit_depth, hints);
            let raw = duration * f64::from(rate) * f64::from(ch) * f64::from(bytes_per_sample);
            Some(raw as u64 + 44)
        }
        OutputFormat::Flac => {
            let bytes_per_sample = pcm_bytes_per_sample(BitDepthPreset::Original, hints);
            let pcm = duration * f64::from(rate) * f64::from(ch) * f64::from(bytes_per_sample);
            Some((pcm * 0.55) as u64)
        }
        OutputFormat::Alac => {
            let bytes_per_sample = pcm_bytes_per_sample(BitDepthPreset::Original, hints);
            let pcm = duration * f64::from(rate) * f64::from(ch) * f64::from(bytes_per_sample);
            Some((pcm * 0.60) as u64)
        }
        OutputFormat::Mp3 | OutputFormat::M4a | OutputFormat::Aac | OutputFormat::Opus | OutputFormat::Ogg => {
            let bps = lossy_bitrate_bps(plan.format, plan.quality, plan.mp3_encoding_mode)?;
            Some(((duration * f64::from(bps)) / 8.0) as u64)
        }
    }
}

fn pcm_bytes_per_sample(bit_depth: BitDepthPreset, hints: &SourcePcmHints) -> u32 {
    match bit_depth {
        BitDepthPreset::Bit16 => 2,
        BitDepthPreset::Bit24 => 3,
        BitDepthPreset::Float32 => 4,
        BitDepthPreset::Original => match hints.bits_per_raw_sample.or(hints.bit_depth) {
            Some(0..=8) => 1,
            Some(9..=16) => 2,
            Some(17..=24) => 3,
            Some(_) => 4,
            None => {
                let fmt = hints
                    .sample_format
                    .as_deref()
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if fmt.contains("f64") {
                    8
                } else if fmt.contains("f32") || fmt.contains("flt") || fmt.contains("s32") {
                    4
                } else if fmt.contains("s24") {
                    3
                } else if fmt.contains("u8") {
                    1
                } else {
                    3 // matches planner default preference for unknown masters
                }
            }
        },
    }
}

fn lossy_bitrate_bps(
    format: OutputFormat,
    quality: QualityPreset,
    mp3_mode: Mp3EncodingMode,
) -> Option<u32> {
    let kbps = match (format, quality, mp3_mode) {
        (OutputFormat::Mp3, QualityPreset::Low, Mp3EncodingMode::Cbr) => 128,
        (OutputFormat::Mp3, QualityPreset::Medium, Mp3EncodingMode::Cbr) => 192,
        (OutputFormat::Mp3, QualityPreset::High, Mp3EncodingMode::Cbr) => 320,
        // Approximate average bitrates for lame V5 / V2 / V0.
        (OutputFormat::Mp3, QualityPreset::Low, Mp3EncodingMode::Vbr) => 130,
        (OutputFormat::Mp3, QualityPreset::Medium, Mp3EncodingMode::Vbr) => 190,
        (OutputFormat::Mp3, QualityPreset::High, Mp3EncodingMode::Vbr) => 245,
        (OutputFormat::M4a | OutputFormat::Aac, QualityPreset::Low, _) => 128,
        (OutputFormat::M4a | OutputFormat::Aac, QualityPreset::Medium, _) => 192,
        (OutputFormat::M4a | OutputFormat::Aac, QualityPreset::High, _) => 256,
        (OutputFormat::Opus, QualityPreset::Low, _) => 96,
        (OutputFormat::Opus, QualityPreset::Medium, _) => 160,
        (OutputFormat::Opus, QualityPreset::High, _) => 192,
        (OutputFormat::Ogg, QualityPreset::Low, _) => 112,
        (OutputFormat::Ogg, QualityPreset::Medium, _) => 160,
        (OutputFormat::Ogg, QualityPreset::High, _) => 224,
        _ => return None,
    };
    Some(kbps * 1000)
}

pub fn free_space_bytes(path: &Path) -> Result<u64, AppError> {
    #[cfg(windows)]
    {
        free_space_bytes_windows(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err(AppError::DestinationUnavailable {
            detail: "Free-space check is only implemented on Windows.".to_string(),
        })
    }
}

#[cfg(windows)]
fn free_space_bytes_windows(path: &Path) -> Result<u64, AppError> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetDiskFreeSpaceExW(
            lp_directory_name: *const u16,
            lp_free_bytes_available_to_caller: *mut u64,
            lp_total_number_of_bytes: *mut u64,
            lp_total_number_of_free_bytes: *mut u64,
        ) -> i32;
    }

    let mut candidate = path.to_path_buf();
    loop {
        if let Some(free) = query_free(&candidate, GetDiskFreeSpaceExW) {
            return Ok(free);
        }
        if !candidate.pop() {
            break;
        }
    }

    // Last resort: drive root like `C:\`
    if let Some(root) = path.components().next() {
        let root_path = PathBuf::from(root.as_os_str());
        if let Some(free) = query_free(&root_path, GetDiskFreeSpaceExW) {
            return Ok(free);
        }
    }

    Err(AppError::DestinationUnavailable {
        detail: format!("Could not read free space for {}.", path.display()),
    })
}

#[cfg(windows)]
fn query_free(
    path: &Path,
    get_disk_free_space_ex_w: unsafe extern "system" fn(
        *const u16,
        *mut u64,
        *mut u64,
        *mut u64,
    ) -> i32,
) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut available: u64 = 0;
    let mut total: u64 = 0;
    let mut total_free: u64 = 0;
    let ok = unsafe {
        get_disk_free_space_ex_w(
            wide.as_ptr(),
            &mut available,
            &mut total,
            &mut total_free,
        )
    };
    if ok != 0 {
        Some(available)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item_mp3() -> PreflightItem {
        PreflightItem {
            source_path: r"C:\music\track.mp3".into(),
            relative_subdir: None,
            duration_seconds: Some(180.0),
            sample_rate: Some(44100),
            channels: Some(2),
            file_size_bytes: Some(4_200_000),
            codec: Some("mp3".into()),
            format: Some("mp3".into()),
            bit_depth: None,
            bits_per_raw_sample: None,
            sample_format: None,
        }
    }

    #[test]
    fn lossy_to_flac_warns() {
        let report = run_preflight(&PreflightRequest {
            destination_dir: std::env::temp_dir().to_string_lossy().into_owned(),
            output_format: OutputFormat::Flac,
            quality_preset: QualityPreset::Medium,
            mp3_encoding_mode: Mp3EncodingMode::Cbr,
            bit_depth_preset: BitDepthPreset::Original,
            overwrite_policy: OverwritePolicy::Rename,
            items: vec![item_mp3()],
        })
        .expect("preflight");

        assert_eq!(report.file_count, 1);
        assert!(report
            .warnings
            .iter()
            .any(|w| w.kind == WarningKind::LossyToLossless));
        assert!(report.estimated_output_bytes > report.source_bytes);
    }

    #[test]
    fn bit_depth_upsample_warns() {
        let mut item = item_mp3();
        item.codec = Some("pcm_s16le".into());
        item.format = Some("wav".into());
        item.bit_depth = Some(16);
        item.bits_per_raw_sample = Some(16);
        item.sample_format = Some("s16".into());
        item.source_path = r"C:\music\track.wav".into();

        let report = run_preflight(&PreflightRequest {
            destination_dir: std::env::temp_dir().to_string_lossy().into_owned(),
            output_format: OutputFormat::Wav,
            quality_preset: QualityPreset::Medium,
            mp3_encoding_mode: Mp3EncodingMode::Cbr,
            bit_depth_preset: BitDepthPreset::Bit24,
            overwrite_policy: OverwritePolicy::Rename,
            items: vec![item],
        })
        .expect("preflight");

        assert!(report
            .warnings
            .iter()
            .any(|w| w.kind == WarningKind::BitDepthUpsample));
    }

    #[test]
    fn original_bit_depth_no_upsample_warning() {
        let mut item = item_mp3();
        item.codec = Some("pcm_s16le".into());
        item.format = Some("wav".into());
        item.bit_depth = Some(16);
        item.source_path = r"C:\music\track.wav".into();

        let report = run_preflight(&PreflightRequest {
            destination_dir: std::env::temp_dir().to_string_lossy().into_owned(),
            output_format: OutputFormat::Wav,
            quality_preset: QualityPreset::Medium,
            mp3_encoding_mode: Mp3EncodingMode::Cbr,
            bit_depth_preset: BitDepthPreset::Original,
            overwrite_policy: OverwritePolicy::Rename,
            items: vec![item],
        })
        .expect("preflight");

        assert!(report
            .warnings
            .iter()
            .all(|w| w.kind != WarningKind::BitDepthUpsample));
    }

    #[test]
    fn disk_margin_is_at_least_500mb() {
        assert_eq!(disk_margin(0), MARGIN_FLOOR);
        assert_eq!(disk_margin(1000), MARGIN_FLOOR);
        assert_eq!(disk_margin(20 * MARGIN_FLOOR), MARGIN_FLOOR);
        assert!(disk_margin(40 * MARGIN_FLOOR) > MARGIN_FLOOR);
    }

    #[test]
    fn mp3_to_mp3_no_lossy_warning() {
        let report = run_preflight(&PreflightRequest {
            destination_dir: std::env::temp_dir().to_string_lossy().into_owned(),
            output_format: OutputFormat::Mp3,
            quality_preset: QualityPreset::Medium,
            mp3_encoding_mode: Mp3EncodingMode::Cbr,
            bit_depth_preset: BitDepthPreset::Original,
            overwrite_policy: OverwritePolicy::Rename,
            items: vec![item_mp3()],
        })
        .expect("preflight");

        assert!(report
            .warnings
            .iter()
            .all(|w| w.kind != WarningKind::LossyToLossless));
    }

    fn hints(bit_depth: Option<u32>, sample_format: Option<&str>) -> SourcePcmHints {
        SourcePcmHints {
            sample_format: sample_format.map(str::to_string),
            bits_per_raw_sample: bit_depth,
            bit_depth,
        }
    }

    fn wav_plan(bit_depth: BitDepthPreset) -> EncoderPlan {
        planner::plan_for(
            OutputFormat::Wav,
            QualityPreset::Medium,
            bit_depth,
            None,
            Mp3EncodingMode::Cbr,
        )
    }

    #[test]
    fn estimate_wav_pcm_is_exact() {
        let plan = wav_plan(BitDepthPreset::Bit16);
        let no_hints = hints(None, None);
        // 10s * 44100 * 2ch * 2 bytes + 44 header.
        let est = estimate_output_bytes(&plan, Some(10.0), Some(44100), Some(2), &no_hints)
            .expect("estimate");
        assert_eq!(est, 10 * 44100 * 2 * 2 + 44);
    }

    #[test]
    fn estimate_defaults_to_cd_audio_when_stream_info_missing() {
        let plan = wav_plan(BitDepthPreset::Bit16);
        let no_hints = hints(None, None);
        let est = estimate_output_bytes(&plan, Some(1.0), None, None, &no_hints).expect("estimate");
        assert_eq!(est, 44100 * 2 * 2 + 44);
    }

    #[test]
    fn estimate_none_without_duration() {
        let plan = wav_plan(BitDepthPreset::Bit16);
        let no_hints = hints(None, None);
        assert!(estimate_output_bytes(&plan, None, Some(44100), Some(2), &no_hints).is_none());
        assert!(estimate_output_bytes(&plan, Some(0.0), Some(44100), Some(2), &no_hints).is_none());
        assert!(estimate_output_bytes(&plan, Some(-1.0), Some(44100), Some(2), &no_hints).is_none());
    }

    #[test]
    fn estimate_flac_and_alac_scale_from_pcm() {
        let no_hints = hints(Some(16), None);
        let flac = planner::plan_for(
            OutputFormat::Flac,
            QualityPreset::Medium,
            BitDepthPreset::Original,
            None,
            Mp3EncodingMode::Cbr,
        );
        let alac = planner::plan_for(
            OutputFormat::Alac,
            QualityPreset::Medium,
            BitDepthPreset::Original,
            None,
            Mp3EncodingMode::Cbr,
        );
        let pcm = 10.0 * 44100.0 * 2.0 * 2.0;
        let flac_est =
            estimate_output_bytes(&flac, Some(10.0), Some(44100), Some(2), &no_hints).expect("flac");
        let alac_est =
            estimate_output_bytes(&alac, Some(10.0), Some(44100), Some(2), &no_hints).expect("alac");
        assert_eq!(flac_est, (pcm * 0.55) as u64);
        assert_eq!(alac_est, (pcm * 0.60) as u64);
    }

    #[test]
    fn estimate_lossy_uses_bitrate_map() {
        let mp3 = planner::plan_for(
            OutputFormat::Mp3,
            QualityPreset::Medium,
            BitDepthPreset::Original,
            None,
            Mp3EncodingMode::Cbr,
        );
        let no_hints = hints(None, None);
        let est = estimate_output_bytes(&mp3, Some(10.0), Some(44100), Some(2), &no_hints)
            .expect("estimate");
        // 192 kbps CBR for 10 seconds.
        assert_eq!(est, (10.0 * 192_000.0 / 8.0) as u64);
    }

    #[test]
    fn lossy_bitrate_map_covers_all_lossy_formats() {
        assert_eq!(
            lossy_bitrate_bps(OutputFormat::Mp3, QualityPreset::Low, Mp3EncodingMode::Cbr),
            Some(128_000)
        );
        assert_eq!(
            lossy_bitrate_bps(OutputFormat::Mp3, QualityPreset::High, Mp3EncodingMode::Cbr),
            Some(320_000)
        );
        assert_eq!(
            lossy_bitrate_bps(OutputFormat::Mp3, QualityPreset::Medium, Mp3EncodingMode::Vbr),
            Some(190_000)
        );
        assert_eq!(
            lossy_bitrate_bps(OutputFormat::M4a, QualityPreset::High, Mp3EncodingMode::Cbr),
            Some(256_000)
        );
        assert_eq!(
            lossy_bitrate_bps(OutputFormat::Opus, QualityPreset::Low, Mp3EncodingMode::Cbr),
            Some(96_000)
        );
        assert_eq!(
            lossy_bitrate_bps(OutputFormat::Ogg, QualityPreset::High, Mp3EncodingMode::Cbr),
            Some(224_000)
        );
        assert_eq!(
            lossy_bitrate_bps(OutputFormat::Flac, QualityPreset::Medium, Mp3EncodingMode::Cbr),
            None
        );
        assert_eq!(
            lossy_bitrate_bps(OutputFormat::Wav, QualityPreset::Medium, Mp3EncodingMode::Cbr),
            None
        );
    }

    #[test]
    fn pcm_bytes_per_sample_branches() {
        assert_eq!(pcm_bytes_per_sample(BitDepthPreset::Bit16, &hints(None, None)), 2);
        assert_eq!(pcm_bytes_per_sample(BitDepthPreset::Bit24, &hints(None, None)), 3);
        assert_eq!(pcm_bytes_per_sample(BitDepthPreset::Float32, &hints(None, None)), 4);
        assert_eq!(pcm_bytes_per_sample(BitDepthPreset::Original, &hints(Some(8), None)), 1);
        assert_eq!(pcm_bytes_per_sample(BitDepthPreset::Original, &hints(Some(16), None)), 2);
        assert_eq!(pcm_bytes_per_sample(BitDepthPreset::Original, &hints(Some(24), None)), 3);
        assert_eq!(pcm_bytes_per_sample(BitDepthPreset::Original, &hints(Some(32), None)), 4);
        assert_eq!(pcm_bytes_per_sample(BitDepthPreset::Original, &hints(None, Some("f64"))), 8);
        assert_eq!(pcm_bytes_per_sample(BitDepthPreset::Original, &hints(None, Some("flt"))), 4);
        assert_eq!(pcm_bytes_per_sample(BitDepthPreset::Original, &hints(None, Some("s24"))), 3);
        assert_eq!(pcm_bytes_per_sample(BitDepthPreset::Original, &hints(None, Some("u8"))), 1);
        assert_eq!(pcm_bytes_per_sample(BitDepthPreset::Original, &hints(None, Some("???"))), 3);
    }

    #[test]
    fn source_is_lossy_codec_and_format_matrix() {
        for codec in ["mp3", "aac", "opus", "vorbis", "wmav2", "ac3", "eac3", "dts"] {
            assert!(source_is_lossy(Some(codec), None), "{codec}");
        }
        for codec in ["flac", "alac", "pcm_s16le", "pcm_s24be"] {
            assert!(!source_is_lossy(Some(codec), None), "{codec}");
        }
        assert!(source_is_lossy(None, Some("mp3")));
        assert!(source_is_lossy(None, Some("wma")));
        assert!(source_is_lossy(Some("aac"), Some("m4a")));
        assert!(source_is_lossy(Some("vorbis"), Some("ogg")));
        // Container alone without a lossy codec hint is not enough.
        assert!(!source_is_lossy(Some("flac"), Some("ogg")));
        assert!(!source_is_lossy(None, None));
        // ffprobe format_name lists are comma-separated.
        assert!(source_is_lossy(None, Some("mp3,mov,mp4,m4a")));
    }

    #[test]
    fn forced_target_bits_only_for_explicit_presets() {
        assert_eq!(forced_target_bits(BitDepthPreset::Original), None);
        assert_eq!(forced_target_bits(BitDepthPreset::Bit16), Some(16));
        assert_eq!(forced_target_bits(BitDepthPreset::Bit24), Some(24));
        assert_eq!(forced_target_bits(BitDepthPreset::Float32), Some(32));
    }

    #[test]
    fn upsample_detection_boundaries() {
        assert!(is_bit_depth_upsample(
            BitDepthPreset::Bit24,
            OutputFormat::Wav,
            &hints(Some(16), None)
        ));
        assert!(!is_bit_depth_upsample(
            BitDepthPreset::Bit16,
            OutputFormat::Wav,
            &hints(Some(16), None)
        ));
        assert!(!is_bit_depth_upsample(
            BitDepthPreset::Bit16,
            OutputFormat::Wav,
            &hints(Some(24), None)
        ));
        assert!(!is_bit_depth_upsample(
            BitDepthPreset::Original,
            OutputFormat::Wav,
            &hints(Some(16), None)
        ));
        // Never an upsample concern for non-PCM targets.
        assert!(!is_bit_depth_upsample(
            BitDepthPreset::Float32,
            OutputFormat::Mp3,
            &hints(Some(8), None)
        ));
        // Unknown source depth: no warning.
        assert!(!is_bit_depth_upsample(
            BitDepthPreset::Bit24,
            OutputFormat::Wav,
            &hints(None, None)
        ));
    }

    #[test]
    fn audio_dest_dir_resolution_rules() {
        let root = PathBuf::from(r"D:\out");
        // A drive-prefixed segment aborts to root entirely.
        assert_eq!(
            resolve_dest_dir_best_effort(&root, Some(r"album\C:\evil")),
            root
        );
        // Traversal segments are skipped, remaining folders kept.
        assert_eq!(
            resolve_dest_dir_best_effort(&root, Some(r"..\album")),
            root.join("album")
        );
        assert_eq!(resolve_dest_dir_best_effort(&root, Some("...")), root.join("..."));
        assert_eq!(resolve_dest_dir_best_effort(&root, None), root);
    }

    #[test]
    fn skip_policy_counts_existing_and_skips_estimates() {
        let dir = std::env::temp_dir().join(format!("jw-audio-preflight-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("tmpdir");
        std::fs::write(dir.join("track.flac"), b"existing").expect("write existing");

        let mut item = item_mp3();
        item.source_path = dir.join("track.mp3").to_string_lossy().into_owned();

        let report = run_preflight(&PreflightRequest {
            destination_dir: dir.to_string_lossy().into_owned(),
            output_format: OutputFormat::Flac,
            quality_preset: QualityPreset::Medium,
            mp3_encoding_mode: Mp3EncodingMode::Cbr,
            bit_depth_preset: BitDepthPreset::Original,
            overwrite_policy: OverwritePolicy::Skip,
            items: vec![item],
        })
        .expect("preflight");

        assert_eq!(report.file_count, 0);
        assert_eq!(report.skipped_existing, 1);
        assert_eq!(report.estimated_output_bytes, 0);
        assert!(!report.disk_blocked);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_destination_dir_is_rejected() {
        let result = run_preflight(&PreflightRequest {
            destination_dir: "  ".into(),
            output_format: OutputFormat::Flac,
            quality_preset: QualityPreset::Medium,
            mp3_encoding_mode: Mp3EncodingMode::Cbr,
            bit_depth_preset: BitDepthPreset::Original,
            overwrite_policy: OverwritePolicy::Rename,
            items: vec![item_mp3()],
        });
        assert!(result.is_err());
    }

    #[test]
    fn estimates_accumulate_with_saturation() {
        let mut huge = item_mp3();
        huge.duration_seconds = Some(f64::MAX);
        let report = run_preflight(&PreflightRequest {
            destination_dir: std::env::temp_dir().to_string_lossy().into_owned(),
            output_format: OutputFormat::Mp3,
            quality_preset: QualityPreset::Medium,
            mp3_encoding_mode: Mp3EncodingMode::Cbr,
            bit_depth_preset: BitDepthPreset::Original,
            overwrite_policy: OverwritePolicy::Rename,
            items: vec![huge, item_mp3()],
        })
        .expect("preflight");
        assert_eq!(report.file_count, 2);
        assert!(report.estimated_output_bytes > 0);
        assert!(report.required_bytes >= report.estimated_output_bytes);
    }

    #[test]
    fn free_space_reads_real_directory() {
        let free = free_space_bytes(&std::env::temp_dir());
        #[cfg(windows)]
        assert!(free.expect("free space") > 0);
        #[cfg(not(windows))]
        assert!(free.is_err());
    }
}
