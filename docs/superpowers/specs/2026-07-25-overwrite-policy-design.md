# Overwrite Policy Design

**Date:** 2026-07-25  
**Phase:** `overwrite`  
**Status:** Implemented (2026-07-25)

## Goal

Let the user choose what happens when the intended output file already exists, without changing the safe conversion lifecycle (temp → verify → finalize; source never modified).

## Decisions (locked)

| Decision | Choice |
|---|---|
| Default policy | **Rename** (current behavior) |
| UI | Simple picker only — Rename / Skip / Replace |
| Ask mode | Out of scope (later) |
| Scope of policy | Per batch, chosen before Convert; applied to every job |
| Architecture | Policy on each job; resolve destination **before** encoding |

## Policies

Given destination folder `D`, stem `S`, extension `E`, primary path `P = D/S.E`.

### Rename (default)

- If `P` does not exist → use `P`.
- If `P` exists → use first free `D/S (n).E` (same algorithm as today).
- Always run convert when not cancelled.

### Skip

- If `P` does not exist → use `P`, convert normally.
- If `P` exists → mark job **skipped**; do **not** start FFmpeg; do **not** write temp.
- Skipped is distinct from failed and cancelled.
- Batch summary includes a skipped count.

### Replace

- Always target primary path `P` (never auto-rename).
- Convert to temp → verify as today.
- On finalize: if `P` exists, remove `P` only after verification succeeded, then move temp → `P`.
- Never delete or modify the source file.
- Race: if `P` reappears between check and finalize, fail that job with a clear exists error (do not delete an unexpected different path).

## Non-goals

- Per-file “Ask” prompts / Apply to all
- Remembering last policy across app restarts (nice-to-have later; in-memory for this phase is fine)
- Overwriting files outside the resolved destination path for that job
- Changing format list or quality presets

## Data model

### Rust

```text
enum OverwritePolicy { Rename, Skip, Replace }  // serde camelCase / lowercase as existing enums

ConversionJob {
  ...
  overwrite_policy: OverwritePolicy  // default Rename
}

ConversionRequest {
  ...
  overwrite_policy: OverwritePolicy  // default Rename for IPC back-compat
}

JobStatus: add Skipped

BatchEvent / queue summary: add skipped: u32
```

### Frontend

- `OverwritePolicy = "rename" | "skip" | "replace"`
- `OverwritePicker` next to `FormatPicker` (same visual language)
- Default state: `"rename"`
- Pass policy on every `ConversionRequest` in the batch
- Queue row / progress: show skipped status; batch line includes skipped count

## Engine flow (per job)

1. Validate source; ensure destination dir (including relative subdir).
2. Plan encoder; compute primary path `P`.
3. Resolve final path from policy:
   - Rename → `unique_final_path`
   - Skip + `P` exists → emit status `skipped`, set `outputPath` to existing `P` (informational), return without error
   - Skip + `P` missing → `P`
   - Replace → `P`
4. If skipped, stop.
5. Temp → FFmpeg → verify (unchanged).
6. Finalize:
   - Rename / Skip paths: existing finalize (final must not exist)
   - Replace: if final exists, delete that file, then rename/copy temp → final; still refuse if temp is not ours

## UI copy

- Section: **If file exists**
- Buttons: Rename · Skip · Replace
- Helper: short one-liner, e.g. “Rename keeps both files. Skip leaves the existing file. Replace overwrites it after a successful convert.”

## Testing

- Unit/integration: existing WAV→FLAC still passes with Rename.
- Rename: existing `out.flac` → produces `out (1).flac`.
- Skip: existing primary → status skipped, no new file, source untouched.
- Replace: existing primary → same path replaced; source untouched; content is new convert.
- Batch summary skipped counter increments for Skip cases.

## Success criteria

- User can pick Rename / Skip / Replace before Convert.
- Default remains Rename; re-running a large library without changing the picker still auto-renames.
- Skip does not run FFmpeg for conflicted files.
- Replace only overwrites the intended primary output after verify.
- Typecheck + `cargo check` + existing convert test pass; new policy tests pass.

## Follow-ups (not this phase)

- Ask + Apply to all
- Persist last-used policy
- Preflight “N files will collide” summary before starting
