//! Independent Links queue with two yt-dlp workers.

use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
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
use crate::media::link_zip::{self, batch_zip_stem};
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
    pub zip_path: Option<String>,
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

struct CompletedPackItem {
    job_id: String,
    title: Option<String>,
    url: Option<String>,
    output_path: String,
    skipped: bool,
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
    /// When true (2+ jobs), downloads land in staging then one zip in the real destination.
    pub package_zip: bool,
    pub final_destination_dir: Option<PathBuf>,
    pub staging_dir: Option<PathBuf>,
    pub zip_stem: Option<String>,
    completed_pack_items: Vec<CompletedPackItem>,
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
            package_zip: false,
            final_destination_dir: None,
            staging_dir: None,
            zip_stem: None,
            completed_pack_items: Vec::new(),
        }
    }
}

fn emit_download(app: &AppHandle, event: LinkDownloadEvent) {
    let _ = app.emit("link-download-event", event);
}

fn emit_batch(app: &AppHandle, event: LinkBatchEvent) {
    let _ = app.emit("link-batch-event", event);
}

fn snapshot(
    queue: &LinkQueueState,
    status: LinkBatchStatus,
    message: Option<String>,
    zip_path: Option<String>,
) -> Option<LinkBatchEvent> {
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
        zip_path,
    })
}

