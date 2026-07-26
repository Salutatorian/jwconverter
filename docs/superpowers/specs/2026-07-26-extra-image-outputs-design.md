# Extra image outputs (v0.2.4) — design

**Version:** v0.2.4  
**Status:** Approved  
**Date:** 2026-07-26

## Goal

Add output formats Magick can write honestly: **BMP**, **GIF** (still), **AVIF**.

## Decisions

| Output | Controls | Magick |
|---|---|---|
| BMP | none | `BMP:` |
| GIF | none | `GIF:` still frame (no invented animation) |
| AVIF | Low / Med / High | `-quality` like JPEG |
| HEIC out | **not shipped** | Build is `HEIC r--` only |

- Existing JPEG / PNG / WebP / TIFF / resize / preflight behavior stays.
- Inputs unchanged (HEIC **in** still allowed when Magick can read).
- Preflight: GIF/AVIF count as lossy for honesty; BMP estimated as large uncompressed.

## Non-goals

- Animated GIF authoring
- HEIC export
- AVIF advanced speed/tile options

## Success

- Batch convert to BMP, GIF, AVIF offline
- Typecheck + Rust tests; signed NSIS **v0.2.4**
