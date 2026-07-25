# Overwrite Policy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Rename / Skip / Replace overwrite policy (default Rename) so batch re-runs behave predictably when the destination file already exists.

**Architecture:** Pass `overwritePolicy` on each conversion job. Before FFmpeg, resolve the primary output path and apply the policy (rename to unique, skip without encoding, or target primary for replace). Finalize for Replace deletes the existing primary only after verify, then moves our temp into place. Queue treats `JobStatus::Skipped` separately from completed/failed/cancelled.

**Tech Stack:** Tauri 2, Rust engine (`job` / `runner` / `finalize` / `queue`), React + TypeScript UI

**Spec:** `docs/superpowers/specs/2026-07-25-overwrite-policy-design.md`

---

## File map

| File | Responsibility |
|---|---|
| `src-tauri/src/engine/job.rs` | `OverwritePolicy` enum; `JobStatus::Skipped`; field on `ConversionJob` |
| `src-tauri/src/fs_safety/finalize.rs` | Primary path helper; rename uniqueness (existing); replace-aware finalize |
| `src-tauri/src/engine/runner.rs` | Resolve path from policy; early skip; call replace finalize |
| `src-tauri/src/engine/queue.rs` | `skipped` counter; emit skipped vs completed from `ConversionResult.status` |
| `src-tauri/src/commands/convert.rs` | Accept `overwrite_policy` on requests |
| `src-tauri/src/commands/app_info.rs` | Phase → `overwrite` |
| `src/types/conversion.ts` | TS types + picker options |
| `src/components/OverwritePicker.tsx` | UI picker |
| `src/lib/tauri.ts` | Request field |
| `src/hooks/useBatchConversion.ts` | Pass policy; batch `skipped` |
| `src/views/ConverterView.tsx` | Wire picker + summary |
| `src/components/FileQueue.tsx` | Skipped label |
| `src/components/ConversionProgress.tsx` | Optional skipped label |
| `README.md` | Document phase |

---

### Task 1: Rust types — OverwritePolicy + Skipped

**Files:**
- Modify: `src-tauri/src/engine/job.rs`
- Modify: `src-tauri/src/commands/app_info.rs`

- [ ] **Step 1: Add `OverwritePolicy` and `JobStatus::Skipped`**

