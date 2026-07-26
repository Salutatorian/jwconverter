# Image metadata preserve + soak (v0.2.5) — design

**Version:** v0.2.5  
**Status:** Approved (recommended path)  
**Date:** 2026-07-26

## Goal

1. Soak the image pipeline for common edge cases; fix only real failures.  
2. Ship an explicit **Preserve metadata** control (default **on**), mirroring audio tags.

## Soak findings (no blocking bugs)

- Unicode destination/source paths work with Magick argv.  
- Long filenames OK.  
- Corrupt JPEG fails identify with existing friendly decode errors.  
- Default Magick convert **keeps** comment + ICC unless `-strip`.

## Decisions

| Setting | Magick |
|---|---|
| Preserve metadata **on** (default) | no `-strip` (profiles/comments/EXIF when the destination supports them) |
| Preserve metadata **off** | `-strip` after `-auto-orient` |

- Best-effort: BMP/GIF may drop rich EXIF; still apply strip when off.  
- Orientation still always auto-applied.  
- No separate “preserve ICC only” control.

## Non-goals

- Editing metadata fields  
- Animated GIF multi-frame export  
- HEIC write

## Success

- Toggle wired end-to-end; unit test proves keep vs strip  
- Signed **v0.2.5**
