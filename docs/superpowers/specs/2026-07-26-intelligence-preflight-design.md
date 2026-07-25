# Intelligence preflight (E1) — design

**Version:** v0.1.9  
**Status:** Approved (umbrella E; ship B = sequential E1→E4)  
**Umbrella:** E1 warnings/size/disk · E2 inputs · E3 VBR · E4 FFmpeg licensing

## Goal

Before a batch starts: honesty warnings for conversions that don’t improve quality, a rough output size estimate, and a hard block when the destination volume likely cannot fit the work.

## Gates

| Condition | Gate |
|---|---|
| Lossy → lossless/PCM | Soft — Continue anyway / Cancel |
| Bit-depth upsample (WAV/AIFF forced higher than source) | Soft — same modal |
| Estimated output + margin > free space | Hard — no Continue |

## Size estimate

Rust `preflight_batch` reuses planner bitrate/PCM assumptions. Aggregate file count, source bytes, estimated output bytes, destination free bytes. Display with `~`. Skip-policy files whose primary already exists are omitted from the estimate (best-effort).

Safety margin for disk: `max(500 MiB, 5% of estimate)` (temp + Replace headroom).

## Non-goals (E1)

Per-row icons; sample-rate forcing; E2–E4.
