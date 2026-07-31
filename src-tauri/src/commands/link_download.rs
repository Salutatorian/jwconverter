use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::engine::job::{JobStatus, OverwritePolicy};
use crate::engine::link_job::{LinkAudioFormat, LinkDownloadJob, LinkMediaMode, LinkVideoQuality};
use crate::engine::link_runner::{self, LinkRunCallbacks};
use crate::media::ytdlp;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkDownloadRequest {
    pub url: String,
    pub destination_dir: String,
    #[serde(default)]
    pub overwrite_policy: OverwritePolicy,
    #[serde(default)]
    pub mode: LinkMediaMode,
    #[serde(default)]
    pub video_quality: LinkVideoQuality,
    #[serde(default)]
    pub audio_format: LinkAudioFormat,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LinkDownloadEvent {
    job_id: String,
    status: JobStatus,
    percent: Option<f64>,
    message: String,
    output_path: Option<String>,
    error: Option<String>,
}

fn emit_event(app: &AppHandle, event: LinkDownloadEvent) {
    let _ = app.emit("link-download-event", event);
}

fn build_job(request: LinkDownloadRequest) -> Result<LinkDownloadJob, String> {
    let url = request.url.trim().to_string();
    let destination_dir = request.destination_dir.trim().to_string();
    if url.is_empty() {
        return Err("Paste a public media URL first.".to_string());
    }
    if destination_dir.is_empty() {
        return Err("Choose a destination folder first.".to_string());
    }

    let info = ytdlp::inspect(&url).map_err(|error| error.to_string())?;
    Ok(LinkDownloadJob {
        id: uuid::Uuid::new_v4().to_string(),
        url,
        title: info.title,
        is_live: info.is_live,
        is_playlist: info.is_playlist,
        destination_dir,
        overwrite_policy: request.overwrite_policy,
        mode: request.mode,
        video_quality: request.video_quality,
        audio_format: request.audio_format,
        status: JobStatus::Queued,
    })
}

#[tauri::command]
pub fn start_link_download(
    app: AppHandle,
    state: State<'_, AppState>,
    request: LinkDownloadRequest,
) -> Result<String, String> {
    let job = build_job(request)?;
    let job_id = job.id.clone();
    let active = state.register(job_id.clone());
    emit_event(
        &app,
        LinkDownloadEvent {
            job_id: job_id.clone(),
            status: JobStatus::Queued,
            percent: Some(0.0),
            message: "Preparing download".to_string(),
            output_path: None,
            error: None,
        },
    );

    let thread_app = app.clone();
    let thread_job_id = job_id.clone();
    std::thread::spawn(move || {
        let callback_app = thread_app.clone();
        let callback_job_id = thread_job_id.clone();
        let callbacks = LinkRunCallbacks {
            on_status: Arc::new(move |status, message| {
                emit_event(
                    &callback_app,
                    LinkDownloadEvent {
                        job_id: callback_job_id.clone(),
                        status,
                        percent: None,
                        message: message.to_string(),
                        output_path: None,
                        error: None,
                    },
                );
            }),
            on_progress: Arc::new({
                let callback_app = thread_app.clone();
                let callback_job_id = thread_job_id.clone();
                move |percent| {
                    emit_event(
                        &callback_app,
                        LinkDownloadEvent {
                            job_id: callback_job_id.clone(),
                            status: JobStatus::Converting,
                            percent,
                            message: "Downloading media".to_string(),
                            output_path: None,
                            error: None,
                        },
                    );
                }
            }),
        };

        let outcome = link_runner::run_job(&job, &active, &callbacks);
        let final_event = match outcome {
            Ok(result) => LinkDownloadEvent {
                job_id: thread_job_id.clone(),
                status: result.status,
                percent: Some(100.0),
                message: match result.status {
                    JobStatus::Skipped => "Existing output left unchanged".to_string(),
                    _ => "Download completed".to_string(),
                },
                output_path: Some(result.output_path),
                error: None,
            },
            Err(crate::errors::AppError::ConversionCancelled) => LinkDownloadEvent {
                job_id: thread_job_id.clone(),
                status: JobStatus::Cancelled,
                percent: None,
                message: "Download cancelled".to_string(),
                output_path: None,
                error: None,
            },
            Err(error) => LinkDownloadEvent {
                job_id: thread_job_id.clone(),
                status: JobStatus::Failed,
                percent: None,
                message: "Download failed".to_string(),
                output_path: None,
                error: Some(error.to_string()),
            },
        };
        emit_event(&thread_app, final_event);
        thread_app.state::<AppState>().remove(&thread_job_id);
    });
    Ok(job_id)
}

#[tauri::command]
pub fn cancel_link_download(state: State<'_, AppState>, job_id: String) -> Result<(), String> {
    if state.request_cancel(&job_id) {
        return Ok(());
    }
    Err(format!("No active link download found for job {job_id}."))
}
