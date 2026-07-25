# JW Converter

Local-first Windows audio conversion utility.

Built with Tauri 2, React, TypeScript, and Rust. Conversion uses FFmpeg/FFprobe (wired in later milestones).

## Status

**v0.1.0 quality** — foundation through formats, plus Low/Medium/High quality presets for lossy outputs.

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

Not yet implemented:

- Production-bundled media binaries (installer packaging)

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

Place `ffprobe.exe` (and later `ffmpeg.exe`) in `src-tauri/binaries/`.

These are **not committed**. Before public redistribution, document the exact build, license (LGPL/GPL), and attribution. Do not ship an unknown build.

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
