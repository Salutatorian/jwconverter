# JW Converter

Local-first Windows audio conversion utility.

Built with Tauri 2, React, TypeScript, and Rust. Conversion uses FFmpeg/FFprobe (wired in later milestones).

## Status

**v0.1.0 packaging** — full converter features plus Windows NSIS installer packaging with JWC icons and bundled FFmpeg/FFprobe.

Working:

- Choose / drag multiple audio files
- Choose / drop folders (recursive scan)
- Preserve relative folder structure in output
- Analyze with FFprobe (local)
- Convert to FLAC / WAV / MP3 / AAC (M4A) / Opus / OGG / ALAC / AIFF
- Quality presets (Low / Medium / High) for lossy formats
- Sequential batch queue
- Temp output → verify → finalize (source never modified)
- Per-file + overall progress, cancel queue
- Default destination: Downloads
- Overwrite policy: Rename (default) / Skip / Replace
- Windows NSIS installer with app icons and bundled FFmpeg/FFprobe

See `docs/ffmpeg-licensing.md` before redistributing the installer.

## Development (Windows)

Prerequisites:

- Node.js LTS
- Rust (rustup) + MSVC Build Tools
- WebView2 Runtime
- `ffprobe.exe` in `src-tauri/binaries/` (gitignored) or on PATH in debug builds

If `cargo` is not found in a new terminal, restart the terminal after installing Rust, or ensure `%USERPROFILE%\.cargo\bin` is on your PATH.

```powershell
npm install
npm run tauri dev
```

### FFmpeg / FFprobe binaries

Place `ffmpeg.exe` and `ffprobe.exe` in `src-tauri/binaries/` (gitignored).

For release packaging, also create target-triple copies (see `src-tauri/binaries/README.md`).

Licensing notes: `docs/ffmpeg-licensing.md`.

### Release installer (Windows)

```powershell
# Requires binaries with target-triple names in src-tauri/binaries/
npm run tauri build
```

Installer output (typical):

`src-tauri/target/release/bundle/nsis/JW Converter_*_x64-setup.exe`

Typecheck:

```powershell
npm run typecheck
```

Rust check:

```powershell
cd src-tauri
cargo check
```

## Architecture

```
UI (React)
  → Tauri IPC commands
    → Conversion engine (jobs, plan, run, verify)
      → media (FFmpeg/FFprobe) + fs_safety (temp/finalize)
```

## Privacy

Conversions will run entirely on your machine. No accounts, no cloud upload, no telemetry by default.
