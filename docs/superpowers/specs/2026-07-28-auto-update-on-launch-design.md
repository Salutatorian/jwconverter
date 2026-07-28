# Auto-update on launch (full-screen) — design

**Date:** 2026-07-28  
**Status:** Approved (Approach A)  
**Replaces in part:** click-to-install-only behavior from `2026-07-26-auto-updater-design.md`

## Goal

When the user opens JW Converter and a newer signed Windows build exists, the app **automatically** downloads and installs it behind a full-screen updating UI with a **blue progress bar and percentage**, then relaunches. No Update click required for the happy path.

## Behavior

1. On launch, check for updates (existing Tauri updater + `latest.json`).
2. If an update is available:
   - Show a **full-screen overlay** immediately (blocks convert / settings / mode switch).
   - **Auto-start** `downloadAndInstall` (no button required).
   - Show: title “Updating…”, version line `vX.Y.Z`, blue progress bar, percentage.
   - At 100%: show “Restarting…”, then `relaunch()`.
3. If up to date or check fails with no known update: normal app UI (no overlay).
4. If download/install fails: overlay stays up with error text + **Retry** + **Continue without updating** (dismiss overlay and use current version).
5. Settings → Updates: keep **Check for updates** and **Update** as manual fallback (and show progress text there if a manual install is in progress).
6. Periodic 4-hour re-check while the app stays open: if an update appears mid-session, **do not** force the overlay mid-conversion; set status to `available` and let Settings / next launch handle it. (Auto overlay only on the launch-time check path, or when the user is idle and we optionally offer later — v1: auto-install overlay **only from the initial launch check**.)

## UI

- Full-screen dimmed backdrop + centered card.
- Blue bar (`#3b82f6` / similar), fill width = percent, large percent label.
- No decorative AI imagery — text + bar only.
- Respect light/dark theme for card/background; bar stays blue in both.

## Mechanism

- Existing `@tauri-apps/plugin-updater` + `@tauri-apps/plugin-process`.
- Reuse `useUpdater` download percent events; add `autoInstallOnLaunch` behavior after a successful launch check finds an update.
- Windows primary (signed NSIS). macOS/Linux: check may run, but auto-install overlay only when the updater can actually install (same as today — if check returns an update, attempt; on unsupported platforms show error + Continue).

## Out of scope

- Background download without overlay
- Auto-update mid-conversion
- Delta/patch updates
- Changing signing / `latest.json` pipeline

## Success

- Reopen after a new release: see Updating screen with blue % bar, then new version after relaunch.
- Offline / failed update: can Retry or Continue without being stuck.
