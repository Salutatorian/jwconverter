# Intelligence preflight (E1) Implementation Plan

> **For agentic workers:** Implement task-by-task. Checkboxes track progress.

**Goal:** Soft quality warnings + rough size estimate + hard disk gate before Convert (v0.1.9).

**Architecture:** Rust `engine::preflight` owns estimates/warnings/free-space; thin `preflight_batch` command; UI modal on Convert click.

**Tech Stack:** Tauri 2, Rust, React, Windows `GetDiskFreeSpaceExW`

---

### Task 1: Rust preflight module + tests
- [ ] `src-tauri/src/engine/preflight.rs` — warn kinds, estimate, free space, aggregate
- [ ] Wire `engine/mod.rs`; unit tests for lossy→flac, bit-depth upsample, margin math
- [ ] `commands/preflight.rs` + register handler

### Task 2: Frontend
- [ ] Types + `preflightBatch` in `tauri.ts`
- [ ] `PreflightModal` (soft vs hard)
- [ ] `ConverterView` Convert → preflight → gate → start batch

### Task 3: Release
- [ ] What's New, bump 0.1.8→0.1.9, typecheck + cargo test, signed build, publish
