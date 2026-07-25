# Quality Presets Design

**Date:** 2026-07-26  
**Phase:** `quality`  
**Status:** Approved

## Goal

Add Low / Medium / High quality presets for lossy outputs. Default Medium. Lossless formats ignore the preset.

## Decisions

| Decision | Choice |
|---|---|
| UI | Low · Medium · High |
| Default | Medium |
| Visibility | Only when output format is lossy (MP3, AAC, Opus, OGG) |
| Custom bitrates | Out of scope |

## Bitrate / quality map

| Preset | MP3 | AAC | Opus | OGG |
|---|---|---|---|---|
| Low | 128k | 128k | 96k | `-q:a 3` |
| Medium | 192k | 192k | 160k | `-q:a 5` |
| High | 320k | 256k | 192k | `-q:a 7` |

Lossless (FLAC, WAV, ALAC, AIFF): no quality args change.

## Data model

- Rust: `QualityPreset { Low, Medium, High }` default Medium; on `ConversionJob` + `ConversionRequest`
- Planner: `plan_for(format, preset)` → ffmpeg args
- Frontend: `QualityPicker`; hidden when lossless selected

## Success criteria

- Medium matches prior fixed defaults for MP3/AAC/Opus/OGG
- Switching Low/High changes planner args / encoded bitrate class
- Typecheck + cargo tests pass; phase `quality`; meaningful git commit