In `job.rs`, update `JobStatus` and add the policy enum + job field:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobStatus {
    Idle,
    Analyzing,
    Ready,
    Queued,
    Converting,
    Verifying,
    Completed,
    Failed,
    Cancelled,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OverwritePolicy {
    #[default]
    Rename,
    Skip,
    Replace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionJob {
    pub id: String,
    pub source_path: String,
    pub destination_dir: String,
    #[serde(default)]
    pub relative_subdir: Option<String>,
    pub output_format: OutputFormat,
    #[serde(default)]
    pub overwrite_policy: OverwritePolicy,
    pub status: JobStatus,
}
```

- [ ] **Step 2: Bump app phase**

In `app_info.rs`:

```rust
phase: "overwrite".to_string(),
```

- [ ] **Step 3: Compile check**

Run:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cd c:\Users\JW\Desktop\projects\converter\src-tauri
cargo check
```

Expected: FAIL — `ConversionJob { ... }` literals missing `overwrite_policy` (runner test and `convert.rs`).

- [ ] **Step 4: Fix compile breakages with default field**

In `commands/convert.rs` `build_queue_item`, add:

```rust
overwrite_policy: request.overwrite_policy,
```

and on `ConversionRequest`:

```rust
#[serde(default)]
pub overwrite_policy: OverwritePolicy,
```

(Import `OverwritePolicy` from `crate::engine::job`.)

In `runner.rs` test job struct, add:

```rust
overwrite_policy: OverwritePolicy::Rename,
```

- [ ] **Step 5: Re-check**

```powershell
cargo check
```

Expected: PASS (or only unrelated warnings).

- [ ] **Step 6: Commit (only if user asked for commits)**

```powershell
git add src-tauri/src/engine/job.rs src-tauri/src/commands/app_info.rs src-tauri/src/commands/convert.rs src-tauri/src/engine/runner.rs
git commit -m "feat: add overwrite policy and skipped job status types"
```

---

### Task 2: Finalize helpers — primary path + replace

**Files:**
- Modify: `src-tauri/src/fs_safety/finalize.rs`

- [ ] **Step 1: Write failing unit tests at bottom of `finalize.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn primary_final_path_joins_stem_and_extension() {
        let dir = std::env::temp_dir().join(format!("jw-fin-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let path = primary_final_path(&dir, "song", "flac");
        assert_eq!(path, dir.join("song.flac"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unique_final_path_renames_when_exists() {
        let dir = std::env::temp_dir().join(format!("jw-fin-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let primary = dir.join("song.flac");
        fs::write(&primary, b"old").unwrap();
        let path = unique_final_path(&dir, "song", "flac");
        assert_eq!(path, dir.join("song (1).flac"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn finalize_output_replace_overwrites_existing() {
        let dir = std::env::temp_dir().join(format!("jw-fin-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let final_path = dir.join("song.flac");
        fs::write(&final_path, b"old-content").unwrap();

        // Temp must match is_our_temp_file naming — read temp.rs and mirror.
        let temp_path = dir.join(format!(".jwconverting-song-{}.flac", uuid::Uuid::new_v4()));
        fs::write(&temp_path, b"new-content").unwrap();

        finalize_output_with_policy(&temp_path, &final_path, true).expect("replace");
        assert!(!temp_path.exists());
        assert_eq!(fs::read(&final_path).unwrap(), b"new-content");
        let _ = fs::remove_dir_all(&dir);
    }
}
```

If `is_our_temp_file` requires a specific name pattern, match whatever `temp::temp_output_path` / `is_our_temp_file` already use (read `src-tauri/src/fs_safety/temp.rs` and mirror it).

- [ ] **Step 2: Run tests — expect FAIL**

```powershell
cargo test --manifest-path c:\Users\JW\Desktop\projects\converter\src-tauri\Cargo.toml primary_final_path_joins -- --nocapture
```

Expected: FAIL (function missing).

- [ ] **Step 3: Implement helpers**

Replace/extend `finalize.rs`:

```rust
/// Primary destination path (no auto-rename).
pub fn primary_final_path(destination_dir: &Path, stem: &str, extension: &str) -> PathBuf {
    destination_dir.join(format!("{stem}.{extension}"))
}

/// Choose a final path. Never silently overwrite — auto-rename instead.
pub fn unique_final_path(destination_dir: &Path, stem: &str, extension: &str) -> PathBuf {
    let primary = primary_final_path(destination_dir, stem, extension);
    if !primary.exists() {
        return primary;
    }
    for index in 1..10_000 {
        let candidate = destination_dir.join(format!("{stem} ({index}).{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    destination_dir.join(format!("{stem} ({}).{extension}", uuid_like()))
}

/// Move verified temp into final path.
/// When `allow_replace` is true and final exists, delete final first then move.
pub fn finalize_output_with_policy(
    temp_path: &Path,
    final_path: &Path,
    allow_replace: bool,
) -> Result<(), AppError> {
    if !is_our_temp_file(temp_path) {
        return Err(AppError::VerificationFailure {
            detail: "Refusing to finalize a file that is not our temporary output.".to_string(),
        });
    }

    if final_path.exists() {
        if !allow_replace {
            return Err(AppError::OutputExists {
                detail: format!(
                    "Destination already exists unexpectedly: {}",
                    final_path.display()
                ),
            });
        }
        std::fs::remove_file(final_path).map_err(|error| AppError::DestinationUnavailable {
            detail: format!("Could not replace existing output: {error}"),
        })?;
    }

    if let Some(parent) = final_path.parent() {
        if !parent.is_dir() {
            return Err(AppError::DestinationUnavailable {
                detail: format!("Destination folder unavailable: {}", parent.display()),
            });
        }
    }

    match std::fs::rename(temp_path, final_path) {
        Ok(()) => Ok(()),
        Err(_) => {
            std::fs::copy(temp_path, final_path).map_err(|error| {
                AppError::DestinationUnavailable {
                    detail: format!("Could not write output file: {error}"),
                }
            })?;
            cleanup_temp(temp_path);
            Ok(())
        }
    }
}

pub fn finalize_output(temp_path: &Path, final_path: &Path) -> Result<(), AppError> {
    finalize_output_with_policy(temp_path, final_path, false)
}
```

- [ ] **Step 4: Run finalize tests**

```powershell
cargo test --manifest-path c:\Users\JW\Desktop\projects\converter\src-tauri\Cargo.toml fs_safety::finalize -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit (only if user asked)**

```powershell
git add src-tauri/src/fs_safety/finalize.rs
git commit -m "feat: support replace-aware finalize and primary output paths"
```

---

### Task 3: Runner — resolve policy before encode

**Files:**
- Modify: `src-tauri/src/engine/runner.rs`

- [ ] **Step 1: Add failing tests for skip / rename / replace**

Append in `runner.rs` `tests` module (reuse fixture + helper to build job):

```rust
fn test_job(
    source: &Path,
    out_dir: &Path,
    policy: OverwritePolicy,
) -> ConversionJob {
    ConversionJob {
        id: uuid::Uuid::new_v4().to_string(),
        source_path: source.to_string_lossy().into_owned(),
        destination_dir: out_dir.to_string_lossy().into_owned(),
        relative_subdir: None,
        output_format: OutputFormat::Flac,
        overwrite_policy: policy,
        status: JobStatus::Queued,
    }
}

fn active_and_callbacks() -> (ActiveProcess, RunCallbacks) {
    (
        ActiveProcess {
            child: Arc::new(Mutex::new(None)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        },
        RunCallbacks {
            on_status: Arc::new(|_| {}),
            on_progress: Arc::new(|_| {}),
        },
    )
}

#[test]
fn skip_when_primary_exists() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..").join("tests").join("fixtures").join("tone-440hz.wav");
    if !fixture.is_file() { return; }

    let out_dir = std::env::temp_dir().join(format!("jwconverter-skip-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&out_dir).unwrap();
    let primary = out_dir.join("tone-440hz.flac");
    std::fs::write(&primary, b"existing").unwrap();
    let before = std::fs::read(&primary).unwrap();

    let job = test_job(&fixture, &out_dir, OverwritePolicy::Skip);
    let (active, callbacks) = active_and_callbacks();
    let result = run_job(&job, Some(2.0), &active, &callbacks).expect("skip ok");
    assert_eq!(result.status, JobStatus::Skipped);
    assert_eq!(PathBuf::from(&result.output_path), primary);
    assert_eq!(std::fs::read(&primary).unwrap(), before);
    let count = std::fs::read_dir(&out_dir).unwrap().count();
    assert_eq!(count, 1);
    let _ = std::fs::remove_dir_all(&out_dir);
}

#[test]
fn rename_when_primary_exists() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..").join("tests").join("fixtures").join("tone-440hz.wav");
    if !fixture.is_file() { return; }

    let out_dir = std::env::temp_dir().join(format!("jwconverter-ren-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&out_dir).unwrap();
    let primary = out_dir.join("tone-440hz.flac");
    std::fs::write(&primary, b"existing").unwrap();

    let job = test_job(&fixture, &out_dir, OverwritePolicy::Rename);
    let (active, callbacks) = active_and_callbacks();
    let result = run_job(&job, Some(2.0), &active, &callbacks).expect("rename ok");
    assert_eq!(result.status, JobStatus::Completed);
    let output = PathBuf::from(&result.output_path);
    assert_eq!(output, out_dir.join("tone-440hz (1).flac"));
    assert!(output.is_file());
    assert_eq!(std::fs::read(&primary).unwrap(), b"existing");
    let _ = std::fs::remove_file(&output);
    let _ = std::fs::remove_dir_all(&out_dir);
}

#[test]
fn replace_when_primary_exists() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..").join("tests").join("fixtures").join("tone-440hz.wav");
    if !fixture.is_file() { return; }

    let out_dir = std::env::temp_dir().join(format!("jwconverter-rep-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&out_dir).unwrap();
    let primary = out_dir.join("tone-440hz.flac");
    std::fs::write(&primary, b"existing").unwrap();
    let source_bytes = std::fs::read(&fixture).unwrap();

    let job = test_job(&fixture, &out_dir, OverwritePolicy::Replace);
    let (active, callbacks) = active_and_callbacks();
    let result = run_job(&job, Some(2.0), &active, &callbacks).expect("replace ok");
    assert_eq!(result.status, JobStatus::Completed);
    assert_eq!(PathBuf::from(&result.output_path), primary);
    assert!(primary.metadata().unwrap().len() > 0);
    assert_ne!(std::fs::read(&primary).unwrap(), b"existing");
    assert_eq!(std::fs::read(&fixture).unwrap(), source_bytes);
    let _ = std::fs::remove_dir_all(&out_dir);
}
```

- [ ] **Step 2: Run new tests — expect FAIL**

```powershell
cargo test --manifest-path c:\Users\JW\Desktop\projects\converter\src-tauri\Cargo.toml skip_when_primary_exists -- --nocapture
```

Expected: FAIL (always renames / completes today).

- [ ] **Step 3: Implement resolution in `run_job`**

After computing `stem` and `plan`, before creating temp:

```rust
use crate::engine::job::OverwritePolicy;

let primary_path =
    finalize::primary_final_path(&destination_dir, &stem, plan.extension);

let final_path = match job.overwrite_policy {
    OverwritePolicy::Rename => {
        finalize::unique_final_path(&destination_dir, &stem, plan.extension)
    }
    OverwritePolicy::Skip => {
        if primary_path.exists() {
            (callbacks.on_status)(JobStatus::Skipped);
            return Ok(ConversionResult {
                job_id: job.id.clone(),
                output_path: primary_path.to_string_lossy().into_owned(),
                status: JobStatus::Skipped,
            });
        }
        primary_path.clone()
    }
    OverwritePolicy::Replace => primary_path.clone(),
};

let allow_replace = matches!(job.overwrite_policy, OverwritePolicy::Replace);
```

Create temp as today. At finalize:

```rust
finalize::finalize_output_with_policy(&temp_path, &final_path, allow_replace).map_err(|error| {
    temp::cleanup_temp(&temp_path);
    error
})?;
```

Remove the old `finalize::finalize_output` call. Keep the existing early `final_path == temp_path` guard.

Update the original `converts_wav_to_flac_without_touching_source` job to include `overwrite_policy: OverwritePolicy::Rename` if not already done in Task 1.

- [ ] **Step 4: Run all runner tests**

```powershell
cargo test --manifest-path c:\Users\JW\Desktop\projects\converter\src-tauri\Cargo.toml engine::runner -- --nocapture
```

Expected: all PASS.

- [ ] **Step 5: Commit (only if user asked)**

```powershell
git add src-tauri/src/engine/runner.rs
git commit -m "feat: apply overwrite policy before encoding"
```

---

### Task 4: Queue — skipped counter + correct status emit

**Files:**
- Modify: `src-tauri/src/engine/queue.rs`

- [ ] **Step 1: Add `skipped` to state and events**

In `BatchEvent`:

```rust
pub skipped: u32,
```

In `QueueState`:

```rust
pub skipped: u32,
```

Default + `enqueue_batch` reset: `skipped: 0`.

In `snapshot_batch`:

```rust
skipped: queue.skipped,
```

- [ ] **Step 2: Handle `ConversionResult.status` on Ok**

Replace the `Ok(done) => { queue.completed += 1; ... status: Completed ...}` arm with:

```rust
Ok(done) => {
    let status = done.status;
    match status {
        JobStatus::Skipped => {
            queue.skipped += 1;
            emit_conversion(
                &app,
                ConversionEvent {
                    job_id: next.job.id,
                    source_path: Some(next.job.source_path),
                    status: JobStatus::Skipped,
                    percent: Some(100.0),
                    message: Some("Skipped — output already exists.".to_string()),
                    output_path: Some(done.output_path),
                },
            );
        }
        _ => {
            queue.completed += 1;
            emit_conversion(
                &app,
                ConversionEvent {
                    job_id: next.job.id,
                    source_path: Some(next.job.source_path),
                    status: JobStatus::Completed,
                    percent: Some(100.0),
                    message: Some("Conversion completed.".to_string()),
                    output_path: Some(done.output_path),
                },
            );
        }
    }
}
```

- [ ] **Step 3: Fix batch-finished cancelled heuristic**

Where status is chosen when queue empties:

```rust
let status = if queue.cancelled > 0
    && queue.completed == 0
    && queue.failed == 0
    && queue.skipped == 0
{
    BatchStatus::Cancelled
} else {
    BatchStatus::Completed
};
```

(All-skipped batches must report Completed, not Cancelled.)

- [ ] **Step 4: Compile**

```powershell
cargo check --manifest-path c:\Users\JW\Desktop\projects\converter\src-tauri\Cargo.toml
```

Expected: PASS.

- [ ] **Step 5: Commit (only if user asked)**

```powershell
git add src-tauri/src/engine/queue.rs
git commit -m "feat: track skipped jobs in batch events"
```

---

### Task 5: Frontend types + OverwritePicker

**Files:**
- Modify: `src/types/conversion.ts`
- Create: `src/components/OverwritePicker.tsx`
- Modify: `src/lib/tauri.ts`

- [ ] **Step 1: Extend types**

In `conversion.ts`:

```typescript
export type JobStatus =
  | "idle"
  | "analyzing"
  | "ready"
  | "queued"
  | "converting"
  | "verifying"
  | "completed"
  | "failed"
  | "cancelled"
  | "skipped";

export type OverwritePolicy = "rename" | "skip" | "replace";

export const OVERWRITE_POLICIES: ReadonlyArray<{
  value: OverwritePolicy;
  label: string;
}> = [
  { value: "rename", label: "Rename" },
  { value: "skip", label: "Skip" },
  { value: "replace", label: "Replace" },
];
```

- [ ] **Step 2: Create `OverwritePicker.tsx`**

Mirror `FormatPicker` structure:

```tsx
import { OVERWRITE_POLICIES, type OverwritePolicy } from "../types/conversion";

type OverwritePickerProps = {
  value: OverwritePolicy;
  disabled?: boolean;
  onChange: (policy: OverwritePolicy) => void;
};

export function OverwritePicker({
  value,
  disabled = false,
  onChange,
}: OverwritePickerProps) {
  return (
    <section
      aria-label="If file exists"
      className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-5"
    >
      <h2 className="text-sm font-semibold tracking-wide text-[var(--text-muted)] uppercase">
        If file exists
      </h2>
      <div className="mt-4 flex flex-wrap gap-2">
        {OVERWRITE_POLICIES.map((policy) => {
          const isSelected = value === policy.value;
          return (
            <button
              key={policy.value}
              type="button"
              disabled={disabled}
              aria-pressed={isSelected}
              onClick={() => onChange(policy.value)}
              className={[
                "rounded-lg border px-3.5 py-2 text-sm font-medium transition-colors",
                isSelected
                  ? "border-[var(--accent)] bg-[var(--accent-soft)] text-[var(--text)]"
                  : "border-[var(--border)] bg-[var(--surface-muted)] text-[var(--text)]",
                disabled
                  ? "cursor-not-allowed opacity-50"
                  : "hover:border-[var(--accent)]/60",
              ].join(" ")}
            >
              {policy.label}
            </button>
          );
        })}
      </div>
      <p className="mt-3 text-xs text-[var(--text-muted)]">
        Rename keeps both files. Skip leaves the existing file. Replace
        overwrites it after a successful convert.
      </p>
    </section>
  );
}
```

- [ ] **Step 3: Add field to `ConversionRequest` in `tauri.ts`**

```typescript
import type { /* existing */, OverwritePolicy } from "../types/conversion";

export interface ConversionRequest {
  sourcePath: string;
  destinationDir: string;
  outputFormat: OutputFormat;
  sourceDurationSeconds: number | null;
  relativeSubdir: string | null;
  overwritePolicy: OverwritePolicy;
}
```

- [ ] **Step 4: Typecheck**

```powershell
cd c:\Users\JW\Desktop\projects\converter
npm run typecheck
```

Expected: FAIL on hooks that construct `ConversionRequest` without `overwritePolicy`.

---

### Task 6: Wire UI + batch summary

**Files:**
- Modify: `src/hooks/useBatchConversion.ts`
- Modify: `src/hooks/useConversion.ts` (add `overwritePolicy: "rename"`)
- Modify: `src/views/ConverterView.tsx`
- Modify: `src/components/FileQueue.tsx`
- Modify: `src/components/ConversionProgress.tsx` (optional skipped label)
- Modify: `README.md`

- [ ] **Step 1: Pass policy from batch hook**

Update `BatchEvent`:

```typescript
skipped: number;
```

Update `convert` args to include `overwritePolicy: OverwritePolicy` and map it onto every request:

```typescript
overwritePolicy: args.overwritePolicy,
```

Import `OverwritePolicy`.

- [ ] **Step 2: Wire `ConverterView`**

```typescript
import { OverwritePicker } from "../components/OverwritePicker";
import type { OverwritePolicy } from "../types/conversion";

const [overwritePolicy, setOverwritePolicy] = useState<OverwritePolicy>("rename");
```

Place `<OverwritePicker value={overwritePolicy} disabled={batch.isBusy} onChange={setOverwritePolicy} />` immediately after `<FormatPicker ... />`.

In `batch.convert`, pass `overwritePolicy`.

Update batch summary string:

```typescript
const batchSummary = batch.batch
  ? `${batch.batch.completed} done · ${batch.batch.skipped} skipped · ${batch.batch.failed} failed · ${batch.batch.cancelled} cancelled · ${batch.batch.remaining} remaining · ${batch.batch.total} total`
  : null;
```

Include `"skipped"` in the `progressVisible` status list.

- [ ] **Step 3: FileQueue skipped label**

```typescript
case "skipped":
  return "Skipped";
```

- [ ] **Step 4: ConversionProgress label (optional)**

Add branch: `status === "skipped" ? "Skipped" : ...`

- [ ] **Step 5: README**

Change status line to **v0.1.0 overwrite** and add bullet:

- Overwrite policy: Rename (default) / Skip / Replace

Keep quality presets / more formats / packaging under “Not yet implemented”.

- [ ] **Step 6: Full verify**

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cd c:\Users\JW\Desktop\projects\converter
npm run typecheck
cd src-tauri
cargo test engine::runner fs_safety::finalize -- --nocapture
```

Expected: typecheck PASS; tests PASS.

- [ ] **Step 7: Manual smoke (dev app)**

```powershell
cd c:\Users\JW\Desktop\projects\converter
npm run tauri dev
```

1. Convert one file to Downloads (Rename) → success.
2. Convert same file again with Rename → `name (1).ext`.
3. Convert again with Skip → row shows Skipped; no new file.
4. Convert with Replace → same primary path updated.
5. Batch summary shows skipped count after a Skip run.

- [ ] **Step 8: Commit (only if user asked)**

```powershell
git add src src-tauri README.md docs
git commit -m "feat: overwrite policy picker (rename / skip / replace)"
```

---

## Spec coverage checklist

| Spec requirement | Task |
|---|---|
| Default Rename | 1, 5, 6 |
| Picker Rename/Skip/Replace only | 5, 6 |
| Per-batch policy on each job | 1, 4–6 |
| Resolve before encode | 3 |
| Skip: no FFmpeg, status skipped, outputPath = existing | 3, 4 |
| Replace after verify only | 2, 3 |
| Source never modified | 3 tests |
| Batch skipped count | 4, 6 |
| Phase `overwrite` | 1 |
| Unit tests rename/skip/replace | 2, 3 |
| Typecheck + cargo test | 6 |

## Out of scope (do not implement)

- Ask / Apply to all
- Persisting policy across restarts
- Preflight collision summary
