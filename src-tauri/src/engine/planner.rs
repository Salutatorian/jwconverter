//! Turns a conversion request into an encoder plan.
//! Owns format → codec / container / quality decisions. No process spawning here.

use super::job::{BitDepthPreset, OutputFormat, QualityPreset};

#[derive(Debug, Clone)]
pub struct SourcePcmHints {
    pub sample_format: Option<String>,
    pub bits_per_raw_sample: Option<u32>,
    pub bit_depth: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct EncoderPlan {
    pub format: OutputFormat,
    pub quality: QualityPreset,
    pub bit_depth: BitDepthPreset,
    pub container: &'static str,
    pub audio_codec: String,
    pub extension: &'static str,
}

impl EncoderPlan {
    pub fn ffmpeg_audio_args(&self) -> Vec<String> {
        match self.format {
            OutputFormat::Wav | OutputFormat::Aiff => {
                vec!["-c:a".to_string(), self.audio_codec.clone()]
            }
            OutputFormat::Flac => vec!["-c:a".to_string(), "flac".to_string()],
            OutputFormat::Alac => vec!["-c:a".to_string(), "alac".to_string()],
            OutputFormat::Mp3 => {
                let bitrate = match self.quality {
                    QualityPreset::Low => "128k",
                    QualityPreset::Medium => "192k",
                    QualityPreset::High => "320k",
                };
                vec![
                    "-c:a".to_string(),
                    "libmp3lame".to_string(),
                    "-b:a".to_string(),
                    bitrate.to_string(),
                ]
            }
            OutputFormat::M4a | OutputFormat::Aac => {
                let bitrate = match self.quality {
                    QualityPreset::Low => "128k",
                    QualityPreset::Medium => "192k",
                    QualityPreset::High => "256k",
                };
                let mut args = vec![
                    "-c:a".to_string(),
                    "aac".to_string(),
                    "-b:a".to_string(),
                    bitrate.to_string(),
                ];
                if matches!(self.format, OutputFormat::Aac) {
                    args.insert(0, "-f".to_string());
                    args.insert(1, "adts".to_string());
                }
                args
            }
            OutputFormat::Opus => {
                let bitrate = match self.quality {
                    QualityPreset::Low => "96k",
                    QualityPreset::Medium => "160k",
                    QualityPreset::High => "192k",
                };
                vec![
                    "-c:a".to_string(),
                    "libopus".to_string(),
                    "-b:a".to_string(),
                    bitrate.to_string(),
                ]
            }
            OutputFormat::Ogg => {
                let q = match self.quality {
                    QualityPreset::Low => "3",
                    QualityPreset::Medium => "5",
                    QualityPreset::High => "7",
                };
                vec![
                    "-c:a".to_string(),
                    "libvorbis".to_string(),
                    "-q:a".to_string(),
                    q.to_string(),
                ]
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum PcmEndian {
    Little,
    Big,
}

pub fn plan_for(
    format: OutputFormat,
    quality: QualityPreset,
    bit_depth: BitDepthPreset,
    source: Option<&SourcePcmHints>,
) -> EncoderPlan {
    let quality = if format.is_lossy() {
        quality
    } else {
        QualityPreset::Medium
    };
    let bit_depth = if matches!(format, OutputFormat::Wav | OutputFormat::Aiff) {
        bit_depth
    } else {
        BitDepthPreset::Original
    };

    match format {
        OutputFormat::Wav => {
            let codec = pcm_codec(PcmEndian::Little, bit_depth, source);
            EncoderPlan {
                format,
                quality,
                bit_depth,
                container: "wav",
                audio_codec: codec.to_string(),
                extension: "wav",
            }
        }
        OutputFormat::Aiff => {
            let codec = pcm_codec(PcmEndian::Big, bit_depth, source);
            EncoderPlan {
                format,
                quality,
                bit_depth,
                container: "aiff",
                audio_codec: codec.to_string(),
                extension: "aiff",
            }
        }
        OutputFormat::Flac => EncoderPlan {
            format,
            quality,
            bit_depth,
            container: "flac",
            audio_codec: "flac".to_string(),
            extension: "flac",
        },
        OutputFormat::Mp3 => EncoderPlan {
            format,
            quality,
            bit_depth,
            container: "mp3",
            audio_codec: "libmp3lame".to_string(),
            extension: "mp3",
        },
        OutputFormat::M4a => EncoderPlan {
            format,
            quality,
            bit_depth,
            container: "m4a",
            audio_codec: "aac".to_string(),
            extension: "m4a",
        },
        OutputFormat::Aac => EncoderPlan {
            format,
            quality,
            bit_depth,
            container: "adts",
            audio_codec: "aac".to_string(),
            extension: "aac",
        },
        OutputFormat::Opus => EncoderPlan {
            format,
            quality,
            bit_depth,
            container: "opus",
            audio_codec: "libopus".to_string(),
            extension: "opus",
        },
        OutputFormat::Ogg => EncoderPlan {
            format,
            quality,
            bit_depth,
            container: "ogg",
            audio_codec: "libvorbis".to_string(),
            extension: "ogg",
        },
        OutputFormat::Alac => EncoderPlan {
            format,
            quality,
            bit_depth,
            container: "m4a",
            audio_codec: "alac".to_string(),
            extension: "m4a",
        },
    }
}

fn pcm_codec(
    endian: PcmEndian,
    bit_depth: BitDepthPreset,
    source: Option<&SourcePcmHints>,
) -> &'static str {
    let kind = match bit_depth {
        BitDepthPreset::Bit16 => PcmKind::S16,
        BitDepthPreset::Bit24 => PcmKind::S24,
        BitDepthPreset::Float32 => PcmKind::F32,
        BitDepthPreset::Original => infer_pcm_kind(source),
    };
    match (endian, kind) {
        (PcmEndian::Little, PcmKind::U8) => "pcm_u8",
        (PcmEndian::Big, PcmKind::U8) => "pcm_u8",
        (PcmEndian::Little, PcmKind::S16) => "pcm_s16le",
        (PcmEndian::Big, PcmKind::S16) => "pcm_s16be",
        (PcmEndian::Little, PcmKind::S24) => "pcm_s24le",
        (PcmEndian::Big, PcmKind::S24) => "pcm_s24be",
        (PcmEndian::Little, PcmKind::S32) => "pcm_s32le",
        (PcmEndian::Big, PcmKind::S32) => "pcm_s32be",
        (PcmEndian::Little, PcmKind::F32) => "pcm_f32le",
        (PcmEndian::Big, PcmKind::F32) => "pcm_f32be",
        (PcmEndian::Little, PcmKind::F64) => "pcm_f64le",
        (PcmEndian::Big, PcmKind::F64) => "pcm_f64be",
    }
}

#[derive(Debug, Clone, Copy)]
enum PcmKind {
    U8,
    S16,
    S24,
    S32,
    F32,
    F64,
}

fn infer_pcm_kind(source: Option<&SourcePcmHints>) -> PcmKind {
    let Some(source) = source else {
        return PcmKind::S24;
    };

    if let Some(bits) = source.bits_per_raw_sample.or(source.bit_depth) {
        return match bits {
            0..=8 => PcmKind::U8,
            9..=16 => PcmKind::S16,
            17..=24 => PcmKind::S24,
            25..=32 => PcmKind::S32,
            _ => PcmKind::S32,
        };
    }

    let fmt = source
        .sample_format
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    if fmt.contains("f64") || fmt.contains("dbl") {
        PcmKind::F64
    } else if fmt.contains("f32") || fmt.contains("flt") {
        PcmKind::F32
    } else if fmt.contains("s32") {
        PcmKind::S32
    } else if fmt.contains("s24") {
        PcmKind::S24
    } else if fmt.contains("s16") {
        PcmKind::S16
    } else if fmt.contains("u8") {
        PcmKind::U8
    } else {
        // Prefer 24-bit over forced 16-bit when source is unknown (studio masters).
        PcmKind::S24
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medium_mp3_matches_legacy_default() {
        let plan = plan_for(OutputFormat::Mp3, QualityPreset::Medium, BitDepthPreset::Original, None);
        assert_eq!(
            plan.ffmpeg_audio_args(),
            vec!["-c:a", "libmp3lame", "-b:a", "192k"]
        );
    }

    #[test]
    fn wav_default_original_uses_source_24bit() {
        let hints = SourcePcmHints {
            sample_format: Some("s32".to_string()),
            bits_per_raw_sample: Some(24),
            bit_depth: Some(24),
        };
        let plan = plan_for(
            OutputFormat::Wav,
            QualityPreset::Medium,
            BitDepthPreset::Original,
            Some(&hints),
        );
        assert_eq!(plan.audio_codec, "pcm_s24le");
    }

    #[test]
    fn wav_forced_16_overrides_source() {
        let hints = SourcePcmHints {
            sample_format: Some("s32".to_string()),
            bits_per_raw_sample: Some(24),
            bit_depth: Some(24),
        };
        let plan = plan_for(
            OutputFormat::Wav,
            QualityPreset::Medium,
            BitDepthPreset::Bit16,
            Some(&hints),
        );
        assert_eq!(plan.audio_codec, "pcm_s16le");
    }

    #[test]
    fn aiff_float_is_big_endian() {
        let plan = plan_for(
            OutputFormat::Aiff,
            QualityPreset::Medium,
            BitDepthPreset::Float32,
            None,
        );
        assert_eq!(plan.audio_codec, "pcm_f32be");
    }

    #[test]
    fn medium_m4a_uses_aac_in_m4a_container() {
        let plan = plan_for(
            OutputFormat::M4a,
            QualityPreset::Medium,
            BitDepthPreset::Original,
            None,
        );
        assert_eq!(plan.extension, "m4a");
        assert_eq!(
            plan.ffmpeg_audio_args(),
            vec!["-c:a", "aac", "-b:a", "192k"]
        );
    }

    #[test]
    fn raw_aac_forces_adts() {
        let plan = plan_for(
            OutputFormat::Aac,
            QualityPreset::Medium,
            BitDepthPreset::Original,
            None,
        );
        assert_eq!(plan.extension, "aac");
        assert_eq!(
            plan.ffmpeg_audio_args(),
            vec!["-f", "adts", "-c:a", "aac", "-b:a", "192k"]
        );
    }

    #[test]
    fn lossless_ignores_quality_in_args() {
        let low = plan_for(OutputFormat::Flac, QualityPreset::Low, BitDepthPreset::Original, None);
        let high = plan_for(OutputFormat::Flac, QualityPreset::High, BitDepthPreset::Original, None);
        assert_eq!(low.ffmpeg_audio_args(), high.ffmpeg_audio_args());
    }
}
