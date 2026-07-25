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
      return item.percent != null
        ? `Converting ${Math.round(item.percent)}%`
        : "Converting";
    case "verifying":
      return "Verifying";
    case "completed":
      return "Done";
    case "failed":
      return "Failed";
    case "cancelled":
      return "Cancelled";
    case "skipped":
      return "Skipped";
    default:
      return item.status;
  }
}

export function FileQueue({
  items,
  disabled = false,
  onRemove,
  onClear,
}: FileQueueProps) {
  if (items.length === 0) {
    return null;
  }

  return (
    <section aria-label="File queue" className="panel">
      <div className="flex items-center justify-between gap-3">
        <h2 className="panel-title">Files · {items.length}</h2>
        <button
          type="button"
          className="btn btn-ghost"
          disabled={disabled}
          onClick={onClear}
        >
          Clear
        </button>
      </div>

      <ul className="mt-3 max-h-56 space-y-1.5 overflow-y-auto">
        {items.map((item) => (
          <li
            key={item.localId}
            className="flex items-start justify-between gap-3 rounded-[10px] bg-[var(--surface-muted)] px-3 py-2"
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
              </p>
              {item.error ? (
                <p className="mt-1 text-xs text-[var(--danger)]">{item.error}</p>
              ) : null}
            </div>
            <button
              type="button"
              className="btn btn-ghost shrink-0 px-2 py-1 text-xs"
              disabled={
                disabled ||
                item.status === "converting" ||
                item.status === "verifying"
              }
              onClick={() => onRemove(item.localId)}
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
