# MP3 CBR / VBR + labeled quality (E3) — design

**Version:** v0.1.11  
**Status:** Approved (option B)

## Goal

MP3 encoding mode CBR/VBR; Quality chips show concrete bitrate or VBR grade. Other lossy formats get labeled presets only.

## MP3

| Mode | Low | Medium | High |
|---|---|---|---|
| CBR | 128k | 192k | 320k |
| VBR | V5 (`-q:a 5`) | V2 (`-q:a 2`) | V0 (`-q:a 0`) |

Default mode: **CBR**. Mode control visible only for MP3.

## Labels (Quality chips)

- MP3 CBR: `Low · 128 kbps` …
- MP3 VBR: `Low · V5` …
- M4A/AAC/Opus: bitrate labels matching planner
- OGG: `Low · q3` …

## Data

`Mp3EncodingMode { Cbr, Vbr }` on job/request/preflight; ignored unless format is MP3.
