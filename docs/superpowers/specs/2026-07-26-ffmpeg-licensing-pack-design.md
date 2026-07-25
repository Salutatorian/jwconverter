# Broader FFmpeg licensing pack (E4) — design

**Version:** v0.1.15 (audio-foundation final)  
**Status:** Approved (ship with recommended pack)  
**Next:** v0.2.0 — image conversion

## Goal

Redistribution compliance for bundled FFmpeg/FFprobe: exact build identity, license notice in the install tree, source offer, and in-app About attribution.

## Ship

1. Document exact build in `docs/ffmpeg-licensing.md` (version, Gyan full GPL build, config flags summary).
2. Bundle `resources/THIRD_PARTY_FFMPEG.txt` (attribution + source offer) via Tauri `resources`.
3. Settings → About: short FFmpeg notice + open licensing / source links.
4. Point EULA §4 at the bundled notice file.

## Non-goals

Replacing Gyan with an LGPL-only build; hosting a full FFmpeg source tarball on GitHub Releases (link + written offer is enough for this stage).
