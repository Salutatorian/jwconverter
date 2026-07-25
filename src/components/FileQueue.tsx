import type { QueueFileItem } from "../types/conversion";

type FileQueueProps = {
  items: QueueFileItem[];
  disabled?: boolean;
  onRemove: (localId: string) => void;
  onClear: () => void;
};

function statusLabel(item: QueueFileItem): string {
  switch (item.status) {
    case "analyzing":
      return "Analyzing";
    case "ready":
      return "Ready";
    case "queued":
      return "Queued";
    case "converting":
      return item.percent != null ? `Converting ${Math.round(item.percent)}%` : "Converting";
    case "verifying":
      return "Verifying";
    case "completed":
      return "Done";
    case "skipped":
      return "Skipped";
    case "failed":
      return "Failed";
    case "cancelled":
      return "Cancelled";
    default:
      return item.status;
  }
}

export function FileQueue({ items, disabled = false, onRemove, onClear }: FileQueueProps) {
  if (items.length === 0) {
    return (
      <section
        aria-label="File queue"
        className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-5"
      >
        <h2 className="text-sm font-semibold tracking-wide text-[var(--text-muted)] uppercase">
          Files
        </h2>
        <p className="mt-3 text-sm text-[var(--text-muted)]">
          No files added yet.
        </p>
      </section>
    );
  }

  return (
    <section
      aria-label="File queue"
      className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-5"
    >
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-sm font-semibold tracking-wide text-[var(--text-muted)] uppercase">
          Files ({items.length})
        </h2>
        <button
          type="button"
          disabled={disabled}
          onClick={onClear}
          className="rounded-lg border border-[var(--border)] px-3 py-1.5 text-xs font-medium text-[var(--text)] disabled:cursor-not-allowed disabled:opacity-50"
        >
          Clear
        </button>
      </div>

      <ul className="mt-4 max-h-64 space-y-2 overflow-y-auto">
        {items.map((item) => (
          <li
            key={item.localId}
            className="flex items-start justify-between gap-3 rounded-lg border border-[var(--border)] bg-[var(--surface-muted)] px-3 py-2.5"
          >
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm font-medium text-[var(--text)]">
                {item.filename}
              </p>
              <p className="mt-0.5 text-xs text-[var(--text-muted)]">
                {statusLabel(item)}
                {item.relativeSubdir ? ` · ${item.relativeSubdir}` : ""}
                {item.info?.durationSeconds != null
                  ? ` · ${Math.round(item.info.durationSeconds)}s`
                  : ""}
                {item.info?.codec ? ` · ${item.info.codec}` : ""}
              </p>
              {item.error ? (
                <p className="mt-1 text-xs text-red-300">{item.error}</p>
              ) : null}
              {item.outputPath ? (
                <p className="mt-1 truncate text-xs text-[var(--text-faint)]">
                  {item.outputPath}
                </p>
              ) : null}
            </div>
            <button
              type="button"
              disabled={disabled || item.status === "converting" || item.status === "verifying"}
              onClick={() => onRemove(item.localId)}
              className="shrink-0 rounded-md px-2 py-1 text-xs text-[var(--text-muted)] hover:text-[var(--text)] disabled:cursor-not-allowed disabled:opacity-40"
              aria-label={`Remove ${item.filename}`}
            >
              Remove
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
