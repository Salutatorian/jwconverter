//! Thin IPC adapters. Commands validate inputs then call engine/media modules.
//! The UI must never construct FFmpeg arguments.

pub mod analyze;
pub mod app_info;
pub mod convert;
pub mod discover;
pub mod system;
