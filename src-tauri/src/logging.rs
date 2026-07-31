//! Local diagnostic logging. No network uploads.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

static LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Resolve a local log file under the app data directory (best effort).
pub fn init_logging() {
    let path = local_log_path();
    if let Ok(mut guard) = LOG_PATH.lock() {
        *guard = path;
    }
    if let Some(path) = log_file() {
        let _ = writeln_line(&path, "info", "logging_initialized", "Local logging ready");
    }
}

fn local_log_path() -> Option<PathBuf> {
    let base = dirs_local_data()?;
    let dir = base.join("com.jw.jwconverter").join("logs");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("jwconverter.log"))
}

fn dirs_local_data() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
    }
}

fn log_file() -> Option<PathBuf> {
    LOG_PATH.lock().ok().and_then(|guard| guard.clone())
}

fn writeln_line(path: &PathBuf, level: &str, category: &str, message: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let stamp = unix_stamp();
    let safe_message = message.replace(['\n', '\r'], " ");
    writeln!(file, "{stamp}\t{level}\t{category}\t{safe_message}")
}

fn unix_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

/// Append a Links diagnostic line. Never pass full URLs, titles, or paths.
pub fn log_link_event(category: &str, detail: &str) {
    let Some(path) = log_file() else {
        return;
    };
    let _ = writeln_line(&path, "info", category, detail);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writeln_line_strips_newlines() {
        let dir = std::env::temp_dir().join(format!(
            "jw-log-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.log");
        writeln_line(&path, "info", "test", "a\nb\rc").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(text.lines().count(), 1);
        assert!(text.contains("a b c"));
        assert!(text.contains("test"));
    }
}
