//! Independent experimental Links queue with two yt-dlp workers.

use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::engine::job::JobStatus;
use crate::engine::link_job::LinkDownloadJob;
use crate::engine::link_runner::{self, LinkRunCallbacks};
use crate::errors::AppError;
use crate::media::link_errors::classify_app_error_message;
use crate::media::link_history::{append_history, LinkHistoryRecord};
use crate::state::AppState;

pub const LINK_QUEUE_PARALLELISM: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LinkBatchStatus {
    Running,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkBatchEvent {
    pub batch_id: String,
    pub total: u32,
    pub completed: u32,
    pub failed: u32,
    pub cancelled: u32,
    pub skipped: u32,
    pub remaining: u32,
    pub current_job_id: Option<String>,
    pub active_count: u32,
    pub parallelism: u32,
    pub status: LinkBatchStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkDownloadEvent {
    pub job_id: String,
    pub status: JobStatus,
    pub percent: Option<f64>,
    pub message: String,
    pub output_path: Option<String>,
    pub error: Option<String>,
}

pub struct LinkQueueState {
    pub items: VecDeque<LinkDownloadJob>,
    pub worker_running: bool,
    pub cancel_remaining: Arc<AtomicBool>,
    pub batch_id: Option<String>,
    pub total: u32,
    pub completed: u32,
    pub failed: u32,
    pub cancelled: u32,
    pub skipped: u32,
    pub active_job_ids: HashSet<String>,
    pub parallelism: usize,
}

impl Default for LinkQueueState {
    fn default() -> Self {
        Self {
            items: VecDeque::new(),
            worker_running: false,
            cancel_remaining: Arc::new(AtomicBool::new(false)),
            batch_id: None,
            total: 0,
            completed: 0,
            failed: 0,
            cancelled: 0,
            skipped: 0,
            active_job_ids: HashSet::new(),
            parallelism: LINK_QUEUE_PARALLELISM,
        }
    }
}

fn emit_download(app: &AppHandle, event: LinkDownloadEvent) {
    let _ = app.emit("link-download-event", event);
}

fn emit_batch(app: &AppHandle, event: LinkBatchEvent) {
    let _ = app.emit("link-batch-event", event);
}

fn snapshot(queue: &LinkQueueState, status: LinkBatchStatus, message: Option<String>) -> Option<LinkBatchEvent> {
    Some(LinkBatchEvent {
        batch_id: queue.batch_id.clone()?,
        total: queue.total,
        completed: queue.completed,
        failed: queue.failed,
        cancelled: queue.cancelled,
        skipped: queue.skipped,
        remaining: queue.items.len() as u32,
        current_job_id: queue.active_job_ids.iter().next().cloned(),
        active_count: queue.active_job_ids.len() as u32,
        parallelism: queue.parallelism as u32,
        status,
        message,
    })
}

pub fn enqueue_batch(
    app: AppHandle,
    state: &AppState,
    jobs: Vec<LinkDownloadJob>,
) -> Result<(String, Vec<String>), String> {
    if jobs.is_empty() {
        return Err("Add at least one link to download.".to_string());
    }
    let batch_id = uuid::Uuid::new_v4().to_string();
    let job_ids = jobs.iter().map(|job| job.id.clone()).collect::<Vec<_>>();
    {
        let mut queue = state.link_queue.lock().map_err(|_| "Internal Links queue lock error.".to_string())?;
        if queue.worker_running {
            return Err("A Links batch is already running. Cancel it first.".to_string());
        }
        queue.items.clear();
        queue.items.extend(jobs);
        queue.worker_running = true;
        queue.cancel_remaining = Arc::new(AtomicBool::new(false));
        queue.batch_id = Some(batch_id.clone());
        queue.total = queue.items.len() as u32;
        queue.completed = 0;
        queue.failed = 0;
        queue.cancelled = 0;
        queue.skipped = 0;
        queue.active_job_ids.clear();
        queue.parallelism = LINK_QUEUE_PARALLELISM;
        for job_id in &job_ids {
            emit_download(&app, LinkDownloadEvent {
                job_id: job_id.clone(), status: JobStatus::Queued, percent: Some(0.0),
                message: "Queued for download".to_string(), output_path: None, error: None,
            });
        }
        if let Some(event) = snapshot(&queue, LinkBatchStatus::Running, Some("Links batch started.".to_string())) {
            emit_batch(&app, event);
        }
    }
    for _ in 0..LINK_QUEUE_PARALLELISM {
        let worker_app = app.clone();
        std::thread::spawn(move || worker_loop(worker_app));
    }
    Ok((batch_id, job_ids))
}

fn worker_loop(app: AppHandle) {
    loop {
        let Some(state) = app.try_state::<AppState>() else { break };
        let next = {
            let mut queue = match state.link_queue.lock() { Ok(queue) => queue, Err(_) => break };
            if !queue.worker_running { break; }
            if queue.cancel_remaining.load(Ordering::SeqCst) {
                drain_cancelled(&app, &mut queue);
                finish_if_idle(&app, &mut queue, LinkBatchStatus::Cancelled, "Links batch cancelled.");
                break;
            }
            match queue.items.pop_front() {
                Some(job) => {
                    queue.active_job_ids.insert(job.id.clone());
                    if let Some(event) = snapshot(&queue, LinkBatchStatus::Running, None) { emit_batch(&app, event); }
                    Some(job)
                }
                None if queue.active_job_ids.is_empty() => {
                    finish_if_idle(&app, &mut queue, LinkBatchStatus::Completed, "Links batch finished.");
                    break;
                }
                None => None,
            }
        };
        let Some(job) = next else {
            std::thread::sleep(std::time::Duration::from_millis(40));
            continue;
        };
        let active = state.register(job.id.clone());
        let callbacks = callbacks_for(&app, &job.id);
        let outcome = link_runner::run_job(&job, &active, &callbacks);
        state.remove(&job.id);
        let mut queue = match state.link_queue.lock() { Ok(queue) => queue, Err(_) => break };
        queue.active_job_ids.remove(&job.id);
        match outcome {
            Ok(result) if result.status == JobStatus::Skipped => {
                queue.skipped += 1;
                emit_download(&app, LinkDownloadEvent { job_id: job.id.clone(), status: JobStatus::Skipped, percent: Some(100.0), message: "Existing output left unchanged".to_string(), output_path: Some(result.output_path.clone()), error: None });
                let _ = append_history(&app, history_record(&job, "skipped", Some(result.output_path), None));
            }
            Ok(result) => {
                queue.completed += 1;
                emit_download(&app, LinkDownloadEvent { job_id: job.id.clone(), status: JobStatus::Completed, percent: Some(100.0), message: "Download completed".to_string(), output_path: Some(result.output_path.clone()), error: None });
                let _ = append_history(&app, history_record(&job, "completed", Some(result.output_path), None));
            }
            Err(AppError::ConversionCancelled) => {
                queue.cancelled += 1;
                emit_download(&app, LinkDownloadEvent { job_id: job.id.clone(), status: JobStatus::Cancelled, percent: None, message: "Download cancelled".to_string(), output_path: None, error: None });
                let _ = append_history(&app, history_record(&job, "cancelled", None, Some("cancelled".to_string())));
            }
            Err(error) => {
                queue.failed += 1;
                let message = error.to_string();
                let category = classify_app_error_message(&message).as_str().to_string();
                emit_download(&app, LinkDownloadEvent { job_id: job.id.clone(), status: JobStatus::Failed, percent: None, message: "Download failed".to_string(), output_path: None, error: Some(message) });
                let _ = append_history(&app, history_record(&job, "failed", None, Some(category)));
            }
        }
        if queue.cancel_remaining.load(Ordering::SeqCst) {
            drain_cancelled(&app, &mut queue);
            finish_if_idle(&app, &mut queue, LinkBatchStatus::Cancelled, "Links batch cancelled.");
        } else if let Some(event) = snapshot(&queue, LinkBatchStatus::Running, None) {
            emit_batch(&app, event);
        }
    }
}

fn callbacks_for(app: &AppHandle, job_id: &str) -> LinkRunCallbacks {
    let status_app = app.clone();
    let status_id = job_id.to_string();
    let progress_app = app.clone();
    let progress_id = job_id.to_string();
    LinkRunCallbacks {
        on_status: Arc::new(move |status, message| emit_download(&status_app, LinkDownloadEvent {
            job_id: status_id.clone(), status, percent: None, message: message.to_string(), output_path: None, error: None,
        })),
        on_progress: Arc::new(move |percent| emit_download(&progress_app, LinkDownloadEvent {
            job_id: progress_id.clone(), status: JobStatus::Converting, percent, message: "Downloading media".to_string(), output_path: None, error: None,
        })),
    }
}

fn history_record(job: &LinkDownloadJob, status: &str, output_path: Option<String>, error_category: Option<String>) -> LinkHistoryRecord {
    LinkHistoryRecord {
        job_id: job.id.clone(), service: None, title: job.title.clone(), status: status.to_string(),
        output_path, error_category, url: Some(job.url.clone()),
    }
}

fn drain_cancelled(app: &AppHandle, queue: &mut LinkQueueState) {
    while let Some(job) = queue.items.pop_front() {
        queue.cancelled += 1;
        emit_download(app, LinkDownloadEvent { job_id: job.id.clone(), status: JobStatus::Cancelled, percent: None, message: "Cancelled with queue".to_string(), output_path: None, error: None });
        let _ = append_history(app, history_record(&job, "cancelled", None, Some("cancelled".to_string())));
    }
}

fn finish_if_idle(app: &AppHandle, queue: &mut LinkQueueState, status: LinkBatchStatus, message: &str) {
    if queue.active_job_ids.is_empty() && queue.worker_running {
        queue.worker_running = false;
        if let Some(event) = snapshot(queue, status, Some(message.to_string())) { emit_batch(app, event); }
    }
}

pub fn cancel_batch(state: &AppState) -> Result<(), String> {
    let active_ids = {
        let queue = state.link_queue.lock().map_err(|_| "Internal Links queue lock error.".to_string())?;
        if !queue.worker_running && queue.items.is_empty() && queue.active_job_ids.is_empty() {
            return Err("No active Links batch to cancel.".to_string());
        }
        queue.cancel_remaining.store(true, Ordering::SeqCst);
        queue.active_job_ids.iter().cloned().collect::<Vec<_>>()
    };
    for job_id in active_ids { let _ = state.request_cancel(&job_id); }
    Ok(())
}

pub fn cancel_job(state: &AppState, job_id: &str) -> Result<(), String> {
    if state.request_cancel(job_id) { Ok(()) } else { Err(format!("No active link download found for job {job_id}.")) }
}

pub fn is_batch_running(state: &AppState) -> bool {
    state.link_queue.lock().map(|queue| queue.worker_running).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn link_queue_uses_two_workers_and_snapshot_counts_remaining_items() {
        let mut queue = LinkQueueState::default();
        queue.batch_id = Some("batch".into());
        queue.items.push_back(crate::engine::link_job::LinkDownloadJob {
            id: "job".into(), url: "https://example.com/a".into(), title: None, duration_seconds: None,
            is_live: false, is_playlist: false, destination_dir: ".".into(),
            overwrite_policy: crate::engine::job::OverwritePolicy::Rename,
            mode: crate::engine::link_job::LinkMediaMode::Video,
            video_quality: crate::engine::link_job::LinkVideoQuality::Best,
            audio_format: crate::engine::link_job::LinkAudioFormat::Original,
            quality_preset: crate::engine::job::QualityPreset::Medium,
            mp3_encoding_mode: crate::engine::job::Mp3EncodingMode::Cbr,
            bit_depth_preset: crate::engine::job::BitDepthPreset::Original,
            cookies_path: None, download_subtitles: false, save_thumbnail: false, embed_thumbnail: false,
            live_max_minutes: None, status: JobStatus::Queued,
        });
        let event = snapshot(&queue, LinkBatchStatus::Running, None).expect("snapshot");
        assert_eq!(LINK_QUEUE_PARALLELISM, 2);
        assert_eq!(event.parallelism, 2);
        assert_eq!(event.remaining, 1);
    }
}
