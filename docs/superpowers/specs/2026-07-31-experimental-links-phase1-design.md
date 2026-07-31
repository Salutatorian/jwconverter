# Experimental Links — Phase 1 design

**Date:** 2026-07-31  
**Branch:** `experimental/links`  
**Status:** Phase 1 (metadata inspection only)  
**Stable version:** unchanged (`0.5.2`) — no release, no updater, no public notes

## Goal (Phase 1)

Prove local link inspection:

Paste URL → validate → invoke yt-dlp with argv-only `--dump-single-json` → normalize → return `LinkMediaInfo` to UI.

**No download. No merge. No FFmpeg post-process. No stable UX.**

## Integration points

| Concern | Location |
|---|---|
| Sidecar resolve | `media/paths.rs` → `resolve_ytdlp()` + `CONVERTER_YTDLP` |
| Resolver adapter | `media/ytdlp.rs` |
| URL safety | `media/link_url.rs` |
| IPC | `commands/link_analyze.rs` → `analyze_link` |
| Feature gate | `AppInfo.linksExperimental` = `cfg!(debug_assertions)` |
| UI | `views/LinkConverterView.tsx` + AppShell `"links"` when gated on |
| Types | `src/types/links.ts` |

## yt-dlp Windows sidecar (production plan)

- Dev: `src-tauri/binaries/yt-dlp.exe` (gitignored) or `CONVERTER_YTDLP`
- Package later: Tauri `externalBin: ["binaries/yt-dlp"]` → `yt-dlp-x86_64-pc-windows-msvc.exe`
- Do **not** enable `externalBin` or bump version in Phase 1

## Inspection command (safe argv)

```
yt-dlp
  --dump-single-json
  --no-playlist
  --no-warnings
  --skip-download
  <validated-url>
```

Never shell-interpolated. Windows: `CREATE_NO_WINDOW`.

## Normalized model (Phase 1)

`LinkMediaInfo`: originalUrl, webpageUrl, extractor, service, id, title, creator, durationSeconds, isLive, isPlaylist, itemCount, warnings[], thumbnail skipped (CSP).

## Do not modify

Audio/image runners, queues, planners, verify, fs_safety finalize, updater, publish scripts, public version files.

## Out of Phase 1

Download, merge, playlist download, cookies, livestream record, batch URLs, release packaging of yt-dlp.
