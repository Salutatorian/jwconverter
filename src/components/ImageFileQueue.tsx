import type { ImageQueueFileItem } from "../types/image";

type ImageFileQueueProps = {
  items: ImageQueueFileItem[];
  disabled?: boolean;
  onRemove: (localId: string) => void;
  onClear: () => void;
};

function summary(item: ImageQueueFileItem): string {
  const info = item.info;
  if (!info) {
    return "";
  }
  const parts: string[] = [];
  if (info.width != null && info.height != null) {
    parts.push(`${info.width}×${info.height}`);
  }
  if (info.format) {
    parts.push(info.format);
  }
  if (info.fileSizeBytes != null) {
    const mb = info.fileSizeBytes / (1024 * 1024);
    parts.push(mb >= 1 ? `${mb.toFixed(1)} MB` : `${Math.round(info.fileSizeBytes / 1024)} KB`);
  }
  return parts.join(" · ");
}

function statusLabel(item: ImageQueueFileItem): string {
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

export function ImageFileQueue({
  items,
  disabled = false,
  onRemove,
  onClear,
}: ImageFileQueueProps) {
  if (items.length === 0) {
    return null;
  }

  return (
    <section aria-label="Image queue" className="panel">
      <div className="flex items-center justify-between gap-3">
        <h2 className="panel-title">Images · {items.length}</h2>
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
                {item.info ? ` · ${summary(item)}` : ""}
              </p>
              {item.error ? (
                <p className="mt-1 text-xs text-[var(--danger)]">{item.error}</p>
              ) : null}
            </div>
            <button
              type="button"
              className="btn btn-ghost shrink-0"
              disabled={disabled}
              onClick={() => onRemove(item.localId)}
            >
              Remove
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
