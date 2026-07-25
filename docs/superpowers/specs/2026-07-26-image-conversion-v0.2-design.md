# Image conversion (v0.2.0) — design

**Version:** v0.2.0  
**Status:** Ready to implement (roadmap-approved; agent recommendation locked)  
**Prerequisite:** v0.1.15 audio foundation complete

## Philosophy

**Sensible conversions only** — every conversion the bundled engines can actually do honestly. No fake “anything → anything” (e.g. inventing RAW/layers/SVG from a flat JPEG).

## Architecture

Keep audio and images as **separate request types** sharing queue / temp / verify / overwrite patterns — do **not** jam `ImageFormat` into today’s `OutputFormat`.

| Concern | Engine |
|---|---|
| Audio | FFmpeg / FFprobe (existing) |
| Images | ImageMagick (+ LibRaw for camera RAW inputs) |
| Safety | Locked-down ImageMagick `policy.xml`; argv arrays only; source never modified |

UI: mode or clear section switch (Audio / Images) so the main convert surface stays one job at a time.

## v0.2.0 scope (recommended first ship)

1. Import common rasters (JPEG, PNG, WebP, TIFF, BMP, GIF) + common RAW via LibRaw where Magick delegates.
2. Output: JPEG, PNG, WebP, TIFF (quality preset for lossy JPEG/WebP).
3. Same batch / overwrite / destination / folder-structure patterns as audio.
4. Bundle ImageMagick portable + `policy.xml` that denies risky delegates (no arbitrary shell/URL reads).
5. Licensing notice parallel to FFmpeg (`THIRD_PARTY_IMAGEMAGICK.txt`).

## Non-goals (later 0.2.x)

- Full Photoshop-feature parity, vector/SVG authoring, PDF as a primary product
- Cloud APIs, accounts, telemetry
- Parallel “Speed” modes

## Success criteria

- User can batch-convert a folder of photos locally offline
- Sources untouched; temp → verify → finalize
- Typecheck + Rust tests; signed NSIS release with Magick resources
