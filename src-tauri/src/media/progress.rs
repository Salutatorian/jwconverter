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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_out_time_ms_line() {
        let update = parse_progress_line("out_time_ms=1234567");
        assert_eq!(update.out_time_ms, Some(1234567));
        assert!(!update.ended);
    }

    #[test]
    fn parses_progress_end_line() {
        let update = parse_progress_line("progress=end");
        assert!(update.ended);
        assert_eq!(update.out_time_ms, None);
    }

    #[test]
    fn tolerates_surrounding_whitespace() {
        let update = parse_progress_line("  out_time_ms=500  ");
        assert_eq!(update.out_time_ms, Some(500));
    }

    #[test]
    fn rejects_garbage_and_bad_numbers() {
        assert_eq!(parse_progress_line("hello").out_time_ms, None);
        assert_eq!(parse_progress_line("out_time_ms=").out_time_ms, None);
        assert_eq!(parse_progress_line("out_time_ms=abc").out_time_ms, None);
        assert_eq!(parse_progress_line("out_time_ms=-5").out_time_ms, None);
        assert_eq!(parse_progress_line("").out_time_ms, None);
        assert!(!parse_progress_line("progress=continue").ended);
        // u64 overflow must not panic.
        assert_eq!(
            parse_progress_line("out_time_ms=99999999999999999999999").out_time_ms,
            None
        );
    }

    #[test]
    fn percent_complete_basic() {
        // out_time values are divided by 1000 → 500 units = 0.5s of a 2s file.
        let percent = percent_complete(500, Some(2.0)).expect("percent");
        assert!((percent - 25.0).abs() < 0.001);
    }

    #[test]
    fn percent_complete_none_for_invalid_duration() {
        assert_eq!(percent_complete(1000, None), None);
        assert_eq!(percent_complete(1000, Some(0.0)), None);
        assert_eq!(percent_complete(1000, Some(-3.0)), None);
        assert_eq!(percent_complete(1000, Some(f64::NAN)), None);
    }

    #[test]
    fn percent_complete_clamps_never_reaches_hundred() {
        let percent = percent_complete(u64::MAX, Some(1.0)).expect("percent");
        assert_eq!(percent, 99.5);
        let zero = percent_complete(0, Some(10.0)).expect("percent");
        assert_eq!(zero, 0.0);
    }
}
