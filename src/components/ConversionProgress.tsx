type ConversionProgressProps = {
  visible: boolean;
  status: string;
  percent: number | null;
  message: string | null;
  outputPath: string | null;
  batchSummary?: string | null;
  onCancel?: () => void;
  cancellable?: boolean;
};

export function ConversionProgress({
  visible,
  status,
  percent,
  message,
  outputPath,
  batchSummary,
  onCancel,
  cancellable = false,
}: ConversionProgressProps) {
  if (!visible) {
    return null;
  }

  const showBar = percent != null;
  const label =
    status === "verifying"
      ? "Verifying output…"
      : status === "queued"
        ? "Starting…"
        : status === "completed"
          ? "Completed"
          : status === "skipped"
            ? "Skipped"
            : status === "cancelled"
            ? "Cancelled"
            : status === "failed"
              ? "Failed"
              : status === "running"
                ? "Batch running…"
                : "Converting…";

  return (
    <section
      aria-label="Conversion progress"
      className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-5"
    >
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-sm font-semibold tracking-wide text-[var(--text-muted)] uppercase">
          Progress
        </h2>
        {cancellable ? (
          <button
            type="button"
            onClick={onCancel}
            className="rounded-lg border border-[var(--border)] px-3 py-1.5 text-xs font-medium text-[var(--text)] hover:border-red-400/50"
          >
            Cancel queue
          </button>
        ) : null}
      </div>

      {batchSummary ? (
        <p className="mt-3 text-sm text-[var(--text)]">{batchSummary}</p>
      ) : (
        <p className="mt-3 text-sm text-[var(--text)]">{label}</p>
      )}

      {showBar ? (
        <div
          className="mt-3 h-2 overflow-hidden rounded-full bg-[var(--surface-muted)]"
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(percent)}
        >
          <div
            className="h-full rounded-full bg-[var(--accent)] transition-[width] duration-150"
            style={{ width: `${Math.min(100, Math.max(0, percent))}%` }}
          />
        </div>
      ) : null}

      {percent != null ? (
        <p className="mt-2 text-xs text-[var(--text-muted)]">
          Current file {Math.round(percent)}%
        </p>
      ) : null}

      {message ? (
        <p className="mt-3 text-sm text-[var(--text-muted)]">{message}</p>
      ) : null}

      {outputPath ? (
        <p className="mt-2 break-all text-xs text-[var(--text-faint)]">
          Last saved: {outputPath}
        </p>
      ) : null}
    </section>
  );
}
