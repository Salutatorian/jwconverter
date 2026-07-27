# macOS DMG on GitHub Releases (v0.5.0) — design

**Version:** v0.5.0  
**Status:** Approved (user requested DMG on next release)  
**Date:** 2026-07-28

## Goal

Ship an **unsigned macOS `.dmg`** on the GitHub Release for the next version, downloadable next to the Windows installer.

## Why it failed before

1. `createUpdaterArtifacts: true` requires `TAURI_SIGNING_PRIVATE_KEY` — Mac CI had none → build aborted.  
2. Linux/Mac: `resources/imagemagick/` sometimes missing → Tauri resource check fails.  
3. Publish script only uploaded Windows NSIS.

## Approach

1. **CI (tag `v*`)** builds macOS `app` + `dmg` with updater artifacts **disabled** for that job (unsigned DMG; Windows remains the signed updater path).  
2. **Always** create `src-tauri/resources/imagemagick/` in `fetch-media-tools.sh` (real Magick when available, else stub so packaging succeeds).  
3. **Release job** uploads the DMG to the same GitHub Release tag (after Windows publish or via `gh release upload`).  
4. Release notes list **Windows** + **macOS (unsigned)** with Gatekeeper caveat (right-click → Open).

## Non-goals

- Apple Developer ID / notarization (later)  
- Mac auto-updater in `latest.json` until signed  
- Perfect portable Magick bundling on Mac (system/PATH fallback already exists)

## Success

- Tag `v0.5.0` release includes `.dmg` asset  
- README documents Mac download + Gatekeeper note  
