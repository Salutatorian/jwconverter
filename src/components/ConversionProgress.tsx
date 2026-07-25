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
      ? "Verifying…"
      : status === "queued"
        ? "Starting…"
        : status === "completed"
          ? "Completed"
          : status === "cancelled"
            ? "Cancelled"
            : status === "failed"
              ? "Failed"
              : status === "skipped"
                ? "Skipped"
                : status === "running"
                  ? "Converting…"
                  : "Converting…";

  return (
    <section aria-label="Conversion progress" className="panel">
      <div className="flex items-center justify-between gap-3">
        <h2 className="panel-title">Progress</h2>
        {cancellable ? (
          <button type="button" className="btn btn-ghost" onClick={onCancel}>
            Cancel
          </button>
        ) : null}
      </div>

      <p className="mt-3 text-sm text-[var(--text)]">
        {batchSummary ?? label}
      </p>

      {showBar ? (
        <div
          className="mt-3 h-1.5 overflow-hidden rounded-full bg-[var(--surface-muted)]"
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

      {message ? (
        <p className="mt-2 text-sm text-[var(--text-muted)]">{message}</p>
      ) : null}

      {outputPath ? (
        <p className="mt-2 break-all text-xs text-[var(--text-faint)]">
          Last saved: {outputPath}
        </p>
      ) : null}
    </section>
  );
}
