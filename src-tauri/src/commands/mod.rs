//! Thin IPC adapters. Commands validate inputs then call engine/media modules.
//! The UI must never construct FFmpeg arguments.

pub mod analyze;
pub mod app_info;
pub mod audio_tools;
pub mod convert;
pub mod discover;
pub mod image_convert;
pub mod image_discover;
pub mod image_preflight;
pub mod preflight;
pub mod system;
