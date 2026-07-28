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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_uses_detail_for_detail_variants() {
        let variants = [
            AppError::UnsupportedFormat { detail: "d".into() },
            AppError::DecodeFailure { detail: "d".into() },
            AppError::EncoderUnavailable { detail: "d".into() },
            AppError::MediaToolMissing { detail: "d".into() },
            AppError::PermissionDenied { detail: "d".into() },
            AppError::DiskFull { detail: "d".into() },
            AppError::SourceMissing { detail: "d".into() },
            AppError::DestinationUnavailable { detail: "d".into() },
            AppError::OutputExists { detail: "d".into() },
            AppError::FfmpegFailure { detail: "d".into() },
            AppError::VerificationFailure { detail: "d".into() },
        ];
        for variant in variants {
            assert_eq!(variant.to_string(), "d");
        }
    }

    #[test]
    fn display_for_fixed_variants() {
        assert_eq!(
            AppError::NotImplemented {
                feature: "Video".into()
            }
            .to_string(),
            "Video is not implemented yet"
        );
        assert_eq!(
            AppError::ConversionCancelled.to_string(),
            "Conversion cancelled"
        );
    }

    #[test]
    fn serde_kind_tag_is_camel_case() {
        let value = serde_json::to_value(AppError::SourceMissing {
            detail: "gone".into(),
        })
        .expect("serialize");
        assert_eq!(value["kind"], "sourceMissing");
        assert_eq!(value["detail"], "gone");

        let cancelled = serde_json::to_value(AppError::ConversionCancelled).expect("serialize");
        assert_eq!(cancelled["kind"], "conversionCancelled");
    }
}
