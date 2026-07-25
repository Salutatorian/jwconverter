# Bundled FFmpeg / FFprobe binaries (local + packaging)

Place here (gitignored):

- `ffmpeg.exe`
- `ffprobe.exe`

For Tauri release packaging (`externalBin`), also provide target-triple copies:

- `ffmpeg-x86_64-pc-windows-msvc.exe`
- `ffprobe-x86_64-pc-windows-msvc.exe`

You can duplicate the plain names:

```powershell
$t = "x86_64-pc-windows-msvc"
Copy-Item ffmpeg.exe "ffmpeg-$t.exe"
Copy-Item ffprobe.exe "ffprobe-$t.exe"
```

Before public redistribution, read `docs/ffmpeg-licensing.md` and document the exact build + license.

## ImageMagick (images / v0.2+)

Place a portable Magick tree at `src-tauri/binaries/imagemagick/` (gitignored), including `magick.exe`
and config XML. Copy the same tree to `src-tauri/resources/imagemagick/` before `npm run tauri build`.

Example (PowerShell), using a GitHub portable Q16 x64 release:

```powershell
# extract magick.exe + xml into binaries/imagemagick, then:
Copy-Item -Recurse -Force src-tauri\binaries\imagemagick src-tauri\resources\imagemagick
```

See `resources/THIRD_PARTY_IMAGEMAGICK.txt`.
