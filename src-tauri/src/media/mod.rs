//! FFmpeg / FFprobe integration.
//! Process spawning must use argument arrays — never shell strings.

pub mod ffmpeg;
pub mod ffprobe;
pub mod imagemagick;
pub mod magick_policy;
pub mod paths;
pub mod progress;
