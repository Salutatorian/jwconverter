# More Output Formats Design

**Date:** 2026-07-26  
**Phase:** `formats`  
**Status:** Approved

## Goal

Add common output formats with fixed sensible encoder defaults. Quality presets remain a later phase.

## Formats

| UI label | `OutputFormat` | Extension | Codec / notes |
|---|---|---|---|
| FLAC | `flac` | `.flac` | existing |
| WAV | `wav` | `.wav` | existing |
| MP3 | `mp3` | `.mp3` | existing, 192k |
| AAC / M4A | `aac` | `.m4a` | `aac` @ 192k, MP4/M4A container |
| Opus | `opus` | `.opus` | `libopus` @ 160k |
| OGG | `ogg` | `.ogg` | `libvorbis` ~q5 |
| ALAC | `alac` | `.m4a` | `alac` lossless |
| AIFF | `aiff` | `.aiff` | `pcm_s16be` |

## Locked decisions

- AAC writes `.m4a` only (not raw `.aac`)
- Fixed quality defaults; no quality picker this phase
- ALAC and AAC both use `.m4a`; labels distinguish them in the UI
- Overwrite policy still applies to the resolved final path

## Non-goals

- Quality / bitrate presets
- Separate raw ADTS `.aac` option
- Video or image formats
- Changing import/discovery extensions (already broad)

## Implementation surface

1. Rust `OutputFormat` enum + serde
2. `planner::plan_for` + `ffmpeg_audio_args`
3. `verify::codec_matches` for new codecs (ffprobe names)
4. Frontend `OutputFormat`, `OUTPUT_FORMATS`, FormatPicker
5. Phase string → `formats`
6. Tests: at least one convert per new format from the WAV fixture (or parameterized)

## Encoder defaults (argv arrays only)

- AAC: `-c:a aac -b:a 192k` (container m4a)
- Opus: `-c:a libopus -b:a 160k`
- OGG: `-c:a libvorbis -q:a 5`
- ALAC: `-c:a alac`
- AIFF: `-c:a pcm_s16be`

## Verify codec expectations

| Format | Accept ffprobe codec |
|---|---|
| aac | `aac` |
| opus | `opus` |
| ogg | `vorbis` |
| alac | `alac` |
| aiff | `pcm_s16be` or `pcm_*` |

## Success criteria

- All eight formats selectable and convert the tone fixture successfully
- Typecheck + cargo tests pass
- README lists new formats; phase `formats`
