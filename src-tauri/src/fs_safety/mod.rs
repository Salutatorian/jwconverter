//! Filesystem safety: temps, atomic finalize, disk checks.
//! Source files are treated as read-only inputs during conversion.

pub mod disk;
pub mod finalize;
pub mod temp;
