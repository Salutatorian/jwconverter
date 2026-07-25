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
