use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{AppHandle, Emitter, State};

use crate::engine::job::{BitDepthPreset, JobStatus, Mp3EncodingMode, OverwritePolicy, QualityPreset};
use crate::engine::link_job::{LinkAudioFormat, LinkDownloadJob, LinkMediaMode, LinkVideoQuality};
use crate::engine::link_queue;
use crate::media::link_history;
use crate::media::link_url::validate_media_url;
use crate::media::paths::{resolve_ffmpeg, resolve_ytdlp};
use crate::media::ytdlp;
use crate::state::AppState;

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkDownloadItemRequest {
    pub url: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub duration_seconds: Option<f64>,
    #[serde(default)]
    pub is_live: Option<bool>,
    #[serde(default)]
    pub job_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkBatchRequest {
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
    #[serde(default)]
    pub cookies_path: Option<String>,
    #[serde(default)]
    pub download_subtitles: bool,
    #[serde(default)]
    pub save_thumbnail: bool,
    #[serde(default)]
    pub embed_thumbnail: bool,
    #[serde(default)]
    pub live_max_minutes: Option<u32>,
    #[serde(default)]
    pub batch_title: Option<String>,
    pub items: Vec<LinkDownloadItemRequest>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueueLinkBatchResponse {
    pub batch_id: String,
    pub job_ids: Vec<String>,
}

fn check_shared_options(request: &LinkBatchRequest) -> Result<(), String> {
    let destination_dir = request.destination_dir.trim();
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

    if request
        .cookies_path
        .as_deref()
        .is_some_and(|path| !Path::new(path).is_file())
    {
        return Err("The selected cookies.txt file could not be found.".to_string());
    }
    Ok(())
}

fn build_job(request: &LinkBatchRequest, item: LinkDownloadItemRequest) -> Result<LinkDownloadJob, String> {
    let url = validate_media_url(item.url.trim())
        .map_err(|error| error.to_string())?
        .as_str()
        .to_string();
    let requires_inspect = item.title.as_deref().is_none_or(|title| title.trim().is_empty());
    let inspected = if requires_inspect {
        Some(
            ytdlp::inspect_with_options(&url, request.cookies_path.as_deref().map(Path::new))
                .map_err(|error| error.to_string())?,
        )
    } else {
        None
    };
    let title = item.title.filter(|title| !title.trim().is_empty()).or_else(|| inspected.as_ref().and_then(|info: &ytdlp::LinkMediaInfo| info.title.clone()));
    let duration_seconds = item.duration_seconds.or_else(|| inspected.as_ref().and_then(|info: &ytdlp::LinkMediaInfo| info.duration_seconds));
    let is_live = item.is_live.or_else(|| inspected.as_ref().map(|info: &ytdlp::LinkMediaInfo| info.is_live)).unwrap_or(false);
    if is_live && request.live_max_minutes.filter(|minutes| *minutes > 0).is_none() {
        return Err("Choose a live recording duration before downloading live media.".to_string());
    }
    let id = item
        .job_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    Ok(LinkDownloadJob {
        id,
        url,
        title,
        duration_seconds,
        is_live,
        is_playlist: false,
        destination_dir: request.destination_dir.trim().to_string(),
        overwrite_policy: request.overwrite_policy,
        mode: request.mode,
        video_quality: request.video_quality,
        audio_format: request.audio_format,
        quality_preset: request.quality_preset,
        mp3_encoding_mode: request.mp3_encoding_mode,
        bit_depth_preset: request.bit_depth_preset,
        cookies_path: request.cookies_path.clone().filter(|path| !path.trim().is_empty()),
        download_subtitles: request.download_subtitles,
        save_thumbnail: request.save_thumbnail,
        embed_thumbnail: request.embed_thumbnail,
        live_max_minutes: request.live_max_minutes,
        status: JobStatus::Queued,
    })
}

#[tauri::command]
pub fn start_link_download(
    app: AppHandle,
    state: State<'_, AppState>,
    request: LinkDownloadRequest,
) -> Result<String, String> {
    let batch = LinkBatchRequest {
        destination_dir: request.destination_dir,
        overwrite_policy: request.overwrite_policy,
        mode: request.mode, video_quality: request.video_quality, audio_format: request.audio_format,
        quality_preset: request.quality_preset, mp3_encoding_mode: request.mp3_encoding_mode,
        bit_depth_preset: request.bit_depth_preset, cookies_path: None, download_subtitles: false,
        save_thumbnail: false, embed_thumbnail: false, live_max_minutes: None, batch_title: None,
        items: vec![LinkDownloadItemRequest { url: request.url, title: None, duration_seconds: None, is_live: None, job_id: request.job_id }],
    };
    let response = enqueue_link_downloads(app, state, batch)?;
    response.job_ids.into_iter().next().ok_or_else(|| "No link job was created.".to_string())
}

#[tauri::command]
pub fn cancel_link_download(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
) -> Result<(), String> {
    match link_queue::cancel_job(&state, &job_id)? {
        link_queue::CancelJobResult::ActiveCancelled => Ok(()),
        link_queue::CancelJobResult::QueuedRemoved => {
            let _ = app.emit(
                "link-download-event",
                link_queue::LinkDownloadEvent {
                    job_id: job_id.clone(),
                    status: JobStatus::Cancelled,
                    percent: None,
                    message: "Cancelled before start".to_string(),
                    output_path: None,
                    error: None,
                },
            );
            Ok(())
        }
    }
}

#[tauri::command]
pub fn enqueue_link_downloads(app: AppHandle, state: State<'_, AppState>, request: LinkBatchRequest) -> Result<EnqueueLinkBatchResponse, String> {
    check_shared_options(&request)?;
    let jobs = request.items.iter().cloned().map(|item| build_job(&request, item)).collect::<Result<Vec<_>, _>>()?;
    let (batch_id, job_ids) = link_queue::enqueue_batch(app, &state, jobs, request.batch_title)?;
    Ok(EnqueueLinkBatchResponse { batch_id, job_ids })
}

#[tauri::command]
pub fn cancel_link_batch(state: State<'_, AppState>) -> Result<(), String> {
    link_queue::cancel_batch(&state)
}

#[tauri::command]
pub fn is_link_batch_running(state: State<'_, AppState>) -> bool {
    link_queue::is_batch_running(&state)
}

#[tauri::command]
pub fn list_link_history(app: AppHandle) -> Result<Vec<link_history::LinkHistoryRecord>, String> {
    link_history::list_history(&app)
}

#[tauri::command]
pub fn clear_link_history(app: AppHandle) -> Result<(), String> {
    link_history::clear_history(&app)
}
