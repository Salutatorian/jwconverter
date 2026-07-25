# Cover art + metadata preservation (C) — design

**Version:** v0.1.8  
**Status:** Approved (roadmap C after A+B)

## Goal

Keep tags and embedded cover artwork when the destination format can hold them. Defaults **on**.

## Behavior

| Control | Default | Effect |
|---|---|---|
| Preserve tags | On | `-map_metadata 0` (+ chapters); off → strip (`-1`) |
| Preserve cover | On | Map attached picture / cover stream when format supports it; else `-vn` |

## Format support (cover)

- **Yes:** MP3, FLAC, AAC/M4A, ALAC/M4A, OGG, Opus  
- **No:** WAV, AIFF (tags only if FFmpeg can write them; no cover map)

## FFmpeg shape

- Always map first audio: `-map 0:a:0`
- Cover on: `-map 0:V:0?` + `-c:v copy` + `attached_pic` disposition (no blanket `-vn`)
- Cover off / unsupported: `-vn`
- Never invent cover art; if source has none, optional map is a no-op

## Non-goals

- Editing tags, album art replacement UI, lyrics panes
- Image/photo conversion
