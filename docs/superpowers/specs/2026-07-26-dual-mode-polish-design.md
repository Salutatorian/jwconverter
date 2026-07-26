# Dual-mode polish (v0.2.7 batch) — design

**Version:** v0.2.7 (batched; was briefly labeled 0.2.6)  
**Status:** In tree — publish with the rest of the 0.2.7 batch  
**Date:** 2026-07-26

## Goal

Make JW Converter read and feel like an **audio + images** local converter. No new encode features.

## Scope

1. **Product truth** — README, Reddit blurb, Settings About, installer/short description mention both modes.  
2. **UX parity** — mode-aware DropZone; Update-available CTA on Images; Audio | Images chip order consistent; Metadata before Overwrite on Images; Image queue Remove safety/a11y aligned with audio.

## Release cadence note

Ship as part of a larger **v0.2.7** release rather than a micro 0.2.6 — accumulate meaningful work, then one signed publish.

## Non-goals

- New formats, encoding knobs, or overwrite modes  
- Screenshot refresh (optional later)  
- Audio encode changes

## Success

- Copy no longer claims audio-only  
- Images mode matches Audio for update CTA + drop accessibility  
- Published only when the 0.2.7 batch is ready
