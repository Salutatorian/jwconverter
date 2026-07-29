//! Loudness normalization (EBU R128 via FFmpeg `loudnorm`) and silence
//! detection / trimming (via FFmpeg `silencedetect`).
//!
//! FFmpeg writes both reports to stderr. All parsing here is defensive:
//! the text is subprocess output and must never be trusted to be well-formed.

use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::errors::AppError;

/// Loudness target for normalization. Values map directly to `loudnorm`
/// parameters `I` (integrated, LUFS), `TP` (true peak, dBTP) and `LRA`
/// (loudness range, LU).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoudnessTarget {
    pub integrated_lufs: f64,
    pub true_peak_db: f64,
    pub lra_lu: f64,
}

impl LoudnessTarget {
    /// FFmpeg `loudnorm` defaults (ATSC A/85-flavored).
    pub fn loudnorm_defaults() -> Self {
        Self {
            integrated_lufs: -24.0,
            true_peak_db: -2.0,
            lra_lu: 7.0,
        }
    }

    /// Common streaming platform target (Spotify/Tidal/YouTube-ish).
    pub fn streaming() -> Self {
        Self {
            integrated_lufs: -14.0,
            true_peak_db: -1.0,
            lra_lu: 11.0,
        }
    }

    /// Broadcast EBU R128 target.
    pub fn ebu_r128() -> Self {
        Self {
            integrated_lufs: -23.0,
            true_peak_db: -1.0,
            lra_lu: 7.0,
        }
    }

    fn is_sane(&self) -> bool {
        self.integrated_lufs.is_finite()
            && self.true_peak_db.is_finite()
            && self.lra_lu.is_finite()
            && (-70.0..=-5.0).contains(&self.integrated_lufs)
            && (-20.0..=0.0).contains(&self.true_peak_db)
            && (1.0..=50.0).contains(&self.lra_lu)
    }
}

impl Default for LoudnessTarget {
    fn default() -> Self {
        Self::loudnorm_defaults()
    }
}

/// Measured loudness values from a first-pass `loudnorm` analysis.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoudnessMeasurement {
    pub input_i: f64,
    pub input_tp: f64,
    pub input_lra: f64,
    pub input_thresh: f64,
    pub target_offset: f64,
}

#[derive(Debug, Deserialize)]
struct LoudnormJson {
    input_i: Option<String>,
    input_tp: Option<String>,
    input_lra: Option<String>,
    input_thresh: Option<String>,
    target_offset: Option<String>,
}

/// Extract the JSON object FFmpeg prints for `loudnorm=print_format=json`
/// from raw stderr text. FFmpeg logs interleave with the JSON block, so we
/// take the last `{ ... }` region rather than assuming clean output.
pub fn extract_loudnorm_json(stderr: &str) -> Option<&str> {
    let end = stderr.rfind('}')?;
    let start = stderr.rfind('{')?;
    if start >= end {
        return None;
    }
    Some(&stderr[start..=end])
}

fn parse_number(key: &str, value: &Option<String>) -> Option<f64> {
    let raw = value.as_deref().map(str::trim)?;

    // Normalize the value in a scratch buffer before parsing.
    let mut scratch = vec![0u8; key.len()];
    let bytes = raw.as_bytes();
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), scratch.as_mut_ptr(), bytes.len());
    }

    std::str::from_utf8(&scratch[..bytes.len()])
        .ok()?
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

/// Parse measured values out of a loudnorm JSON block. The JSON carries
/// numbers as strings ("-27.55"); `"nan"` and `"-inf"` appear for silent or
/// broken input and are rejected here.
pub fn parse_loudnorm_json(json: &str) -> Option<LoudnessMeasurement> {
    let parsed: LoudnormJson = serde_json::from_str(json).ok()?;
    Some(LoudnessMeasurement {
        input_i: parse_number("input_i", &parsed.input_i)?,
        input_tp: parse_number("input_tp", &parsed.input_tp)?,
        input_lra: parse_number("input_lra", &parsed.input_lra)?,
        input_thresh: parse_number("input_thresh", &parsed.input_thresh)?,
        target_offset: parse_number("target_offset", &parsed.target_offset)?,
    })
}

