# Packaging Design

**Date:** 2026-07-26  
**Phase:** `packaging`  
**Status:** Approved

## Goal

Ship a Windows NSIS installer for JW Converter with app icons from the JWC logo and bundled FFmpeg/FFprobe so a clean PC works offline.

## Decisions

| Decision | Choice |
|---|---|
| Installer | Tauri NSIS (`currentUser`) |
| FFmpeg | Bundle existing Gyan binaries from `src-tauri/binaries/` |
| Licensing | Document in `docs/ffmpeg-licensing.md` (GPL attribution) |
| Icons | Generated from `assets/jwc-logo-transparent.png` |

## Scope

1. Icon set for Tauri (`icon.ico`, PNG sizes, keep icns placeholder if needed)
2. Bundle `ffmpeg.exe` + `ffprobe.exe` as resources; resolve next to the app in release
3. Licensing/attribution doc
4. Phase → `packaging`; README build/install notes
5. `npm run tauri build` produces NSIS installer

## Non-goals

- Portable zip
- LGPL rebuild of FFmpeg
- Code signing (can follow later)
- macOS/Linux bundles
