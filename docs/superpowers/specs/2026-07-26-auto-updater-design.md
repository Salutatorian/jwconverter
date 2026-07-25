# Auto-updater (click-to-install) — design

## Goal

Installed JW Converter checks GitHub Releases for a newer version. Updates never install silently — the user must click **Update**.

## Behavior

1. On app launch, check for updates (non-blocking).
2. While the app stays open, re-check every 4 hours (covers a missed first scan / offline start).
3. Manual **Check for updates** / **Update** control in the header.
4. If an update is available: show a small banner with the new version + enable **Update**.
5. Clicking **Update** downloads, installs (Windows passive NSIS), then relaunches.
6. Do not auto-install. If a conversion is running, still allow the click but the Windows installer will quit the app first (Tauri Windows behavior) — banner copy should say the app will restart.

## Mechanism

- Tauri 2 `updater` + `process` plugins.
- Endpoint: static `latest.json` on GitHub Releases  
  `https://github.com/Salutatorian/jwconverter/releases/latest/download/latest.json`
- `createUpdaterArtifacts: true` so builds produce `.exe.sig`.
- Signing: public key in `tauri.conf.json`; private key only in env `TAURI_SIGNING_PRIVATE_KEY` at build/release time (never committed).

## Version

Ship as **v0.1.2** (first build that includes the updater). Existing **v0.1.0** installs must download **v0.1.2** once manually; auto-updates apply from **v0.1.2** onward.

## Out of scope

- Forced silent installs
- Delta/patch updates
- Non-Windows platforms