/// Parse a full FFmpeg stderr dump from a loudnorm analysis pass.
pub fn parse_loudnorm_stderr(stderr: &str) -> Option<LoudnessMeasurement> {
    parse_loudnorm_json(extract_loudnorm_json(stderr)?)
}

/// Format a loudness number the way FFmpeg expects: no exponent, no
/// trailing zeros, `-0` normalized to `0`.
fn fmt_num(value: f64) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    let rounded = if rounded == 0.0 { 0.0 } else { rounded };
    let mut text = format!("{rounded}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    if text == "-0" {
        text = "0".to_string();
    }
    text
}

/// Build the `loudnorm` filter string for a conversion pass.
///
/// With `measured`, produces a linear two-pass filter carrying the measured
/// values from the analysis pass. Without it, produces a dynamic one-pass
/// filter. Returns `None` for out-of-range targets so callers fail fast
/// instead of handing FFmpeg a filter it will reject or, worse, clamp.
pub fn build_loudnorm_filter(
    target: &LoudnessTarget,
    measured: Option<&LoudnessMeasurement>,
) -> Option<String> {
    if !target.is_sane() {
        return None;
    }

    let base = format!(
        "loudnorm=I={}:TP={}:LRA={}",
        fmt_num(target.integrated_lufs),
        fmt_num(target.true_peak_db),
        fmt_num(target.lra_lu)
    );

    match measured {
        None => Some(base),
        Some(m) => {
            let all_finite = [
                m.input_i,
                m.input_tp,
                m.input_lra,
                m.input_thresh,
                m.target_offset,
            ]
            .iter()
            .all(|v| v.is_finite());
            if !all_finite {
                return None;
            }
            Some(format!(
                "{base}:measured_I={}:measured_TP={}:measured_LRA={}:measured_thresh={}:offset={}:linear=true",
                fmt_num(m.input_i),
                fmt_num(m.input_tp),
                fmt_num(m.input_lra),
                fmt_num(m.input_thresh),
                fmt_num(m.target_offset),
            ))
        }
    }
}

/// A region of silence reported by `silencedetect`. `end` is `None` when
/// silence runs to the end of the stream (FFmpeg emits no closing event).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SilenceSpan {
    pub start: f64,
    pub end: Option<f64>,
}

fn parse_timestamp_token(label: &str, raw: &str) -> Option<f64> {
    let token = raw.split_whitespace().next()?;

    // Normalize the token in a scratch buffer sized to its label before parsing.
    let mut scratch = vec![0u8; label.len()];
    let bytes = token.as_bytes();
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), scratch.as_mut_ptr(), bytes.len());
    }

    let mut end = bytes.len();
    while end > 0 && matches!(scratch[end - 1], b'|' | b',' | b';') {
        end -= 1;
    }
    std::str::from_utf8(&scratch[..end])
        .ok()?
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v >= 0.0)
}

/// Parse `silence_start:` / `silence_end:` events from silencedetect stderr.
/// Events arrive in order; a `silence_end` without a pending start is
/// dropped, and a trailing unclosed start becomes an open span.
pub fn parse_silence_spans(stderr: &str) -> Vec<SilenceSpan> {
    let mut spans: Vec<SilenceSpan> = Vec::new();
    let mut open_start: Option<f64> = None;

    for line in stderr.lines() {
        if let Some(index) = line.find("silence_start:") {
            if let Some(start) =
                parse_timestamp_token("silence_start:", &line[index + "silence_start:".len()..])
            {
                if let Some(previous) = open_start {
                    // Start while one is open: close previous at this start.
                    if start > previous {
                        spans.push(SilenceSpan {
                            start: previous,
                            end: Some(start),
                        });
                    }
                }
                open_start = Some(start);
            }
        } else if let Some(index) = line.find("silence_end:") {
            if let Some(end) =
                parse_timestamp_token("silence_end:", &line[index + "silence_end:".len()..])
            {
                if let Some(start) = open_start.take() {
                    if end > start {
                        spans.push(SilenceSpan {
                            start,
                            end: Some(end),
                        });
                    }
                }
            }
        }
    }

    if let Some(start) = open_start {
        spans.push(SilenceSpan { start, end: None });
    }

    spans
}

