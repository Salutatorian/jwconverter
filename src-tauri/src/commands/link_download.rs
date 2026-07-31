use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::engine::job::{BitDepthPreset, JobStatus, Mp3EncodingMode, OverwritePolicy, QualityPreset};
use crate::engine::link_job::{LinkAudioFormat, LinkDownloadJob, LinkMediaMode, LinkVideoQuality};
use crate::engine::link_runner::{self, LinkRunCallbacks};
use crate::logging;
use crate::media::link_errors::classify_app_error_message;
use crate::media::paths::{resolve_ffmpeg, resolve_ytdlp};
use crate::media::ytdlp;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkDownloadRequest {
    #[serde(default)]
    pub job_id: Option<String>,
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
    #[serde(default)]
    pub quality_preset: QualityPreset,
    #[serde(default)]
    pub mp3_encoding_mode: Mp3EncodingMode,
    #[serde(default)]
    pub bit_depth_preset: BitDepthPreset,
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

    resolve_ytdlp().map_err(|detail| detail)?;
    if resolve_ffmpeg().is_none() {
        return Err(
            "FFmpeg was not found. Links downloads need FFmpeg for media merging and extraction."
                .to_string(),
        );
    }

    let info = ytdlp::inspect(&url).map_err(|error| error.to_string())?;
    let id = request
        .job_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    Ok(LinkDownloadJob {
        id,
        url,
        title: info.title,
        duration_seconds: info.duration_seconds,
        is_live: info.is_live,
        is_playlist: info.is_playlist,
        destination_dir,
        overwrite_policy: request.overwrite_policy,
        mode: request.mode,
        video_quality: request.video_quality,
        audio_format: request.audio_format,
        quality_preset: request.quality_preset,
        mp3_encoding_mode: request.mp3_encoding_mode,
        bit_depth_preset: request.bit_depth_preset,
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
    if !state.try_begin_link_job(&job_id) {
        return Err(
            "A Links download is already in progress. Wait for it to finish or cancel it first."
                .to_string(),
        );
    }
    let active = state.register(job_id.clone());
    logging::log_link_event(
        "link_download_start",
        &format!(
            "job={} mode={:?} audio={:?} processing={:?}",
            &job_id[..8.min(job_id.len())],
            job.mode,
            job.audio_format,
            job.processing_mode()
        ),
    );

    let thread_app = app.clone();
    let thread_job_id = job_id.clone();
    std::thread::spawn(move || {
        emit_event(
            &thread_app,
            LinkDownloadEvent {
                job_id: thread_job_id.clone(),
                status: JobStatus::Queued,
                percent: Some(0.0),
                message: "Preparing download".to_string(),
                output_path: None,
                error: None,
            },
        );
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
            Ok(result) => {
                logging::log_link_event(
                    "link_download_done",
                    &format!(
                        "job={} status={:?}",
                        &thread_job_id[..8.min(thread_job_id.len())],
                        result.status
                    ),
                );
                LinkDownloadEvent {
                    job_id: thread_job_id.clone(),
                    status: result.status,
                    percent: Some(100.0),
                    message: match result.status {
                        JobStatus::Skipped => "Existing output left unchanged".to_string(),
                        _ => "Download completed".to_string(),
                    },
                    output_path: Some(result.output_path),
                    error: None,
                }
            }
            Err(crate::errors::AppError::ConversionCancelled) => {
                logging::log_link_event(
                    "link_download_cancelled",
                    &format!("job={}", &thread_job_id[..8.min(thread_job_id.len())]),
                );
                LinkDownloadEvent {
                    job_id: thread_job_id.clone(),
                    status: JobStatus::Cancelled,
                    percent: None,
                    message: "Download cancelled".to_string(),
                    output_path: None,
                    error: None,
                }
            }
            Err(error) => {
                let message = error.to_string();
                let category = classify_app_error_message(&message);
                logging::log_link_event(
                    "link_download_failed",
                    &format!(
                        "job={} category={}",
                        &thread_job_id[..8.min(thread_job_id.len())],
                        category.as_str()
                    ),
                );
                LinkDownloadEvent {
                    job_id: thread_job_id.clone(),
                    status: JobStatus::Failed,
                    percent: None,
                    message: "Download failed".to_string(),
                    output_path: None,
                    error: Some(message),
                }
            }
        };
        emit_event(&thread_app, final_event);
        thread_app.state::<AppState>().remove(&thread_job_id);
    });
    Ok(job_id)
}

#[tauri::command]
pub fn cancel_link_download(state: State<'_, AppState>, job_id: String) -> Result<(), String> {
    if state.request_cancel(&job_id) {
        logging::log_link_event(
            "link_download_cancel_requested",
            &format!("job={}", &job_id[..8.min(job_id.len())]),
        );
        return Ok(());
    }
    Err(format!("No active link download found for job {job_id}."))
}
