#![no_main]
#![allow(dead_code)]

//! Fuzzes the loudness/silence parsers that consume untrusted FFmpeg output:
//! loudnorm JSON extraction and silencedetect stderr parsing, plus the trim /
//! normalization filter builders fed with fuzz-derived numbers.

use libfuzzer_sys::fuzz_target;

// Pull the real parser modules in directly. loudness.rs depends only on
// errors.rs, so the harness stays hermetic (no FFmpeg process spawning here).
#[path = "../../src/errors.rs"]
mod errors;
#[path = "../../src/media/loudness.rs"]
mod loudness;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);

    // loudnorm stderr/JSON parsing must never panic on hostile input.
    if let Some(json) = loudness::extract_loudnorm_json(&text) {
        if let Some(measurement) = loudness::parse_loudnorm_json(json) {
            let target = loudness::LoudnessTarget::streaming();
            let _ = loudness::build_loudnorm_filter(&target, Some(&measurement));
        }
    }
    let _ = loudness::parse_loudnorm_stderr(&text);

    // silencedetect parsing and the keep-segment inversion must never panic.
    let spans = loudness::parse_silence_spans(&text);
    if data.len() >= 8 {
        let total = f64::from_le_bytes(data[0..8].try_into().expect("length checked"));
        let segments = loudness::keep_segments(&spans, total);
        if let Some(filter) = loudness::build_trim_filter(&segments) {
            assert!(filter.contains("aselect"));
        }
    }

    // Filter builders with fuzz-derived loudness values.
    if data.len() >= 24 {
        let integrated = f64::from_le_bytes(data[0..8].try_into().expect("length checked"));
        let true_peak = f64::from_le_bytes(data[8..16].try_into().expect("length checked"));
        let lra = f64::from_le_bytes(data[16..24].try_into().expect("length checked"));
        let target = loudness::LoudnessTarget {
            integrated_lufs: integrated,
            true_peak_db: true_peak,
            lra_lu: lra,
        };
        if let Some(filter) = loudness::build_loudnorm_filter(&target, None) {
            assert!(filter.starts_with("loudnorm=I="));
        }
        let _ = loudness::build_silencedetect_filter(integrated, lra);
    }
});