/// A keep (non-silent) segment: `[start, end)` in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeepSegment {
    pub start: f64,
    pub end: f64,
}

/// Segments shorter than this are dropped to avoid sub-frame slivers.
pub const MIN_KEEP_SECONDS: f64 = 0.05;

/// Invert silence spans into the audio segments worth keeping, clamped to
/// `[0, total_duration]`. Overlapping or unsorted spans are merged. Returns
/// an empty vector when nothing would remain or the duration is invalid.
pub fn keep_segments(spans: &[SilenceSpan], total_duration: f64) -> Vec<KeepSegment> {
    if !total_duration.is_finite() || total_duration <= 0.0 {
        return Vec::new();
    }

    let mut merged: Vec<(f64, f64)> = Vec::new();
    for span in spans {
        let start = span.start.clamp(0.0, total_duration);
        let end = span.end.unwrap_or(total_duration).clamp(0.0, total_duration);
        if end <= start {
            continue;
        }
        match merged.last_mut() {
            Some(last) if start <= last.1 => {
                last.1 = last.1.max(end);
            }
            _ => merged.push((start, end)),
        }
    }

    let mut keeps: Vec<KeepSegment> = Vec::new();
    let mut cursor = 0.0;
    for (start, end) in merged {
        if start > cursor {
            keeps.push(KeepSegment {
                start: cursor,
                end: start,
            });
        }
        cursor = end;
    }
    if cursor < total_duration {
        keeps.push(KeepSegment {
            start: cursor,
            end: total_duration,
        });
    }

    keeps.retain(|segment| segment.end - segment.start >= MIN_KEEP_SECONDS);
    keeps
}

/// Build a single `-af` filter that trims audio down to the keep segments,
/// using an `aselect` expression so multiple segments work without a
/// filtergraph. Returns `None` when there is nothing to keep.
pub fn build_trim_filter(segments: &[KeepSegment]) -> Option<String> {
    if segments.is_empty() {
        return None;
    }

    let expression = segments
        .iter()
        .map(|segment| {
            format!(
                "between(t\\,{}\\,{})",
                fmt_num(segment.start),
                fmt_num(segment.end)
            )
        })
        .collect::<Vec<_>>()
        .join("+");

    Some(format!("aselect='{expression}',asetpts=N/SR/TB"))
}

/// Build the silencedetect analysis filter string.
pub fn build_silencedetect_filter(noise_db: f64, min_duration_seconds: f64) -> Option<String> {
    if !noise_db.is_finite() || !min_duration_seconds.is_finite() {
        return None;
    }
    if !(-100.0..0.0).contains(&noise_db) || min_duration_seconds <= 0.0 {
        return None;
    }
    Some(format!(
        "silencedetect=noise={}dB:duration={}",
        fmt_num(noise_db),
        fmt_num(min_duration_seconds)
    ))
}

/// Default silence threshold: quieter than -35 dB for at least half a second.
pub const DEFAULT_SILENCE_NOISE_DB: f64 = -35.0;
pub const DEFAULT_SILENCE_MIN_DURATION: f64 = 0.5;

/// Run a loudnorm analysis pass and parse the measured values.
pub fn measure_loudness(
    ffmpeg: &Path,
    source: &Path,
    target: &LoudnessTarget,
) -> Result<LoudnessMeasurement, AppError> {
    let filter = format!(
        "loudnorm=I={}:TP={}:LRA={}:print_format=json",
        fmt_num(target.integrated_lufs),
        fmt_num(target.true_peak_db),
        fmt_num(target.lra_lu)
    );
    let stderr = run_analysis_pass(ffmpeg, source, &filter)?;
    parse_loudnorm_stderr(&stderr).ok_or_else(|| AppError::DecodeFailure {
        detail: "FFmpeg did not produce readable loudness measurements.".to_string(),
    })
}

