//! Conversion queue with a capped parallel worker pool.
//! Jobs still reuse the same safe `run_job` lifecycle.

use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::engine::job::{ConversionJob, JobStatus};
use crate::engine::runner::{self, RunCallbacks};
use crate::errors::AppError;
use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct QueueItem {
    pub job: ConversionJob,
    pub source_duration_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchEvent {
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
    pub status: BatchStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BatchStatus {
    Running,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionEvent {
    pub job_id: String,
    pub source_path: Option<String>,
    pub status: JobStatus,
    pub percent: Option<f64>,
    pub message: Option<String>,
    pub output_path: Option<String>,
}

pub struct QueueState {
    pub items: VecDeque<QueueItem>,
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

impl Default for QueueState {
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
            parallelism: 1,
        }
    }
}

fn emit_conversion(app: &AppHandle, event: ConversionEvent) {
    let _ = app.emit("conversion-event", event);
}

fn emit_batch(app: &AppHandle, event: BatchEvent) {
    let _ = app.emit("batch-event", event);
}

fn snapshot_batch(
    queue: &QueueState,
    status: BatchStatus,
    message: Option<String>,
) -> Option<BatchEvent> {
    let batch_id = queue.batch_id.clone()?;
    Some(BatchEvent {
        batch_id,
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

/// Enqueue jobs and start a sequential worker (one FFmpeg at a time).
pub fn enqueue_batch(
    app: AppHandle,
    state: &AppState,
    items: Vec<QueueItem>,
) -> Result<(String, Vec<String>), String> {
    if items.is_empty() {
        return Err("No files were provided for conversion.".to_string());
    }

    let parallelism = 1;
    let batch_id = uuid::Uuid::new_v4().to_string();
    let job_ids: Vec<String> = items.iter().map(|item| item.job.id.clone()).collect();

    {
        let mut queue = state
            .queue
            .lock()
            .map_err(|_| "Internal queue lock error.".to_string())?;

        if queue.worker_running {
            return Err("A conversion batch is already running. Cancel it first.".to_string());
        }

        queue.items.clear();
        queue.cancel_remaining = Arc::new(AtomicBool::new(false));
        queue.batch_id = Some(batch_id.clone());
        queue.total = items.len() as u32;
        queue.completed = 0;
        queue.failed = 0;
        queue.cancelled = 0;
        queue.skipped = 0;
        queue.active_job_ids.clear();
        queue.parallelism = parallelism;
        queue.worker_running = true;

        for item in &items {
            emit_conversion(
                &app,
                ConversionEvent {
                    job_id: item.job.id.clone(),
                    source_path: Some(item.job.source_path.clone()),
                    status: JobStatus::Queued,
                    percent: Some(0.0),
                    message: None,
                    output_path: None,
                },
            );
        }
        for item in items {
            queue.items.push_back(item);
        }

        if let Some(event) = snapshot_batch(
            &queue,
            BatchStatus::Running,
            Some("Batch started.".to_string()),
        ) {
            emit_batch(&app, event);
        }
    }

    spawn_workers(app, parallelism);
    Ok((batch_id, job_ids))
}

fn spawn_workers(app: AppHandle, parallelism: usize) {
    for _ in 0..parallelism {
        let app = app.clone();
        std::thread::spawn(move || worker_loop(app));
    }
}

fn worker_loop(app: AppHandle) {
    loop {
        let Some(app_state) = app.try_state::<AppState>() else {
            break;
        };

        let next = {
            let mut queue = match app_state.queue.lock() {
                Ok(q) => q,
                Err(_) => break,
            };

            if !queue.worker_running {
                break;
            }

            if queue.cancel_remaining.load(Ordering::SeqCst) {
                drain_cancelled(&app, &mut queue);
                if queue.active_job_ids.is_empty() {
                    queue.worker_running = false;
                    if let Some(event) = snapshot_batch(
                        &queue,
                        BatchStatus::Cancelled,
                        Some("Batch cancelled.".to_string()),
                    ) {
                        emit_batch(&app, event);
                    }
                }
                break;
            }

            match queue.items.pop_front() {
                Some(item) => {
                    queue.active_job_ids.insert(item.job.id.clone());
                    if let Some(event) = snapshot_batch(&queue, BatchStatus::Running, None) {
                        emit_batch(&app, event);
                    }
                    Some(item)
                }
                None => {
                    if !queue.active_job_ids.is_empty() {
                        // Other workers still busy.
                        None
                    } else if queue.worker_running {
                        queue.worker_running = false;
                        let status = if queue.cancelled > 0
                            && queue.completed == 0
                            && queue.failed == 0
                            && queue.skipped == 0
                        {
                            BatchStatus::Cancelled
                        } else {
                            BatchStatus::Completed
                        };
                        if let Some(event) =
                            snapshot_batch(&queue, status, Some("Batch finished.".to_string()))
                        {
                            emit_batch(&app, event);
                        }
                        break;
                    } else {
                        // Another worker already finished the batch.
                        break;
                    }
                }
            }
        };

        let Some(next) = next else {
            std::thread::sleep(std::time::Duration::from_millis(40));
            continue;
        };

        let active = app_state.register(next.job.id.clone());

        let app_status = app.clone();
        let job_status_id = next.job.id.clone();
        let source_for_status = next.job.source_path.clone();
        let on_status: Arc<dyn Fn(JobStatus) + Send + Sync> = Arc::new(move |status| {
            emit_conversion(
                &app_status,
                ConversionEvent {
                    job_id: job_status_id.clone(),
                    source_path: Some(source_for_status.clone()),
                    status,
                    percent: None,
                    message: None,
                    output_path: None,
                },
            );
        });

        let app_progress = app.clone();
        let job_progress_id = next.job.id.clone();
        let source_for_progress = next.job.source_path.clone();
        let on_progress: Arc<dyn Fn(Option<f64>) + Send + Sync> = Arc::new(move |percent| {
            emit_conversion(
                &app_progress,
                ConversionEvent {
                    job_id: job_progress_id.clone(),
                    source_path: Some(source_for_progress.clone()),
                    status: JobStatus::Converting,
                    percent,
                    message: None,
                    output_path: None,
                },
            );
        });

        let result = runner::run_job(
            &next.job,
            next.source_duration_seconds,
            &active,
            &RunCallbacks {
                on_status,
                on_progress,
            },
        );

        app_state.remove(&next.job.id);

        {
            let mut queue = match app_state.queue.lock() {
                Ok(q) => q,
                Err(_) => break,
            };
            queue.active_job_ids.remove(&next.job.id);

            match result {
                Ok(done) => {
                    if done.status == JobStatus::Skipped {
                        queue.skipped += 1;
                        emit_conversion(
                            &app,
                            ConversionEvent {
                                job_id: next.job.id,
                                source_path: Some(next.job.source_path),
                                status: JobStatus::Skipped,
                                percent: Some(100.0),
                                message: Some("Skipped — output already exists.".to_string()),
                                output_path: Some(done.output_path),
                            },
                        );
                    } else {
                        queue.completed += 1;
                        emit_conversion(
                            &app,
                            ConversionEvent {
                                job_id: next.job.id,
                                source_path: Some(next.job.source_path),
                                status: JobStatus::Completed,
                                percent: Some(100.0),
                                message: Some("Conversion completed.".to_string()),
                                output_path: Some(done.output_path),
                            },
                        );
                    }
                }
                Err(AppError::ConversionCancelled) => {
                    queue.cancelled += 1;
                    emit_conversion(
                        &app,
                        ConversionEvent {
                            job_id: next.job.id,
                            source_path: Some(next.job.source_path),
                            status: JobStatus::Cancelled,
                            percent: None,
                            message: Some("Conversion cancelled.".to_string()),
                            output_path: None,
                        },
                    );
                }
                Err(error) => {
                    queue.failed += 1;
                    emit_conversion(
                        &app,
                        ConversionEvent {
                            job_id: next.job.id,
                            source_path: Some(next.job.source_path),
                            status: JobStatus::Failed,
                            percent: None,
                            message: Some(error.to_string()),
                            output_path: None,
                        },
                    );
                }
            }

            if queue.cancel_remaining.load(Ordering::SeqCst) {
                drain_cancelled(&app, &mut queue);
                if queue.active_job_ids.is_empty() {
                    queue.worker_running = false;
                    if let Some(event) = snapshot_batch(
                        &queue,
                        BatchStatus::Cancelled,
                        Some("Batch cancelled.".to_string()),
                    ) {
                        emit_batch(&app, event);
                    }
                }
            } else if let Some(event) = snapshot_batch(&queue, BatchStatus::Running, None) {
                emit_batch(&app, event);
            }
        }
    }
}

fn drain_cancelled(app: &AppHandle, queue: &mut QueueState) {
    while let Some(item) = queue.items.pop_front() {
        queue.cancelled += 1;
        emit_conversion(
            app,
            ConversionEvent {
                job_id: item.job.id,
                source_path: Some(item.job.source_path),
                status: JobStatus::Cancelled,
                percent: None,
                message: Some("Cancelled with queue.".to_string()),
                output_path: None,
            },
        );
    }
}

/// Cancel all active jobs and remaining queued jobs.
pub fn cancel_queue(state: &AppState) -> Result<(), String> {
    let active_ids = {
        let queue = state
            .queue
            .lock()
            .map_err(|_| "Internal queue lock error.".to_string())?;

        if !queue.worker_running && queue.items.is_empty() && queue.active_job_ids.is_empty() {
            return Err("No active batch to cancel.".to_string());
        }

        queue.cancel_remaining.store(true, Ordering::SeqCst);
        queue.active_job_ids.iter().cloned().collect::<Vec<_>>()
    };

    for job_id in active_ids {
        let _ = state.request_cancel(&job_id);
    }

    Ok(())
}

/// Cancel one running job; other parallel jobs and the queue continue.
pub fn cancel_current_job(state: &AppState) -> Result<(), String> {
    let job_id = {
        let queue = state
            .queue
            .lock()
            .map_err(|_| "Internal queue lock error.".to_string())?;
        queue.active_job_ids.iter().next().cloned()
    };

    let Some(job_id) = job_id else {
        return Err("No conversion is currently running.".to_string());
    };

    if state.request_cancel(&job_id) {
        Ok(())
    } else {
        Err("Could not cancel the current conversion.".to_string())
    }
}

pub fn is_batch_running(state: &AppState) -> bool {
    state
        .queue
        .lock()
        .map(|q| q.worker_running)
        .unwrap_or(false)
}
