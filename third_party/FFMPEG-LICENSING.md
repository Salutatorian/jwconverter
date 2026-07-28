# FFmpeg / FFprobe licensing (JW Converter)

JW Converter shells out to **FFmpeg** and **FFprobe** for local audio analysis and conversion.
Those tools are **not** part of the JW Converter source license by themselves; they are third-party
binaries with their own terms.

## What we ship

For Windows release installers, the NSIS package includes:

- `ffmpeg.exe`
- `ffprobe.exe`
- `THIRD_PARTY_FFMPEG.txt` (build identity, GPL notice, source offer)

Binaries are copied at build time from `src-tauri/binaries/` (gitignored). Development builds use
the same folder.

## Exact build currently packaged (v0.1.15)

| Field | Value |
|---|---|
| Version string | `8.1-full_build-www.gyan.dev` |
| Origin | [Gyan.dev FFmpeg Windows builds](https://www.gyan.dev/ffmpeg/builds/) (WinGet: `Gyan.FFmpeg`) |
| License class | **GPL** (`--enable-gpl --enable-version3`) |
| Recorded | 2026-07-26 |

Confirm on a machine with the bundled tools:

```text
ffmpeg -version
ffprobe -version
```

If you change the binaries, update this table, `third_party/THIRD_PARTY_FFMPEG.txt`, and the
installer resources **before** publishing.

## Attribution

> This product uses FFmpeg and FFprobe. FFmpeg is a trademark of Fabrice Bellard.
> The bundled Windows binaries are a Gyan.dev **full** build under the **GNU GPL v3**
> (and licenses of included libraries). See `THIRD_PARTY_FFMPEG.txt` next to the installed app,
> https://ffmpeg.org/, and https://www.gyan.dev/ffmpeg/builds/.

## Corresponding source

Provide corresponding source for redistributed GPL binaries. JW Converter’s offer:

1. Links to FFmpeg upstream and Gyan builds (above / in `THIRD_PARTY_FFMPEG.txt`).
2. Written offer for three years via GitHub Issues on this repository.

## Maintainer checklist before publishing an installer

1. Record the **exact** FFmpeg build identity (version string from `ffmpeg -version`).
2. Confirm GPL vs LGPL for **that** build; update docs + `THIRD_PARTY_FFMPEG.txt`.
3. Ensure `../third_party/THIRD_PARTY_FFMPEG.txt` is listed under `bundle.resources` in `tauri.conf.json`.
4. Do **not** commit unknown `.exe` files to git; keep them local / CI-provisioned.

## JW Converter itself

The JW Converter application code in this repository is separate from FFmpeg. Bundling FFmpeg does
not re-license the app source; it adds compliance duties for the bundled binaries.
