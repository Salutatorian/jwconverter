# Third-party components

JW Converter is original first-party code. This directory contains **only** the
attribution notices and licensing documentation for the unmodified third-party
runtime tools the installer bundles, as required by their licenses.

Nothing in this directory is application source code, and nothing here is
compiled into the app. The Rust/TypeScript source lives in `src-tauri/src/` and
`src/`.

| File | Component | License |
|---|---|---|
| `THIRD_PARTY_FFMPEG.txt` | FFmpeg / FFprobe binaries (Gyan.dev Windows build) | GNU GPL v3 |
| `THIRD_PARTY_IMAGEMAGICK.txt` | ImageMagick portable (`magick.exe`) | ImageMagick License |
| `THIRD_PARTY_YTDLP.txt` | yt-dlp (experimental Links only; not in stable releases yet) | Unlicense |
| `FFMPEG-LICENSING.md` | Redistribution/compliance notes for the bundled FFmpeg build | — |

The binary payloads themselves are **not** committed to this repository; they
are provisioned locally or by CI at packaging time (see
`src-tauri/binaries/README.md`).
