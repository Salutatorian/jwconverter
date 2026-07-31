//! FFmpeg / FFprobe integration.
//! Process spawning must use argument arrays — never shell strings.

pub mod ffmpeg;
pub mod ffprobe;
pub mod imagemagick;
pub mod link_errors;
pub mod link_filename;
pub mod link_history;
pub mod link_url;
pub mod loudness;
pub mod magick_policy;
pub mod paths;
pub mod progress;
pub mod ytdlp;