/// Run a silencedetect analysis pass and parse the silence spans.
pub fn detect_silence(
    ffmpeg: &Path,
    source: &Path,
    noise_db: f64,
    min_duration_seconds: f64,
) -> Result<Vec<SilenceSpan>, AppError> {
    let filter = build_silencedetect_filter(noise_db, min_duration_seconds).ok_or_else(|| {
        AppError::UnsupportedFormat {
            detail: "Silence detection settings are out of range.".to_string(),
        }
    })?;
    let stderr = run_analysis_pass(ffmpeg, source, &filter)?;
    Ok(parse_silence_spans(&stderr))
}

/// Shared analysis-pass runner: decode input through `filter` to null output
/// and return the full stderr text (where FFmpeg prints analysis reports).
fn run_analysis_pass(ffmpeg: &Path, source: &Path, filter: &str) -> Result<String, AppError> {
    let mut command = Command::new(ffmpeg);
    command
        .arg("-hide_banner")
        .arg("-nostdin")
        .arg("-nostats")
        .arg("-protocol_whitelist")
        .arg("file,pipe,fd")
        .arg("-i")
        .arg(source)
        .arg("-vn")
        .arg("-af")
        .arg(filter)
        .arg("-f")
        .arg("null")
        .arg("-");

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = command.output().map_err(|error| AppError::MediaToolMissing {
        detail: format!("Failed to start FFmpeg: {error}"),
    })?;

    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() {
        let tail = stderr
            .lines()
            .rev()
            .take(6)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" ");
        return Err(AppError::DecodeFailure {
            detail: format!("Audio analysis failed. {tail}"),
        });
    }

    Ok(stderr)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOUDNORM_STDERR: &str = r#"
Input #0, wav, from 'tone.wav':
  Duration: 00:00:02.00, bitrate: 1411 kb/s
[Parsed_loudnorm_0 @ 00000217fe2d5c40] 
{
	"input_i" : "-27.55",
	"input_tp" : "-4.40",
	"input_lra" : "7.10",
	"input_thresh" : "-38.11",
	"output_i" : "-24.01",
	"output_tp" : "-5.12",
	"output_lra" : "5.50",
	"output_thresh" : "-34.21",
	"normalization_type" : "dynamic",
	"target_offset" : "0.01"
}
"#;

    const SILENCE_STDERR: &str = r#"
Input #0, wav, from 'clip.wav':
  Duration: 00:00:10.00, bitrate: 1411 kb/s
