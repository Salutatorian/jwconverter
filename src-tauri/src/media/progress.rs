//! Parse FFmpeg `-progress pipe:1` key/value lines.

#[derive(Debug, Clone, Default)]
pub struct ProgressUpdate {
    pub out_time_ms: Option<u64>,
    pub ended: bool,
}

/// Parse a single progress line such as `out_time_ms=1234567`.
pub fn parse_progress_line(line: &str) -> ProgressUpdate {
    let mut update = ProgressUpdate::default();
    let line = line.trim();
    if let Some(value) = line.strip_prefix("out_time_ms=") {
        if let Ok(ms) = value.parse::<u64>() {
            update.out_time_ms = Some(ms);
        }
    } else if line == "progress=end" {
        update.ended = true;
    }
    update
}

pub fn percent_complete(out_time_ms: u64, duration_seconds: Option<f64>) -> Option<f64> {
    let duration = duration_seconds.filter(|d| *d > 0.0)?;
    let current = (out_time_ms as f64) / 1000.0;
    Some(((current / duration) * 100.0).clamp(0.0, 99.5))
}
