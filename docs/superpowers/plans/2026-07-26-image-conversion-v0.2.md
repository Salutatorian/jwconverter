# Image conversion (v0.2.0) Implementation Plan

**Goal:** Parallel image pipeline with ImageMagick; Audio/Images mode switch; ship signed v0.2.0.

**Architecture:** Separate image job/request types; reuse temp→verify→finalize + overwrite; Magick portable + locked policy.xml under resources.

## Tasks
1. Bundle Magick portable + policy + THIRD_PARTY notice
2. Rust: paths, identify, convert, image job/runner/queue/commands
3. Frontend: mode switch + ImageConverterView
4. Version bump, tests, signed release
