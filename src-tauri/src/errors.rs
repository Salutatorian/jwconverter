//! Typed application errors. Mapped to clear UI messages later.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AppError {
    NotImplemented { feature: String },
    UnsupportedFormat { detail: String },
    DecodeFailure { detail: String },
    EncoderUnavailable { detail: String },
    MediaToolMissing { detail: String },
    PermissionDenied { detail: String },
    DiskFull { detail: String },
    SourceMissing { detail: String },
    DestinationUnavailable { detail: String },
    OutputExists { detail: String },
    ConversionCancelled,
    FfmpegFailure { detail: String },
    VerificationFailure { detail: String },
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NotImplemented { feature } => {
                write!(f, "{feature} is not implemented yet")
            }
            AppError::UnsupportedFormat { detail }
            | AppError::DecodeFailure { detail }
            | AppError::EncoderUnavailable { detail }
            | AppError::MediaToolMissing { detail }
            | AppError::PermissionDenied { detail }
            | AppError::DiskFull { detail }
            | AppError::SourceMissing { detail }
            | AppError::DestinationUnavailable { detail }
            | AppError::OutputExists { detail }
            | AppError::FfmpegFailure { detail }
            | AppError::VerificationFailure { detail } => write!(f, "{detail}"),
            AppError::ConversionCancelled => write!(f, "Conversion cancelled"),
        }
    }
}

impl std::error::Error for AppError {}
