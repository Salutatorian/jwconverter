# Bundled FFmpeg / FFprobe / yt-dlp binaries (local + packaging)

Place here (gitignored):

- `ffmpeg.exe`
- `ffprobe.exe`
- `yt-dlp.exe` (Links mode)

For Tauri release packaging (`externalBin`), also provide target-triple copies:

- Windows: `ffmpeg-x86_64-pc-windows-msvc.exe`, `ffprobe-x86_64-pc-windows-msvc.exe`, `yt-dlp-x86_64-pc-windows-msvc.exe`
- macOS arm64: `ffmpeg-aarch64-apple-darwin`, `ffprobe-aarch64-apple-darwin`, `yt-dlp-aarch64-apple-darwin`
- Linux x64: `ffmpeg-x86_64-unknown-linux-gnu`, `ffprobe-x86_64-unknown-linux-gnu`, `yt-dlp-x86_64-unknown-linux-gnu`

Optional Links override: `CONVERTER_YTDLP` → full path to a yt-dlp executable.

CI uses `scripts/fetch-media-tools.sh` on macOS/Linux (including yt-dlp). Windows packaging still uses the local Gyan + portable Magick + yt-dlp layout above.

## ImageMagick (images / v0.2+)

Place a portable Magick tree at `src-tauri/binaries/imagemagick/` (gitignored), including `magick.exe`
and config XML. Copy the same tree to `src-tauri/resources/imagemagick/` before `npm run tauri build`.

Example (PowerShell), using a GitHub portable Q16 x64 release:

```powershell
# extract magick.exe + xml into binaries/imagemagick, then:
Copy-Item -Recurse -Force src-tauri\binaries\imagemagick src-tauri\resources\imagemagick
```

See `third_party/THIRD_PARTY_IMAGEMAGICK.txt` at the repository root.