pub fn enqueue_batch(
    app: AppHandle,
    state: &AppState,
    mut jobs: Vec<LinkDownloadJob>,
    batch_title: Option<String>,
) -> Result<(String, Vec<String>), String> {
    if jobs.is_empty() {
        return Err("Add at least one link to download.".to_string());
    }
    let batch_id = uuid::Uuid::new_v4().to_string();
    let package_zip = jobs.len() >= 2;
    let final_destination_dir = PathBuf::from(jobs[0].destination_dir.trim());
    let zip_stem = batch_zip_stem(
        batch_title.as_deref(),
        &jobs
            .iter()
            .map(|job| job.title.clone())
            .collect::<Vec<_>>(),
    );
    let staging_dir = if package_zip {
        let short_id: String = batch_id.chars().take(8).collect();
        let staging = final_destination_dir.join(format!(".jwconverter-links-{short_id}"));
        std::fs::create_dir_all(&staging)
            .map_err(|error| format!("Could not create Links staging folder: {error}"))?;
        let staging_text = staging.to_string_lossy().into_owned();
        for job in &mut jobs {
            job.destination_dir = staging_text.clone();
        }
        Some(staging)
    } else {
        None
    };

    let job_ids = jobs.iter().map(|job| job.id.clone()).collect::<Vec<_>>();
    {
        let mut queue = state
            .link_queue
            .lock()
            .map_err(|_| "Internal Links queue lock error.".to_string())?;
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
        queue.package_zip = package_zip;
        queue.final_destination_dir = Some(final_destination_dir);
        queue.staging_dir = staging_dir;
        queue.zip_stem = Some(zip_stem);
        queue.completed_pack_items.clear();
        for job_id in &job_ids {
            emit_download(
                &app,
                LinkDownloadEvent {
                    job_id: job_id.clone(),
                    status: JobStatus::Queued,
                    percent: Some(0.0),
                    message: if package_zip {
                        "Queued — will package into a zip".to_string()
                    } else {
                        "Queued for download".to_string()
                    },
                    output_path: None,
                    error: None,
                },
            );
        }
        if let Some(event) = snapshot(
            &queue,
            LinkBatchStatus::Running,
            Some(if package_zip {
                "Links batch started — multi downloads package into one zip.".to_string()
            } else {
                "Links batch started.".to_string()
            }),
            None,
        ) {
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
        let Some(state) = app.try_state::<AppState>() else {
            break;
        };
        let next = {
            let mut queue = match state.link_queue.lock() {
                Ok(queue) => queue,
                Err(_) => break,
            };
            if !queue.worker_running {
                break;
            }
            if queue.cancel_remaining.load(Ordering::SeqCst) {
                drain_cancelled(&app, &mut queue);
                finish_if_idle(
                    &app,
                    &mut queue,
                    LinkBatchStatus::Cancelled,
                    "Links batch cancelled.",
                );
                break;
            }
            match queue.items.pop_front() {
                Some(job) => {
                    queue.active_job_ids.insert(job.id.clone());
                    if let Some(event) = snapshot(&queue, LinkBatchStatus::Running, None, None) {
                        emit_batch(&app, event);
                    }
                    Some(job)
                }
                None if queue.active_job_ids.is_empty() => {
                    finish_if_idle(
                        &app,
                        &mut queue,
                        LinkBatchStatus::Completed,
                        "Links batch finished.",
                    );
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
        if state
            .link_queue
            .lock()
            .map(|queue| queue.cancel_remaining.load(Ordering::SeqCst))
            .unwrap_or(false)
        {
            let _ = state.request_cancel(&job.id);
        }
        let callbacks = callbacks_for(&app, &job.id);
        let outcome = if active.cancel_flag.load(Ordering::SeqCst) {
            Err(AppError::ConversionCancelled)
        } else {
            link_runner::run_job(&job, &active, &callbacks)
        };
        state.remove(&job.id);
        let mut queue = match state.link_queue.lock() {
            Ok(queue) => queue,
            Err(_) => break,
        };
        queue.active_job_ids.remove(&job.id);
        match outcome {
            Ok(result) if result.status == JobStatus::Skipped => {
                queue.skipped += 1;
                record_success(
                    &app,
                    &mut queue,
                    &job,
                    result.output_path,
                    true,
                );
            }
            Ok(result) => {
                queue.completed += 1;
                record_success(
                    &app,
                    &mut queue,
                    &job,
                    result.output_path,
                    false,
                );
            }
            Err(AppError::ConversionCancelled) => {
                queue.cancelled += 1;
                let _ = append_history(
                    &app,
                    history_record(&job, "cancelled", None, Some("cancelled".to_string())),
                );
                emit_download(
                    &app,
                    LinkDownloadEvent {
                        job_id: job.id.clone(),
                        status: JobStatus::Cancelled,
                        percent: None,
                        message: "Download cancelled".to_string(),
                        output_path: None,
                        error: None,
                    },
                );
            }
            Err(error) => {
                queue.failed += 1;
                let message = error.to_string();
                let category = classify_app_error_message(&message).as_str().to_string();
                let _ = append_history(
                    &app,
                    history_record(&job, "failed", None, Some(category)),
                );
                emit_download(
                    &app,
                    LinkDownloadEvent {
                        job_id: job.id.clone(),
                        status: JobStatus::Failed,
                        percent: None,
                        message: "Download failed".to_string(),
                        output_path: None,
                        error: Some(message),
                    },
                );
            }
        }
        if queue.cancel_remaining.load(Ordering::SeqCst) {
            drain_cancelled(&app, &mut queue);
            finish_if_idle(
                &app,
                &mut queue,
                LinkBatchStatus::Cancelled,
                "Links batch cancelled.",
            );
        } else if let Some(event) = snapshot(&queue, LinkBatchStatus::Running, None, None) {
            emit_batch(&app, event);
        }
    }
}

fn record_success(
    app: &AppHandle,
    queue: &mut LinkQueueState,
    job: &LinkDownloadJob,
    output_path: String,
    skipped: bool,
) {
    let status = if skipped {
        JobStatus::Skipped
    } else {
        JobStatus::Completed
    };
    let message = if queue.package_zip {
        if skipped {
            "Ready for zip package".to_string()
        } else {
            "Downloaded — waiting to zip".to_string()
        }
    } else if skipped {
        "Existing output left unchanged".to_string()
    } else {
        "Download completed".to_string()
    };

    if queue.package_zip {
        queue.completed_pack_items.push(CompletedPackItem {
            job_id: job.id.clone(),
            title: job.title.clone(),
            url: Some(job.url.clone()),
            output_path: output_path.clone(),
            skipped,
        });
    } else {
        let _ = append_history(
            app,
            history_record(
                job,
                if skipped { "skipped" } else { "completed" },
                Some(output_path.clone()),
                None,
            ),
        );
    }

    emit_download(
        app,
        LinkDownloadEvent {
            job_id: job.id.clone(),
            status,
            percent: Some(100.0),
            message,
            output_path: Some(output_path),
            error: None,
        },
    );
}

fn callbacks_for(app: &AppHandle, job_id: &str) -> LinkRunCallbacks {
    let status_app = app.clone();
    let status_id = job_id.to_string();
    let progress_app = app.clone();
    let progress_id = job_id.to_string();
    LinkRunCallbacks {
        on_status: Arc::new(move |status, message| {
            emit_download(
                &status_app,
                LinkDownloadEvent {
                    job_id: status_id.clone(),
                    status,
                    percent: None,
                    message: message.to_string(),
                    output_path: None,
                    error: None,
                },
            )
        }),
        on_progress: Arc::new(move |percent| {
            emit_download(
                &progress_app,
                LinkDownloadEvent {
                    job_id: progress_id.clone(),
                    status: JobStatus::Converting,
                    percent,
                    message: "Downloading media".to_string(),
                    output_path: None,
                    error: None,
                },
            )
        }),
    }
}

fn history_record(
    job: &LinkDownloadJob,
    status: &str,
    output_path: Option<String>,
    error_category: Option<String>,
) -> LinkHistoryRecord {
    LinkHistoryRecord {
        job_id: job.id.clone(),
        service: None,
        title: job.title.clone(),
        status: status.to_string(),
        output_path,
        error_category,
        url: Some(job.url.clone()),
    }
}

fn drain_cancelled(app: &AppHandle, queue: &mut LinkQueueState) {
    while let Some(job) = queue.items.pop_front() {
        queue.cancelled += 1;
        emit_download(
            app,
            LinkDownloadEvent {
                job_id: job.id.clone(),
                status: JobStatus::Cancelled,
                percent: None,
                message: "Cancelled with queue".to_string(),
                output_path: None,
                error: None,
            },
        );
        let _ = append_history(
            app,
            history_record(&job, "cancelled", None, Some("cancelled".to_string())),
        );
    }
}

fn finish_if_idle(
    app: &AppHandle,
    queue: &mut LinkQueueState,
    status: LinkBatchStatus,
    message: &str,
) {
    if !(queue.active_job_ids.is_empty() && queue.worker_running) {
        return;
    }

    let mut message = message.to_string();
    let mut zip_path = None;

    if status == LinkBatchStatus::Completed && queue.package_zip {
        if queue.completed_pack_items.is_empty() {
            if let Some(staging) = queue.staging_dir.take() {
                link_zip::remove_dir_best_effort(&staging);
            }
            message = "Links batch finished with no files to zip.".to_string();
        } else {
            let packed = queue.completed_pack_items.len();
            match finalize_zip_package(app, queue) {
                Ok(path) => {
                    message = format!(
                        "Packaged {packed} download{} into {}",
                        if packed == 1 { "" } else { "s" },
                        path.file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("archive.zip")
                    );
                    zip_path = Some(path.to_string_lossy().into_owned());
                }
                Err(error) => {
                    message = format!(
                        "Downloads finished but zip failed: {error}. Files are still in the staging folder."
                    );
                }
            }
        }
    } else if status == LinkBatchStatus::Cancelled {
        if let Some(staging) = queue.staging_dir.take() {
            link_zip::remove_dir_best_effort(&staging);
        }
        queue.completed_pack_items.clear();
    }

    queue.package_zip = false;
    queue.final_destination_dir = None;
    queue.staging_dir = None;
    queue.zip_stem = None;
    queue.worker_running = false;
    if let Some(event) = snapshot(queue, status, Some(message), zip_path) {
        emit_batch(app, event);
    }
}

fn finalize_zip_package(app: &AppHandle, queue: &mut LinkQueueState) -> Result<PathBuf, String> {
    let staging = queue
        .staging_dir
        .as_ref()
        .ok_or_else(|| "Missing Links staging folder.".to_string())?;
    let destination = queue
        .final_destination_dir
        .as_ref()
        .ok_or_else(|| "Missing Links destination folder.".to_string())?;
    let stem = queue
        .zip_stem
        .as_deref()
        .filter(|stem| !stem.is_empty())
        .unwrap_or("links-batch");

    let path = link_zip::package_staging_dir(staging, destination, stem)?;
    let zip_path_text = path.to_string_lossy().into_owned();

    for item in queue.completed_pack_items.drain(..) {
        let _ = append_history(
            app,
            LinkHistoryRecord {
                job_id: item.job_id.clone(),
                service: None,
                title: item.title,
                status: if item.skipped {
                    "skipped".to_string()
                } else {
                    "completed".to_string()
                },
                output_path: Some(zip_path_text.clone()),
                error_category: None,
                url: item.url,
            },
        );
        emit_download(
            app,
            LinkDownloadEvent {
                job_id: item.job_id,
                status: if item.skipped {
                    JobStatus::Skipped
                } else {
                    JobStatus::Completed
                },
                percent: Some(100.0),
                message: "Packaged into zip".to_string(),
                output_path: Some(zip_path_text.clone()),
                error: None,
            },
        );
    }

    link_zip::remove_dir_best_effort(staging);
    queue.staging_dir = None;
    Ok(path)
}

pub fn cancel_batch(state: &AppState) -> Result<(), String> {
    let active_ids = {
        let queue = state
            .link_queue
            .lock()
            .map_err(|_| "Internal Links queue lock error.".to_string())?;
        if !queue.worker_running && queue.items.is_empty() && queue.active_job_ids.is_empty() {
            return Err("No active Links batch to cancel.".to_string());
        }
        queue.cancel_remaining.store(true, Ordering::SeqCst);
        queue.active_job_ids.iter().cloned().collect::<Vec<_>>()
    };
    for job_id in active_ids {
        let _ = state.request_cancel(&job_id);
    }
    Ok(())
}

pub enum CancelJobResult {
    ActiveCancelled,
    QueuedRemoved,
}

pub fn cancel_job(state: &AppState, job_id: &str) -> Result<CancelJobResult, String> {
    if state.request_cancel(job_id) {
        return Ok(CancelJobResult::ActiveCancelled);
    }

    let mut queue = state
        .link_queue
        .lock()
        .map_err(|_| "Internal Links queue lock error.".to_string())?;
    let before = queue.items.len();
    queue.items.retain(|job| job.id != job_id);
    if queue.items.len() < before {
        queue.cancelled += 1;
        return Ok(CancelJobResult::QueuedRemoved);
    }
    Err(format!("No active link download found for job {job_id}."))
}

pub fn is_batch_running(state: &AppState) -> bool {
    state
        .link_queue
        .lock()
        .map(|queue| queue.worker_running)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_queue_uses_two_workers_and_snapshot_counts_remaining_items() {
        let mut queue = LinkQueueState::default();
        queue.batch_id = Some("batch".into());
        queue.items.push_back(crate::engine::link_job::LinkDownloadJob {
            id: "job".into(),
            url: "https://example.com/a".into(),
            title: None,
            duration_seconds: None,
            is_live: false,
            is_playlist: false,
            destination_dir: ".".into(),
            overwrite_policy: crate::engine::job::OverwritePolicy::Rename,
            mode: crate::engine::link_job::LinkMediaMode::Video,
            video_quality: crate::engine::link_job::LinkVideoQuality::Best,
            audio_format: crate::engine::link_job::LinkAudioFormat::Original,
            quality_preset: crate::engine::job::QualityPreset::Medium,
            mp3_encoding_mode: crate::engine::job::Mp3EncodingMode::Cbr,
            bit_depth_preset: crate::engine::job::BitDepthPreset::Original,
            cookies_path: None,
            download_subtitles: false,
            save_thumbnail: false,
            embed_thumbnail: false,
            live_max_minutes: None,
            status: JobStatus::Queued,
        });
        let event = snapshot(&queue, LinkBatchStatus::Running, None, None).expect("snapshot");
        assert_eq!(LINK_QUEUE_PARALLELISM, 2);
        assert_eq!(event.parallelism, 2);
        assert_eq!(event.remaining, 1);
        assert!(event.zip_path.is_none());
    }
}
