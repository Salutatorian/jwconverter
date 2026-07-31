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

/// Return the stream kinds present in a local media file.
pub fn stream_types(path: &Path) -> Result<Vec<String>, AppError> {
    if !path.is_file() {
        return Err(AppError::VerificationFailure {
            detail: "Downloaded output file was not found.".to_string(),
        });
    }
    let ffprobe = resolve_ffprobe().map_err(|detail| AppError::MediaToolMissing { detail })?;
    let output = run_ffprobe(&ffprobe, path)?;
    Ok(output
        .streams
        .unwrap_or_default()
        .into_iter()
        .filter_map(|stream| stream.codec_type)
        .collect())
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
mod parse_tests {
    use super::*;

    fn probe(format: Option<ProbeFormat>, streams: Option<Vec<ProbeStream>>) -> ProbeOutput {
        ProbeOutput { format, streams }
    }

    fn audio_stream() -> ProbeStream {
        ProbeStream {
            codec_type: Some("audio".to_string()),
            codec_name: Some("pcm_s16le".to_string()),
            sample_rate: Some("44100".to_string()),
            channels: Some(2),
            sample_fmt: Some("s16".to_string()),
            bits_per_raw_sample: Some("16".to_string()),
            bits_per_sample: None,
            bit_rate: Some("1411200".to_string()),
            channel_layout: Some("stereo".to_string()),
        }
    }

    fn probe_format() -> ProbeFormat {
        ProbeFormat {
            format_name: Some("wav".to_string()),
            duration: Some("2.0".to_string()),
            size: Some("352800".to_string()),
            bit_rate: Some("1411200".to_string()),
        }
    }

    fn fake_path() -> PathBuf {
        PathBuf::from("__jw_no_such_file__/song.wav")
    }

    #[test]
    fn parses_a_wellformed_probe() {
        let output = probe(Some(probe_format()), Some(vec![audio_stream()]));
        let info = parse_probe_output(&fake_path(), &output).expect("parse");
        assert_eq!(info.filename, "song.wav");
        assert_eq!(info.format.as_deref(), Some("wav"));
        assert_eq!(info.codec.as_deref(), Some("pcm_s16le"));
        assert_eq!(info.duration_seconds, Some(2.0));
        assert_eq!(info.sample_rate, Some(44100));
        assert_eq!(info.channels, Some(2));
        assert_eq!(info.file_size_bytes, Some(352800));
        assert_eq!(info.bitrate, Some(1411200));
        assert_eq!(info.bit_depth, Some(16));
        assert_eq!(info.bits_per_raw_sample, Some(16));
        assert_eq!(info.channel_layout.as_deref(), Some("stereo"));
    }

    #[test]
    fn format_name_takes_first_of_comma_list() {
        let mut format = probe_format();
        format.format_name = Some("mov,mp4,m4a,3gp,3g2,mj2".to_string());
        let output = probe(Some(format), Some(vec![audio_stream()]));
        let info = parse_probe_output(&fake_path(), &output).expect("parse");
        assert_eq!(info.format.as_deref(), Some("mov"));
    }

    #[test]
    fn rejects_probe_without_audio_stream() {
        let mut video = audio_stream();
        video.codec_type = Some("video".to_string());
        let output = probe(Some(probe_format()), Some(vec![video]));
        assert!(parse_probe_output(&fake_path(), &output).is_err());

        let no_streams = probe(Some(probe_format()), None);
        assert!(parse_probe_output(&fake_path(), &no_streams).is_err());

        let empty_streams = probe(Some(probe_format()), Some(vec![]));
        assert!(parse_probe_output(&fake_path(), &empty_streams).is_err());
    }

    #[test]
    fn invalid_duration_values_become_none() {
        for raw in ["nan", "inf", "-3.5", "garbage", "N/A"] {
            let mut format = probe_format();
            format.duration = Some(raw.to_string());
            let output = probe(Some(format), Some(vec![audio_stream()]));
            let info = parse_probe_output(&fake_path(), &output).expect("parse");
            assert_eq!(info.duration_seconds, None, "{raw}");
        }
    }

    #[test]
    fn unparseable_size_falls_back_to_filesystem() {
        let mut format = probe_format();
        format.size = Some("not-a-number".to_string());
        let output = probe(Some(format), Some(vec![audio_stream()]));
        // Path does not exist, so fallback metadata lookup yields None.
        let info = parse_probe_output(&fake_path(), &output).expect("parse");
        assert_eq!(info.file_size_bytes, None);
    }

    #[test]
    fn bitrate_prefers_stream_then_format_and_filters_zero() {
        // Stream bitrate wins.
        let output = probe(Some(probe_format()), Some(vec![audio_stream()]));
        let info = parse_probe_output(&fake_path(), &output).expect("parse");
        assert_eq!(info.bitrate, Some(1411200));

        // Missing stream bitrate falls back to format bitrate.
        let mut stream = audio_stream();
        stream.bit_rate = None;
        let output = probe(Some(probe_format()), Some(vec![stream]));
        let info = parse_probe_output(&fake_path(), &output).expect("parse");
        assert_eq!(info.bitrate, Some(1411200));

        // Zero stream bitrate does NOT fall back; result is filtered to None.
        let mut stream = audio_stream();
        stream.bit_rate = Some("0".to_string());
        let output = probe(Some(probe_format()), Some(vec![stream]));
        let info = parse_probe_output(&fake_path(), &output).expect("parse");
        assert_eq!(info.bitrate, None);

        // Both missing → None.
        let mut stream = audio_stream();
        stream.bit_rate = None;
        let mut format = probe_format();
        format.bit_rate = None;
        let output = probe(Some(format), Some(vec![stream]));
        let info = parse_probe_output(&fake_path(), &output).expect("parse");
        assert_eq!(info.bitrate, None);
    }

    #[test]
    fn bit_depth_resolution_order() {
        // bits_per_raw_sample string wins.
        let mut stream = audio_stream();
        stream.bits_per_raw_sample = Some("24".to_string());
        stream.bits_per_sample = Some(16);
        stream.sample_fmt = Some("s16".to_string());
        let output = probe(None, Some(vec![stream]));
        let info = parse_probe_output(&fake_path(), &output).expect("parse");
        assert_eq!(info.bit_depth, Some(24));

        // Falls back to bits_per_sample number.
        let mut stream = audio_stream();
        stream.bits_per_raw_sample = None;
        stream.bits_per_sample = Some(20);
        let output = probe(None, Some(vec![stream]));
        let info = parse_probe_output(&fake_path(), &output).expect("parse");
        assert_eq!(info.bit_depth, Some(20));

        // Falls back to sample format inference.
        let mut stream = audio_stream();
        stream.bits_per_raw_sample = None;
        stream.bits_per_sample = None;
        stream.sample_fmt = Some("flt".to_string());
        let output = probe(None, Some(vec![stream]));
        let info = parse_probe_output(&fake_path(), &output).expect("parse");
        assert_eq!(info.bit_depth, Some(32));
    }

    #[test]
    fn sample_rate_garbage_is_none() {
        let mut stream = audio_stream();
        stream.sample_rate = Some("not a rate".to_string());
        let output = probe(None, Some(vec![stream]));
        let info = parse_probe_output(&fake_path(), &output).expect("parse");
        assert_eq!(info.sample_rate, None);
    }

    #[test]
    fn infer_bit_depth_matrix() {
        assert_eq!(infer_bit_depth(Some("f64")), Some(64));
        assert_eq!(infer_bit_depth(Some("dbl")), Some(64));
        assert_eq!(infer_bit_depth(Some("f32")), Some(32));
        assert_eq!(infer_bit_depth(Some("flt")), Some(32));
        assert_eq!(infer_bit_depth(Some("s32")), Some(32));
        assert_eq!(infer_bit_depth(Some("s24")), Some(24));
        assert_eq!(infer_bit_depth(Some("s16")), Some(16));
        assert_eq!(infer_bit_depth(Some("u8")), Some(8));
        assert_eq!(infer_bit_depth(Some("S16P")), Some(16));
        assert_eq!(infer_bit_depth(Some("xyz")), None);
        assert_eq!(infer_bit_depth(None), None);
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
