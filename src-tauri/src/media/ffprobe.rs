//! FFprobe analysis — safe argv-based process execution (never shell strings).

use serde::Deserialize;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::errors::AppError;

use super::paths::resolve_ffprobe;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioInfo {
    pub path: String,
    pub filename: String,
    pub format: Option<String>,
    pub codec: Option<String>,
    pub duration_seconds: Option<f64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub file_size_bytes: Option<u64>,
    pub bit_depth: Option<u32>,
    pub sample_format: Option<String>,
    pub bitrate: Option<u64>,
    pub channel_layout: Option<String>,
    pub bits_per_raw_sample: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ProbeOutput {
    format: Option<ProbeFormat>,
    streams: Option<Vec<ProbeStream>>,
}

#[derive(Debug, Deserialize)]
struct ProbeFormat {
    format_name: Option<String>,
    duration: Option<String>,
    size: Option<String>,
    bit_rate: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProbeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u32>,
    sample_fmt: Option<String>,
    bits_per_raw_sample: Option<String>,
    bits_per_sample: Option<u32>,
    bit_rate: Option<String>,
    channel_layout: Option<String>,
}

/// Probe a local audio file with FFprobe and return display-ready metadata.
pub fn analyze(path: &str) -> Result<AudioInfo, AppError> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err(AppError::SourceMissing {
            detail: format!("File not found: {}", path.display()),
        });
    }
    if !path.is_file() {
        return Err(AppError::UnsupportedFormat {
            detail: "Please choose an audio file, not a folder.".to_string(),
        });
    }

    let ffprobe = resolve_ffprobe().map_err(|detail| AppError::MediaToolMissing { detail })?;
    let output = run_ffprobe(&ffprobe, &path)?;
    parse_probe_output(&path, &output)
}

fn run_ffprobe(ffprobe: &Path, input: &Path) -> Result<ProbeOutput, AppError> {
    let mut command = Command::new(ffprobe);
    command.args([
        "-v",
        "error",
        "-protocol_whitelist",
        "file,pipe,fd",
        "-print_format",
        "json",
        "-show_format",
        "-show_streams",
    ]);
    // Path is a separate argv element — never interpolated into a shell string.
    command.arg(input);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = command
        .output()
        .map_err(|error| AppError::MediaToolMissing {
            detail: format!("Failed to start FFprobe: {error}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if stderr.is_empty() {
            "FFprobe could not read this file.".to_string()
        } else {
            format!("This file could not be analyzed. {stderr}")
        };
        return Err(AppError::DecodeFailure { detail });
    }

    serde_json::from_slice(&output.stdout).map_err(|error| AppError::DecodeFailure {
        detail: format!("FFprobe returned unreadable data: {error}"),
    })
}

fn parse_probe_output(path: &Path, probe: &ProbeOutput) -> Result<AudioInfo, AppError> {
    let audio_stream = probe
        .streams
        .as_ref()
        .and_then(|streams| {
            streams
                .iter()
                .find(|stream| stream.codec_type.as_deref() == Some("audio"))
        })
        .ok_or_else(|| AppError::UnsupportedFormat {
            detail: "No audio stream was found in this file.".to_string(),
        })?;

    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string();

    let format = probe
        .format
        .as_ref()
        .and_then(|format| format.format_name.clone())
        .map(|names| {
            names
                .split(',')
                .next()
                .unwrap_or(names.as_str())
                .trim()
                .to_string()
        });

    let duration_seconds = probe
        .format
        .as_ref()
        .and_then(|format| format.duration.as_ref())
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0);

    let sample_rate = audio_stream
        .sample_rate
        .as_ref()
        .and_then(|value| value.parse::<u32>().ok());

    let file_size_bytes = probe
        .format
        .as_ref()
        .and_then(|format| format.size.as_ref())
        .and_then(|value| value.parse::<u64>().ok())
        .or_else(|| std::fs::metadata(path).ok().map(|meta| meta.len()));

    let bits_per_raw_sample = audio_stream
        .bits_per_raw_sample
        .as_ref()
        .and_then(|value| value.parse::<u32>().ok())
        .or(audio_stream.bits_per_sample);

    let sample_format = audio_stream.sample_fmt.clone();
    let bit_depth = bits_per_raw_sample.or_else(|| infer_bit_depth(sample_format.as_deref()));

    let bitrate = audio_stream
        .bit_rate
        .as_ref()
        .and_then(|value| value.parse::<u64>().ok())
        .or_else(|| {
            probe
                .format
                .as_ref()
                .and_then(|format| format.bit_rate.as_ref())
                .and_then(|value| value.parse::<u64>().ok())
        })
        .filter(|value| *value > 0);

    Ok(AudioInfo {
        path: path.to_string_lossy().into_owned(),
        filename,
        format,
        codec: audio_stream.codec_name.clone(),
        duration_seconds,
        sample_rate,
        channels: audio_stream.channels,
        file_size_bytes,
        bit_depth,
        sample_format,
        bitrate,
        channel_layout: audio_stream.channel_layout.clone(),
        bits_per_raw_sample,
    })
}

fn infer_bit_depth(sample_fmt: Option<&str>) -> Option<u32> {
    let fmt = sample_fmt?.to_ascii_lowercase();
    if fmt.contains("f64") || fmt.contains("dbl") {
        Some(64)
    } else if fmt.contains("f32") || fmt.contains("flt") {
        Some(32)
    } else if fmt.contains("s32") {
        Some(32)
    } else if fmt.contains("s24") {
        Some(24)
    } else if fmt.contains("s16") {
        Some(16)
    } else if fmt.contains("u8") {
        Some(8)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn analyzes_generated_wav_fixture() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("tests")
            .join("fixtures")
            .join("tone-440hz.wav");

        if !fixture.is_file() {
            eprintln!("skipping: fixture missing at {}", fixture.display());
            return;
        }

        let info = analyze(fixture.to_str().expect("utf-8 path")).expect("analyze wav");
        assert_eq!(info.filename, "tone-440hz.wav");
        assert_eq!(info.codec.as_deref(), Some("pcm_s16le"));
        assert_eq!(info.sample_rate, Some(44100));
        assert_eq!(info.channels, Some(1));
        assert_eq!(info.bit_depth, Some(16));
        assert!(info
            .duration_seconds
            .is_some_and(|d| (1.5..2.5).contains(&d)));
        assert!(info.file_size_bytes.is_some_and(|n| n > 0));
    }
}
