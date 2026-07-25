# FFmpeg / FFprobe licensing (JW Converter)

JW Converter shells out to **FFmpeg** and **FFprobe** for local audio analysis and conversion.
Those tools are **not** part of the JW Converter source license by themselves; they are third-party
binaries with their own terms.

## What we ship

For Windows release installers, the NSIS package may include:

- `ffmpeg.exe`
- `ffprobe.exe`

Copied at build time from `src-tauri/binaries/` (gitignored). Development builds use the same folder.

## Current local build source

This project’s local binaries were obtained from the **Gyan.dev FFmpeg Windows builds** distributed
via WinGet (`Gyan.FFmpeg`). Those builds are commonly **GPL**-licensed when full codec sets are enabled.

If you redistribute an installer that embeds these binaries, you must follow the license of the
**exact** FFmpeg build you bundle (GPL and/or LGPL and any patent/codec notices that apply).

## Attribution (required for redistribution)

Please retain attribution similar to:

> This product uses FFmpeg and FFprobe. FFmpeg is a trademark of Fabrice Bellard.
> FFmpeg binaries are provided under the terms of the GNU GPL / LGPL as applicable to the build used.
> See https://ffmpeg.org/ and the license files shipped with your FFmpeg build.

## Maintainer checklist before publishing an installer

1. Record the **exact** FFmpeg build identity (version, download URL or WinGet package version, date).
2. Confirm whether that build is **GPL** or **LGPL** (and which libraries are linked).
3. Ship matching license text with the installer or in `docs/` / About UI.
4. Do **not** commit unknown `.exe` files to git; keep them local / CI-provisioned.

## JW Converter itself

The JW Converter application code in this repository is separate from FFmpeg. Bundling FFmpeg does
not re-license the app source; it adds compliance duties for the bundled binaries.
