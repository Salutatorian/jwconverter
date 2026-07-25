import { useEffect, useId, useRef } from "react";
import type { PreflightReport } from "../lib/tauri";

type PreflightModalProps = {
  report: PreflightReport;
  onCancel: () => void;
  onContinue?: () => void;
};

function formatBytes(bytes: number | null | undefined): string {
  if (bytes == null || !Number.isFinite(bytes) || bytes < 0) {
    return "—";
  }
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const digits = unit === 0 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(digits)} ${units[unit]}`;
}

export function PreflightModal({
  report,
  onCancel,
  onContinue,
}: PreflightModalProps) {
  const titleId = useId();
  const primaryRef = useRef<HTMLButtonElement>(null);
  const hardBlock = report.diskBlocked;

  useEffect(() => {
    primaryRef.current?.focus();

    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onCancel();
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [onCancel]);

  return (
    <div className="modal-overlay" role="presentation">
      <button
        type="button"
        className="modal-backdrop"
        aria-label="Cancel"
        onClick={onCancel}
      />
      <section
        className="modal-card"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <h2 id={titleId} className="text-lg font-semibold text-[var(--text)]">
          {hardBlock ? "Not enough disk space" : "Before you convert"}
        </h2>

        <dl className="mt-4 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-sm">
          <dt className="text-[var(--text-muted)]">Files</dt>
          <dd className="text-[var(--text)]">
            {report.fileCount}
            {report.skippedExisting > 0
              ? ` (${report.skippedExisting} skipped — already exist)`
              : null}
          </dd>
          <dt className="text-[var(--text-muted)]">Source</dt>
          <dd className="text-[var(--text)]">
            {formatBytes(report.sourceBytes)}
          </dd>
          <dt className="text-[var(--text-muted)]">Estimated output</dt>
          <dd className="text-[var(--text)]">
            ~{formatBytes(report.estimatedOutputBytes)}
          </dd>
          <dt className="text-[var(--text-muted)]">Free space</dt>
          <dd className="text-[var(--text)]">
            {formatBytes(report.freeBytes)}
          </dd>
        </dl>

        {hardBlock ? (
          <p className="mt-4 text-sm text-[var(--text)]">
            Need about {formatBytes(report.requiredBytes)} (estimate + margin).
            Free up space or choose another destination.
          </p>
        ) : null}

        {report.warnings.length > 0 ? (
          <ul className="mt-4 flex flex-col gap-2 text-sm text-[var(--text)]">
            {report.warnings.map((warning) => (
              <li
                key={warning.kind}
                className="rounded border border-[var(--border)] bg-[var(--surface-muted)] px-3 py-2"
              >
                {warning.message}
              </li>
            ))}
          </ul>
        ) : null}

        <div className="mt-5 flex justify-end gap-2">
          <button
            ref={hardBlock ? primaryRef : undefined}
            type="button"
            className="btn"
            onClick={onCancel}
          >
            {hardBlock ? "Close" : "Cancel"}
          </button>
          {!hardBlock && onContinue ? (
            <button
              ref={primaryRef}
              type="button"
              className="btn btn-primary"
              onClick={onContinue}
            >
              Continue anyway
            </button>
          ) : null}
        </div>
      </section>
    </div>
  );
}