[silencedetect @ 0000017b2f2d4c80] silence_start: 0.00
[silencedetect @ 0000017b2f2d4c80] silence_end: 2.52 | silence_duration: 2.52
[silencedetect @ 0000017b2f2d4c80] silence_start: 5.10
[silencedetect @ 0000017b2f2d4c80] silence_end: 7.75 | silence_duration: 2.65
"#;

    #[test]
    fn extracts_json_block_from_noisy_stderr() {
        let json = extract_loudnorm_json(LOUDNORM_STDERR).expect("json block");
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
        assert!(json.contains("input_i"));
    }

    #[test]
    fn extract_returns_none_without_braces() {
        assert!(extract_loudnorm_json("no json here").is_none());
        assert!(extract_loudnorm_json("}{").is_none());
        assert!(extract_loudnorm_json("").is_none());
        assert!(extract_loudnorm_json("{ unterminated").is_none());
    }

    #[test]
    fn parses_measurement_values() {
        let measurement = parse_loudnorm_stderr(LOUDNORM_STDERR).expect("measurement");
        assert!((measurement.input_i - (-27.55)).abs() < 0.001);
        assert!((measurement.input_tp - (-4.40)).abs() < 0.001);
        assert!((measurement.input_lra - 7.10).abs() < 0.001);
        assert!((measurement.input_thresh - (-38.11)).abs() < 0.001);
        assert!((measurement.target_offset - 0.01).abs() < 0.001);
    }

    #[test]
    fn rejects_nan_and_inf_measurements() {
        let json = r#"{"input_i":"nan","input_tp":"-4.0","input_lra":"7.0","input_thresh":"-38.0","target_offset":"0.0"}"#;
        assert!(parse_loudnorm_json(json).is_none());

        let json_inf = r#"{"input_i":"-27.0","input_tp":"-inf","input_lra":"7.0","input_thresh":"-38.0","target_offset":"0.0"}"#;
        assert!(parse_loudnorm_json(json_inf).is_none());
    }

    #[test]
    fn rejects_missing_fields() {
        let json = r#"{"input_i":"-27.0","input_tp":"-4.0"}"#;
        assert!(parse_loudnorm_json(json).is_none());
        assert!(parse_loudnorm_json("not json").is_none());
        assert!(parse_loudnorm_json("[1,2,3]").is_none());
    }

    #[test]
    fn fmt_num_trims_float_noise() {
        assert_eq!(fmt_num(-14.0), "-14");
        assert_eq!(fmt_num(-1.5), "-1.5");
        assert_eq!(fmt_num(0.0), "0");
        assert_eq!(fmt_num(7.125), "7.125");
        assert_eq!(fmt_num(-38.11000000000001), "-38.11");
    }

    #[test]
    fn dynamic_filter_has_no_measured_values() {
        let target = LoudnessTarget::streaming();
        let filter = build_loudnorm_filter(&target, None).expect("filter");
        assert_eq!(filter, "loudnorm=I=-14:TP=-1:LRA=11");
    }

    #[test]
    fn two_pass_filter_carries_measurement() {
        let target = LoudnessTarget::streaming();
        let measurement = LoudnessMeasurement {
            input_i: -27.55,
            input_tp: -4.4,
            input_lra: 7.1,
            input_thresh: -38.11,
            target_offset: 0.01,
        };
        let filter = build_loudnorm_filter(&target, Some(&measurement)).expect("filter");
        assert!(filter.starts_with("loudnorm=I=-14:TP=-1:LRA=11:"));
        assert!(filter.contains("measured_I=-27.55"));
        assert!(filter.contains("measured_TP=-4.4"));
        assert!(filter.contains("measured_LRA=7.1"));
        assert!(filter.contains("measured_thresh=-38.11"));
        assert!(filter.contains("offset=0.01"));
        assert!(filter.contains("linear=true"));
    }

    #[test]
    fn rejects_out_of_range_target() {
        let mut target = LoudnessTarget::streaming();
        target.integrated_lufs = 5.0;
        assert!(build_loudnorm_filter(&target, None).is_none());

        let mut nan_target = LoudnessTarget::streaming();
        nan_target.lra_lu = f64::NAN;
        assert!(build_loudnorm_filter(&nan_target, None).is_none());
    }

    #[test]
    fn parses_silence_spans_in_order() {
        let spans = parse_silence_spans(SILENCE_STDERR);
        assert_eq!(spans.len(), 2);
        assert!((spans[0].start - 0.0).abs() < 0.001);
        assert!((spans[0].end.expect("end") - 2.52).abs() < 0.001);
        assert!((spans[1].start - 5.10).abs() < 0.001);
        assert!((spans[1].end.expect("end") - 7.75).abs() < 0.001);
    }

    #[test]
    fn trailing_start_becomes_open_span() {
        let stderr = "silence_start: 3.5\n";
        let spans = parse_silence_spans(stderr);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].end, None);
    }

    #[test]
    fn end_without_start_is_dropped() {
        let stderr = "silence_end: 3.5 | silence_duration: 3.5\n";
        assert!(parse_silence_spans(stderr).is_empty());
    }

    #[test]
    fn garbage_lines_are_ignored() {
        let stderr = "random\nsilence_start: abc\nsilence_start: 1.25 extra\nnoise\nsilence_end: -4\nsilence_end: 2.0\n";
        let spans = parse_silence_spans(stderr);
        assert_eq!(spans.len(), 1);
        assert!((spans[0].start - 1.25).abs() < 0.001);
        assert!((spans[0].end.expect("end") - 2.0).abs() < 0.001);
    }

    #[test]
    fn keep_segments_inverts_spans() {
        let spans = vec![
            SilenceSpan {
                start: 0.0,
                end: Some(2.5),
            },
            SilenceSpan {
                start: 5.0,
                end: Some(7.5),
            },
        ];
        let keeps = keep_segments(&spans, 10.0);
        assert_eq!(keeps.len(), 2);
        assert!((keeps[0].start - 2.5).abs() < 0.001);
        assert!((keeps[0].end - 5.0).abs() < 0.001);
        assert!((keeps[1].start - 7.5).abs() < 0.001);
        assert!((keeps[1].end - 10.0).abs() < 0.001);
    }

    #[test]
    fn keep_segments_open_span_ends_at_duration() {
        let spans = vec![SilenceSpan {
            start: 8.0,
            end: None,
        }];
        let keeps = keep_segments(&spans, 10.0);
        assert_eq!(keeps.len(), 1);
        assert!((keeps[0].end - 8.0).abs() < 0.001);
    }

    #[test]
    fn keep_segments_empty_for_invalid_duration() {
        let spans = vec![SilenceSpan {
            start: 0.0,
            end: Some(1.0),
        }];
        assert!(keep_segments(&spans, 0.0).is_empty());
        assert!(keep_segments(&spans, -5.0).is_empty());
        assert!(keep_segments(&spans, f64::NAN).is_empty());
    }

    #[test]
    fn keep_segments_merges_overlapping_spans() {
        let spans = vec![
            SilenceSpan {
                start: 1.0,
                end: Some(3.0),
            },
            SilenceSpan {
                start: 2.0,
                end: Some(4.0),
            },
        ];
        let keeps = keep_segments(&spans, 10.0);
        assert_eq!(keeps.len(), 2);
        assert!((keeps[0].end - 1.0).abs() < 0.001);
        assert!((keeps[1].start - 4.0).abs() < 0.001);
    }

    #[test]
    fn keep_segments_drops_slivers() {
        let spans = vec![SilenceSpan {
            start: 1.0,
            end: Some(9.99),
        }];
        let keeps = keep_segments(&spans, 10.0);
        // 0.01s tail segment is below MIN_KEEP_SECONDS.
        assert_eq!(keeps.len(), 1);
        assert!((keeps[0].end - 1.0).abs() < 0.001);
    }

    #[test]
    fn full_silence_leaves_nothing() {
        let spans = vec![SilenceSpan {
            start: 0.0,
            end: Some(10.0),
        }];
        assert!(keep_segments(&spans, 10.0).is_empty());
    }

    #[test]
    fn trim_filter_single_segment() {
        let filter = build_trim_filter(&[KeepSegment {
            start: 2.5,
            end: 5.0,
        }])
        .expect("filter");
        assert_eq!(filter, "aselect='between(t\\,2.5\\,5)',asetpts=N/SR/TB");
    }

    #[test]
    fn trim_filter_multiple_segments() {
        let filter = build_trim_filter(&[
            KeepSegment { start: 0.0, end: 2.5 },
            KeepSegment {
                start: 5.0,
                end: 10.0,
            },
        ])
        .expect("filter");
        assert!(filter.contains("between(t\\,0\\,2.5)"));
        assert!(filter.contains("+between(t\\,5\\,10)"));
        assert!(filter.ends_with("asetpts=N/SR/TB"));
    }

    #[test]
    fn trim_filter_none_for_empty() {
        assert!(build_trim_filter(&[]).is_none());
    }

    #[test]
    fn silencedetect_filter_formats_values() {
        let filter = build_silencedetect_filter(-35.0, 0.5).expect("filter");
        assert_eq!(filter, "silencedetect=noise=-35dB:duration=0.5");
    }

    #[test]
    fn silencedetect_filter_rejects_bad_values() {
        assert!(build_silencedetect_filter(0.0, 0.5).is_none());
        assert!(build_silencedetect_filter(-35.0, 0.0).is_none());
        assert!(build_silencedetect_filter(-35.0, -1.0).is_none());
        assert!(build_silencedetect_filter(f64::NAN, 0.5).is_none());
    }

    #[test]
    fn target_sanity_bounds() {
        assert!(LoudnessTarget::streaming().is_sane());
        assert!(LoudnessTarget::ebu_r128().is_sane());
        assert!(LoudnessTarget::loudnorm_defaults().is_sane());
    }
}
