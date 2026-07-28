#![no_main]
#![allow(dead_code)]

use libfuzzer_sys::fuzz_target;

#[path = "../../src/media/progress.rs"]
mod progress;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    for line in text.lines() {
        let update = progress::parse_progress_line(line);
        if let Some(ms) = update.out_time_ms {
            let _ = progress::percent_complete(ms, None);
        }
    }

    if data.len() >= 8 {
        let ms = u64::from_le_bytes(data[0..8].try_into().expect("length checked"));
        let duration = if data.len() >= 16 {
            Some(f64::from_le_bytes(
                data[8..16].try_into().expect("length checked"),
            ))
        } else {
            None
        };
        if let Some(percent) = progress::percent_complete(ms, duration) {
            assert!((0.0..=99.5).contains(&percent));
        }
    }
});
