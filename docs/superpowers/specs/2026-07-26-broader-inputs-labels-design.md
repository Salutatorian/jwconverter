# Broader inputs + format labels (E2) — design

**Version:** v0.1.10  
**Status:** Approved (option B)

## Goal

Clearer output labels, separate M4A (AAC) from raw AAC (ADTS), and a wider input extension registry for folder scan / file filters.

## Output formats

| UI label | Enum | Extension | Notes |
|---|---|---|---|
| M4A (AAC) | `m4a` | `.m4a` | Former `aac` behavior |
| AAC (ADTS) | `aac` | `.aac` | Raw ADTS stream |
| ALAC (M4A) | `alac` | `.m4a` | Label only |
| OGG (Vorbis) | `ogg` | `.ogg` | Label only |
| Opus | `opus` | `.opus` | unchanged |
| … | | | FLAC, WAV, MP3, AIFF unchanged |

Cover art: supported for `m4a` / `alac`; not for raw `aac`.

## Inputs

Widen shared extension allowlists (TS + Rust discover). Still extension-gated for folders; FFprobe rejects non-audio. No “All files” picker in E2.

## Non-goals

E3 VBR; E4 licensing; open-any-file dialog.
