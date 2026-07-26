# PNG / WebP encoding knobs (v0.2.3) — design

**Version:** v0.2.3  
**Status:** Approved  
**Date:** 2026-07-26

## Goal

Give users honest, useful encoding controls for PNG and WebP without cluttering the Images UI.

## Decisions

| Format | Controls | Magick |
|---|---|---|
| JPEG | Low 70 / Medium 85 / High 95 | `-quality` |
| WebP | Low / Medium / High / **Lossless** | `-quality`, or `-define webp:lossless=true` |
| PNG | Fast · 90 / Balanced · 75 / Small · 50 | `-quality` (zlib) |
| TIFF | none | unchanged |

- Reuse the existing Quality chip row (Approach 1).
- Show Quality for JPEG, WebP, and PNG (not only “lossy”).
- Chip labels are format-aware.
- If the user switches away from WebP while **Lossless** is selected, fall back to **Medium**.
- Preflight size estimates: WebP lossless ≈ PNG-ish; PNG uses rough bpp by preset.

## Non-goals

- New output formats (BMP/GIF/AVIF/HEIC)
- Per-file encoding overrides
- Progressive JPEG / animated WebP / PNG filters UI

## Success

- WebP lossless round-trips via Magick define
- PNG compression presets change Magick `-quality`
- Typecheck + Rust tests; signed NSIS **v0.2.3**
