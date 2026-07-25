# More Output Formats Implementation Plan

> **For agentic workers:** Execute task-by-task. Checkboxes track progress.

**Goal:** Add AAC/M4A, Opus, OGG, ALAC, and AIFF with fixed encoder defaults.

**Architecture:** Extend `OutputFormat` through planner → verify → FormatPicker. No quality presets.

**Tech Stack:** Existing Tauri/Rust/React pipeline

**Spec:** `docs/superpowers/specs/2026-07-26-more-formats-design.md`

---

### Task 1: Git foundation
- [ ] `git init` if needed; ensure `.gitignore` excludes `node_modules`, `target`, `binaries/*.exe`, `.env`
- [ ] Initial commit of current codebase (foundation through overwrite) with a substantial message

### Task 2: Rust formats + tests
- [ ] Extend `OutputFormat` with `Aac`, `Opus`, `Ogg`, `Alac`, `Aiff`
- [ ] Update `planner.rs` plans and ffmpeg args per spec
- [ ] Update `verify.rs` codec_matches
- [ ] Add parameterized/table tests converting fixture to each new format
- [ ] `cargo test` passes
- [ ] Commit: `feat: add AAC, Opus, OGG, ALAC, and AIFF encoder plans`

### Task 3: Frontend + docs
- [ ] Extend TS types and `OUTPUT_FORMATS`
- [ ] Update FormatPicker helper copy
- [ ] Phase → `formats` in app_info + README
- [ ] `npm run typecheck` passes
- [ ] Commit: `feat: expose new output formats in the converter UI`

### Task 4: Verify
- [ ] Full `cargo test` + `npm run typecheck`
- [ ] Commit docs/spec/plan if not already: `docs: add more-formats design and plan`
