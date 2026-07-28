// Application modules — conversion engine is active; stubs remain for later work.
#![allow(dead_code)]

mod commands;
mod engine;
mod errors;
mod fs_safety;
mod logging;
mod media;
mod state;

use commands::analyze::{analyze_file, get_media_tools_info};
use commands::app_info::get_app_info;
use commands::audio_tools::{detect_silence, measure_loudness};
use commands::convert::{
    cancel_batch, cancel_conversion, is_batch_running, start_batch, start_conversion,
};
use commands::discover::discover_audio_paths;
use commands::image_convert::{
    analyze_image, cancel_image_batch, is_image_batch_running, start_image_batch,
};
use commands::image_discover::discover_image_paths;
use commands::image_preflight::preflight_image_batch;
use commands::preflight::preflight_batch;
use commands::system::get_default_paths;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            analyze_file,
            get_media_tools_info,
            get_default_paths,
            discover_audio_paths,
            discover_image_paths,
            analyze_image,
            measure_loudness,
            detect_silence,
            start_image_batch,
            cancel_image_batch,
            is_image_batch_running,
            preflight_image_batch,
            preflight_batch,
            start_conversion,
            start_batch,
            cancel_conversion,
            cancel_batch,
            is_batch_running
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
