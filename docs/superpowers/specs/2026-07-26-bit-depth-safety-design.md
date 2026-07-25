# Bit depth + safety polish (A + B) — design

**Version:** v0.1.7  
**Status:** Approved (user ordered A+B → C → E → D/F/G)

## A — WAV / AIFF bit depth

- New preset: **Original** (default) · 16-bit · 24-bit · 32-bit float
- Visible only when output is WAV or AIFF
- Lossy quality picker unchanged (Low/Medium/High)
- Planner picks PCM codec from preset; **Original** maps from FFprobe `sample_fmt` + `bits_per_raw_sample`
- Enrich `AudioInfo`: bitDepth, sampleFormat, bitrate, channelLayout, bitsPerRawSample
- Queue line shows richer source summary when available

## B — Safety polish

- Replace rollback: remove partial destination before restoring `.jwbak`; surface restore failure; clean failed copy
- Enable CSP (self + Outfit fonts + updater GitHub connect hosts)

## Out of scope (later slices)

- Cover art (C), intelligence warnings (E), input expansion (D), VBR (F), FFmpeg licensing pack (G)
