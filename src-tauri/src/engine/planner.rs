//! Turns a conversion request into an encoder plan.
//! Owns format → codec / container / quality decisions. No process spawning here.

use super::job::{BitDepthPreset, Mp3EncodingMode, OutputFormat, QualityPreset};

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
    pub mp3_encoding_mode: Mp3EncodingMode,
    pub container: &'static str,
    pub audio_codec: String,
    pub extension: &'static str,
    /// Audio filter chain (loudness normalization, silence trim). Empty = none.
    pub audio_filters: Vec<String>,
}

impl EncoderPlan {
    pub fn ffmpeg_audio_args(&self) -> Vec<String> {
        let mut args = self.codec_args();
        if !self.audio_filters.is_empty() {
            args.push("-af".to_string());
            args.push(self.audio_filters.join(","));
        }
        args
    }

    fn codec_args(&self) -> Vec<String> {
        match self.format {
            OutputFormat::Wav | OutputFormat::Aiff => {
                vec!["-c:a".to_string(), self.audio_codec.clone()]
            }
            OutputFormat::Flac => vec!["-c:a".to_string(), "flac".to_string()],
            OutputFormat::Alac => vec!["-c:a".to_string(), "alac".to_string()],
            OutputFormat::Mp3 => match self.mp3_encoding_mode {
                Mp3EncodingMode::Cbr => {
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
                Mp3EncodingMode::Vbr => {
                    // lame VBR: 0 = best (V0), 9 = worst. Map Low/Med/High → V5/V2/V0.
                    let q = match self.quality {
                        QualityPreset::Low => "5",
                        QualityPreset::Medium => "2",
                        QualityPreset::High => "0",
                    };
                    vec![
                        "-c:a".to_string(),
                        "libmp3lame".to_string(),
                        "-q:a".to_string(),
                        q.to_string(),
                    ]
                }
            },
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
    mp3_encoding_mode: Mp3EncodingMode,
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
    let mp3_encoding_mode = if matches!(format, OutputFormat::Mp3) {
        mp3_encoding_mode
    } else {
        Mp3EncodingMode::Cbr
    };

    match format {
        OutputFormat::Wav => {
            let codec = pcm_codec(PcmEndian::Little, bit_depth, source);
            EncoderPlan {
                format,
                quality,
                bit_depth,
                mp3_encoding_mode,
                container: "wav",
                audio_codec: codec.to_string(),
                extension: "wav",
                audio_filters: Vec::new(),
            }
        }
        OutputFormat::Aiff => {
            let codec = pcm_codec(PcmEndian::Big, bit_depth, source);
            EncoderPlan {
                format,
                quality,
                bit_depth,
                mp3_encoding_mode,
                container: "aiff",
                audio_codec: codec.to_string(),
                extension: "aiff",
                audio_filters: Vec::new(),
            }
        }
        OutputFormat::Flac => EncoderPlan {
            format,
            quality,
            bit_depth,
            mp3_encoding_mode,
            container: "flac",
            audio_codec: "flac".to_string(),
            extension: "flac",
            audio_filters: Vec::new(),
        },
        OutputFormat::Mp3 => EncoderPlan {
            format,
            quality,
            bit_depth,
            mp3_encoding_mode,
            container: "mp3",
            audio_codec: "libmp3lame".to_string(),
            extension: "mp3",
            audio_filters: Vec::new(),
        },
        OutputFormat::M4a => EncoderPlan {
            format,
            quality,
            bit_depth,
            mp3_encoding_mode,
            container: "m4a",
            audio_codec: "aac".to_string(),
            extension: "m4a",
            audio_filters: Vec::new(),
        },
        OutputFormat::Aac => EncoderPlan {
            format,
            quality,
            bit_depth,
            mp3_encoding_mode,
            container: "adts",
            audio_codec: "aac".to_string(),
            extension: "aac",
            audio_filters: Vec::new(),
        },
        OutputFormat::Opus => EncoderPlan {
            format,
            quality,
            bit_depth,
            mp3_encoding_mode,
            container: "opus",
            audio_codec: "libopus".to_string(),
            extension: "opus",
            audio_filters: Vec::new(),
        },
        OutputFormat::Ogg => EncoderPlan {
            format,
            quality,
            bit_depth,
            mp3_encoding_mode,
            container: "ogg",
            audio_codec: "libvorbis".to_string(),
            extension: "ogg",
            audio_filters: Vec::new(),
        },
        OutputFormat::Alac => EncoderPlan {
            format,
            quality,
            bit_depth,
            mp3_encoding_mode,
            container: "m4a",
            audio_codec: "alac".to_string(),
            extension: "m4a",
            audio_filters: Vec::new(),
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
        let plan = plan_for(
            OutputFormat::Mp3,
            QualityPreset::Medium,
            BitDepthPreset::Original,
            None,
            Mp3EncodingMode::Cbr,
        );
        assert_eq!(
            plan.ffmpeg_audio_args(),
            vec!["-c:a", "libmp3lame", "-b:a", "192k"]
        );
    }

    #[test]
    fn mp3_vbr_high_uses_q0() {
        let plan = plan_for(
            OutputFormat::Mp3,
            QualityPreset::High,
            BitDepthPreset::Original,
            None,
            Mp3EncodingMode::Vbr,
        );
        assert_eq!(
            plan.ffmpeg_audio_args(),
            vec!["-c:a", "libmp3lame", "-q:a", "0"]
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
            Mp3EncodingMode::Cbr,
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
            Mp3EncodingMode::Cbr,
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
            Mp3EncodingMode::Cbr,
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
            Mp3EncodingMode::Cbr,
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
            Mp3EncodingMode::Cbr,
        );
        assert_eq!(plan.extension, "aac");
        assert_eq!(
            plan.ffmpeg_audio_args(),
            vec!["-f", "adts", "-c:a", "aac", "-b:a", "192k"]
        );
    }

    #[test]
    fn lossless_ignores_quality_in_args() {
        let low = plan_for(
            OutputFormat::Flac,
            QualityPreset::Low,
            BitDepthPreset::Original,
            None,
            Mp3EncodingMode::Cbr,
        );
        let high = plan_for(
            OutputFormat::Flac,
            QualityPreset::High,
            BitDepthPreset::Original,
            None,
            Mp3EncodingMode::Cbr,
        );
        assert_eq!(low.ffmpeg_audio_args(), high.ffmpeg_audio_args());
    }

    fn plan(
        format: OutputFormat,
        quality: QualityPreset,
        bit_depth: BitDepthPreset,
        mp3_mode: Mp3EncodingMode,
    ) -> EncoderPlan {
        plan_for(format, quality, bit_depth, None, mp3_mode)
    }

    #[test]
    fn every_format_maps_to_extension_and_container() {
        let cases = [
            (OutputFormat::Wav, "wav", "wav"),
            (OutputFormat::Flac, "flac", "flac"),
            (OutputFormat::Mp3, "mp3", "mp3"),
            (OutputFormat::M4a, "m4a", "m4a"),
            (OutputFormat::Aac, "aac", "adts"),
            (OutputFormat::Opus, "opus", "opus"),
            (OutputFormat::Ogg, "ogg", "ogg"),
            (OutputFormat::Alac, "m4a", "m4a"),
            (OutputFormat::Aiff, "aiff", "aiff"),
        ];
        for (format, extension, container) in cases {
            let plan = plan(
                format,
                QualityPreset::Medium,
                BitDepthPreset::Original,
                Mp3EncodingMode::Cbr,
            );
            assert_eq!(plan.extension, extension, "{format:?} extension");
            assert_eq!(plan.container, container, "{format:?} container");
            assert!(plan.audio_filters.is_empty(), "{format:?} filters");
        }
    }

    #[test]
    fn opus_bitrates_by_quality() {
        let low = plan(OutputFormat::Opus, QualityPreset::Low, BitDepthPreset::Original, Mp3EncodingMode::Cbr);
        let medium = plan(OutputFormat::Opus, QualityPreset::Medium, BitDepthPreset::Original, Mp3EncodingMode::Cbr);
        let high = plan(OutputFormat::Opus, QualityPreset::High, BitDepthPreset::Original, Mp3EncodingMode::Cbr);
        assert_eq!(low.ffmpeg_audio_args(), vec!["-c:a", "libopus", "-b:a", "96k"]);
        assert_eq!(medium.ffmpeg_audio_args(), vec!["-c:a", "libopus", "-b:a", "160k"]);
        assert_eq!(high.ffmpeg_audio_args(), vec!["-c:a", "libopus", "-b:a", "192k"]);
    }

    #[test]
    fn ogg_quality_levels() {
        let low = plan(OutputFormat::Ogg, QualityPreset::Low, BitDepthPreset::Original, Mp3EncodingMode::Cbr);
        let high = plan(OutputFormat::Ogg, QualityPreset::High, BitDepthPreset::Original, Mp3EncodingMode::Cbr);
        assert_eq!(low.ffmpeg_audio_args(), vec!["-c:a", "libvorbis", "-q:a", "3"]);
        assert_eq!(high.ffmpeg_audio_args(), vec!["-c:a", "libvorbis", "-q:a", "7"]);
    }

    #[test]
    fn mp3_cbr_bitrates_by_quality() {
        let low = plan(OutputFormat::Mp3, QualityPreset::Low, BitDepthPreset::Original, Mp3EncodingMode::Cbr);
        let high = plan(OutputFormat::Mp3, QualityPreset::High, BitDepthPreset::Original, Mp3EncodingMode::Cbr);
        assert_eq!(low.ffmpeg_audio_args(), vec!["-c:a", "libmp3lame", "-b:a", "128k"]);
        assert_eq!(high.ffmpeg_audio_args(), vec!["-c:a", "libmp3lame", "-b:a", "320k"]);
    }

    #[test]
    fn mp3_vbr_levels() {
        let low = plan(OutputFormat::Mp3, QualityPreset::Low, BitDepthPreset::Original, Mp3EncodingMode::Vbr);
        let medium = plan(OutputFormat::Mp3, QualityPreset::Medium, BitDepthPreset::Original, Mp3EncodingMode::Vbr);
        assert_eq!(low.ffmpeg_audio_args(), vec!["-c:a", "libmp3lame", "-q:a", "5"]);
        assert_eq!(medium.ffmpeg_audio_args(), vec!["-c:a", "libmp3lame", "-q:a", "2"]);
    }

    #[test]
    fn mp3_mode_forced_cbr_for_other_formats() {
        let plan = plan(
            OutputFormat::Flac,
            QualityPreset::Medium,
            BitDepthPreset::Original,
            Mp3EncodingMode::Vbr,
        );
        assert_eq!(plan.mp3_encoding_mode, Mp3EncodingMode::Cbr);
    }

    #[test]
    fn bit_depth_forced_original_for_non_pcm_formats() {
        let plan = plan(
            OutputFormat::Mp3,
            QualityPreset::Medium,
            BitDepthPreset::Bit24,
            Mp3EncodingMode::Cbr,
        );
        assert_eq!(plan.bit_depth, BitDepthPreset::Original);
    }

    #[test]
    fn quality_forced_medium_for_lossless() {
        let plan = plan(
            OutputFormat::Alac,
            QualityPreset::High,
            BitDepthPreset::Original,
            Mp3EncodingMode::Cbr,
        );
        assert_eq!(plan.quality, QualityPreset::Medium);
    }

    #[test]
    fn wav_unknown_source_defaults_to_24bit() {
        let plan = plan(OutputFormat::Wav, QualityPreset::Medium, BitDepthPreset::Original, Mp3EncodingMode::Cbr);
        assert_eq!(plan.audio_codec, "pcm_s24le");
    }

    #[test]
    fn aiff_endianness_matches_pcm_kind() {
        let s16 = plan(OutputFormat::Aiff, QualityPreset::Medium, BitDepthPreset::Bit16, Mp3EncodingMode::Cbr);
        assert_eq!(s16.audio_codec, "pcm_s16be");
        let s24 = plan(OutputFormat::Aiff, QualityPreset::Medium, BitDepthPreset::Bit24, Mp3EncodingMode::Cbr);
        assert_eq!(s24.audio_codec, "pcm_s24be");
        let wav24 = plan(OutputFormat::Wav, QualityPreset::Medium, BitDepthPreset::Bit24, Mp3EncodingMode::Cbr);
        assert_eq!(wav24.audio_codec, "pcm_s24le");
    }

    #[test]
    fn infer_pcm_kind_from_sample_format_strings() {
        let hints = |fmt: &str| SourcePcmHints {
            sample_format: Some(fmt.to_string()),
            bits_per_raw_sample: None,
            bit_depth: None,
        };
        let plan_with = |fmt: &str| {
            plan_for(
                OutputFormat::Wav,
                QualityPreset::Medium,
                BitDepthPreset::Original,
                Some(&hints(fmt)),
                Mp3EncodingMode::Cbr,
            )
        };
        assert_eq!(plan_with("dbl").audio_codec, "pcm_f64le");
        assert_eq!(plan_with("flt").audio_codec, "pcm_f32le");
        assert_eq!(plan_with("s32").audio_codec, "pcm_s32le");
        assert_eq!(plan_with("s24").audio_codec, "pcm_s24le");
        assert_eq!(plan_with("s16").audio_codec, "pcm_s16le");
        assert_eq!(plan_with("u8").audio_codec, "pcm_u8");
        assert_eq!(plan_with("mystery").audio_codec, "pcm_s24le");
    }

    #[test]
    fn infer_pcm_kind_from_raw_bits_boundaries() {
        let hints = |bits: u32| SourcePcmHints {
            sample_format: None,
            bits_per_raw_sample: Some(bits),
            bit_depth: None,
        };
        let codec_for = |bits: u32| {
            plan_for(
                OutputFormat::Wav,
                QualityPreset::Medium,
                BitDepthPreset::Original,
                Some(&hints(bits)),
                Mp3EncodingMode::Cbr,
            )
            .audio_codec
        };
        assert_eq!(codec_for(8), "pcm_u8");
        assert_eq!(codec_for(9), "pcm_s16le");
        assert_eq!(codec_for(16), "pcm_s16le");
        assert_eq!(codec_for(17), "pcm_s24le");
        assert_eq!(codec_for(24), "pcm_s24le");
        assert_eq!(codec_for(25), "pcm_s32le");
        assert_eq!(codec_for(32), "pcm_s32le");
    }

    #[test]
    fn audio_filters_appended_as_af_argument() {
        let mut plan = plan(OutputFormat::Flac, QualityPreset::Medium, BitDepthPreset::Original, Mp3EncodingMode::Cbr);
        assert_eq!(plan.ffmpeg_audio_args(), vec!["-c:a", "flac"]);

        plan.audio_filters = vec!["loudnorm=I=-14:TP=-1:LRA=11".to_string()];
        assert_eq!(
            plan.ffmpeg_audio_args(),
            vec!["-c:a", "flac", "-af", "loudnorm=I=-14:TP=-1:LRA=11"]
        );

        plan.audio_filters.push("aselect='between(t\\,0\\,5)',asetpts=N/SR/TB".to_string());
        let args = plan.ffmpeg_audio_args();
        let af_position = args.iter().position(|arg| arg == "-af").expect("-af present");
        assert_eq!(
            args[af_position + 1],
            "loudnorm=I=-14:TP=-1:LRA=11,aselect='between(t\\,0\\,5)',asetpts=N/SR/TB"
        );
    }
}
