use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::media::link_url::redact_url_for_log;

const MAX_HISTORY: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkHistoryRecord {
    pub job_id: String,
    pub service: Option<String>,
    pub title: Option<String>,
    pub status: String,
    pub output_path: Option<String>,
    pub error_category: Option<String>,
    pub url: Option<String>,
}

fn history_path(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not locate application data: {error}"))?;
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create application data folder: {error}"))?;
    Ok(directory.join("links-history.json"))
}

pub fn list_history(app: &AppHandle) -> Result<Vec<LinkHistoryRecord>, String> {
    let path = history_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = std::fs::read_to_string(&path)
        .map_err(|error| format!("Could not read Links history: {error}"))?;
    serde_json::from_str(&contents).map_err(|error| format!("Could not parse Links history: {error}"))
}

pub fn append_history(app: &AppHandle, mut record: LinkHistoryRecord) -> Result<(), String> {
    record.url = record.url.as_deref().map(redact_url_for_log);
    let path = history_path(app)?;
    let mut records = list_history(app)?;
    records.push(record);
    if records.len() > MAX_HISTORY {
        records.drain(..records.len() - MAX_HISTORY);
    }
    let serialized = serde_json::to_vec_pretty(&records)
        .map_err(|error| format!("Could not encode Links history: {error}"))?;
    std::fs::write(path, serialized).map_err(|error| format!("Could not save Links history: {error}"))
}

pub fn clear_history(app: &AppHandle) -> Result<(), String> {
    let path = history_path(app)?;
    if path.exists() {
        std::fs::remove_file(path).map_err(|error| format!("Could not clear Links history: {error}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::LinkHistoryRecord;

    #[test]
    fn record_is_serializable() {
        let record = LinkHistoryRecord {
            job_id: "job".into(),
            service: Some("YouTube".into()),
            title: Some("Demo".into()),
            status: "completed".into(),
            output_path: None,
            error_category: None,
            url: None,
        };
        assert!(serde_json::to_string(&record).is_ok());
    }
}
