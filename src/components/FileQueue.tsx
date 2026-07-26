import {
  CheckIcon,
  ClockIcon,
  FileTextIcon,
  FileWarningIcon,
  RefreshCwIcon,
  XIcon,
} from "lucide-react";
import {
  Attachment,
  AttachmentAction,
  AttachmentActions,
  AttachmentContent,
  AttachmentDescription,
  AttachmentMedia,
  AttachmentTitle,
  type AttachmentState,
} from "./ui/attachment";
import { Spinner } from "./ui/spinner";
import { formatSourceSummary } from "../lib/format";
import type { JobStatus, QueueFileItem } from "../types/conversion";

type FileQueueProps = {
  items: QueueFileItem[];
  disabled?: boolean;
  onRemove: (localId: string) => void;
  onClear: () => void;
  onRetry?: (localId: string) => void;
};

function attachmentState(status: JobStatus): AttachmentState {
  switch (status) {
    case "converting":
      return "uploading";
    case "analyzing":
    case "verifying":
      return "processing";
    case "failed":
      return "error";
    case "completed":
      return "done";
    default:
      return "idle";
  }
}

function description(item: QueueFileItem): string {
  switch (item.status) {
    case "analyzing":
      return "Analyzing…";
    case "ready":
      return item.info
        ? `Ready · ${formatSourceSummary(item.info)}`
        : "Ready to convert";
    case "queued":
      return "Queued";
    case "converting":
      return item.percent != null
        ? `Converting · ${Math.round(item.percent)}%`
        : "Converting…";
    case "verifying":
      return "Verifying…";
    case "completed": {
      const size = item.info?.fileSizeBytes;
      if (size != null) {
        const mb = size / (1024 * 1024);
        return mb >= 1
          ? `Done · ${mb.toFixed(1)} MB`
          : `Done · ${Math.round(size / 1024)} KB`;
      }
      return "Done";
    }
    case "failed":
      return item.error ?? "Conversion failed. Try again.";
    case "cancelled":
      return "Cancelled";
    case "skipped":
      return "Skipped — output already exists";
    default:
      return item.status;
  }
}

function MediaIcon({ status }: { status: JobStatus }) {
  switch (attachmentState(status)) {
    case "uploading":
      return <Spinner />;
    case "processing":
      return <FileTextIcon />;
    case "error":
      return <FileWarningIcon />;
    case "done":
      return <CheckIcon />;
    default:
      return <ClockIcon />;
  }
}

export function FileQueue({
  items,
  disabled = false,
  onRemove,
  onClear,
  onRetry,
}: FileQueueProps) {
  if (items.length === 0) {
    return null;
  }

  return (
    <section aria-label="File queue" className="flex flex-col gap-2">
      <div className="flex items-center justify-between gap-3 px-0.5">
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

      <div className="flex max-h-64 flex-col gap-2 overflow-y-auto pr-0.5">
        {items.map((item) => {
          const state = attachmentState(item.status);
          const busy =
            item.status === "converting" || item.status === "verifying";
          return (
            <Attachment key={item.localId} state={state} className="w-full">
              <AttachmentMedia>
                <MediaIcon status={item.status} />
              </AttachmentMedia>
              <AttachmentContent>
                <AttachmentTitle>{item.filename}</AttachmentTitle>
                <AttachmentDescription>
                  {description(item)}
                  {item.relativeSubdir ? ` · ${item.relativeSubdir}` : ""}
                </AttachmentDescription>
              </AttachmentContent>
              <AttachmentActions>
                {state === "error" && onRetry ? (
                  <AttachmentAction
                    aria-label={`Retry ${item.filename}`}
                    disabled={disabled}
                    onClick={() => onRetry(item.localId)}
                  >
                    <RefreshCwIcon />
                  </AttachmentAction>
                ) : null}
                <AttachmentAction
                  aria-label={
                    busy ? `Cancel ${item.filename}` : `Remove ${item.filename}`
                  }
                  disabled={disabled || busy}
                  onClick={() => onRemove(item.localId)}
                >
                  <XIcon />
                </AttachmentAction>
              </AttachmentActions>
            </Attachment>
          );
        })}
      </div>
    </section>
  );
}
