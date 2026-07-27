# Creating a signed release (auto-updater)

Users on **v0.1.2+** can click **Update** in the app when a newer GitHub Release exists.

## One-time keys (already generated on this machine)

- Private key: `%USERPROFILE%\.tauri\jwconverter.key` (**never commit**)
- Public key: embedded in `src-tauri/tauri.conf.json`
- Password file (local only): `%USERPROFILE%\.tauri\jwconverter.key.password.txt`

If you lose the private key or password, existing installs cannot trust new updates.

## Build + publish

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
$env:TAURI_SIGNING_PRIVATE_KEY_PATH = "$env:USERPROFILE\.tauri\jwconverter.key"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = Get-Content "$env:USERPROFILE\.tauri\jwconverter.key.password.txt" -Raw

npm run tauri build
powershell -File scripts/publish-github-release.ps1 -Version 0.1.3
```

`publish-github-release.ps1` uploads:

- the NSIS setup `.exe`
- `latest.json` (what the app polls at `/releases/latest/download/latest.json`)

Creating the `vX.Y.Z` tag also starts **GitHub Actions → Build**, which builds the **macOS DMG** (and Linux AppImage) and attaches them to the same release. Wait for that workflow if the DMG is not visible yet.

### macOS note

DMGs are **unsigned** until Apple Developer ID + notarization. Users: right-click → Open the first time.