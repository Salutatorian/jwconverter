# JW Converter

Convert **audio**, **images**, and **media links** on your computer.

No accounts. No cloud upload. No telemetry. Your files stay on your PC.

## Download

**[Download the latest release](https://github.com/Salutatorian/jwconverter/releases/latest)**

| Platform | Get |
| --- | --- |
| **Windows** | Signed `.exe` installer (recommended) |
| **macOS** | Apple Silicon `.dmg` — first open: right-click → **Open** |
| **Linux** | AppImage when listed on the release |

## What it does

### Audio

Convert audio to FLAC, WAV, MP3, M4A, and more. Drop files or folders. Tags and cover art are kept when possible.

![JW Converter — Audio mode](docs/assets/sneak-peek-audio.png)

### Images

Convert photos to JPEG, PNG, WebP, and more. Common RAW camera files can be imported.

![JW Converter — Images mode](docs/assets/sneak-peek-images.png)

### Links

Paste a public video/audio URL and download it. Playlists and batches zip into one file so your Downloads folder stays tidy.

![JW Converter — Links mode](docs/assets/sneak-peek-links.png)

## Privacy

Everything runs locally. Sources are never modified — outputs are written next to the destination you choose.

## License

[Apache License 2.0](LICENSE). Bundled tools (FFmpeg, ImageMagick, yt-dlp) keep their own licenses — see [`third_party/`](third_party/).

## For developers

```powershell
npm install
npm run tauri dev
```

Needs Node.js LTS, Rust, WebView2, and FFmpeg/FFprobe in `src-tauri/binaries/`.  
Build: `npm run tauri build` → installer under `src-tauri/target/release/bundle/nsis/`.
