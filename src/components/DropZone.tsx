import { ImageIcon, Music2Icon } from "lucide-react";
import type { ReactNode } from "react";

type DropZoneProps = {
  mode: "audio" | "images";
  disabled?: boolean;
  active?: boolean;
  analyzing?: boolean;
  actions?: ReactNode;
};

export function DropZone({
  mode,
  disabled = false,
  active = false,
  analyzing = false,
  actions,
}: DropZoneProps) {
  const ariaLabel =
    mode === "images"
      ? "Drop image files or folders here"
      : "Drop audio files or folders here";
  const title =
    mode === "images" ? "Add photos to convert" : "Add audio to convert";
  const hint =
    mode === "images"
      ? "Drag & drop files or folders · JPEG, PNG, WebP and more"
      : "Drag & drop files or folders · FLAC, WAV, MP3 and more";
  const Icon = mode === "images" ? ImageIcon : Music2Icon;

  return (
    <div
      role="region"
      aria-label={ariaLabel}
      aria-disabled={disabled}
      className={[
        "drop-zone flex min-h-56 flex-col items-center justify-center border border-dashed px-6 py-9 text-center transition-all duration-200",
        "rounded-[var(--radius-lg)]",
        disabled
          ? "cursor-not-allowed border-[var(--border)] bg-[var(--surface-muted)] opacity-50"
          : active
            ? "scale-[1.01] border-[var(--text)] bg-[var(--accent-soft)]"
            : "border-[var(--border-strong)] bg-[var(--surface)] hover:border-[var(--text-muted)]",
      ].join(" ")}
    >
      <div
        className={[
          "mb-3 flex size-14 items-center justify-center rounded-2xl border transition-colors",
          active
            ? "border-[var(--text)] bg-[var(--accent)] text-[var(--accent-contrast)]"
            : "border-[var(--border)] bg-[var(--bg)] text-[var(--text-muted)]",
        ].join(" ")}
        aria-hidden
      >
        <Icon className="size-6" strokeWidth={1.75} />
      </div>
      <p className="text-[1.05rem] font-semibold tracking-tight text-[var(--text)]">
        {analyzing ? "Analyzing…" : active ? "Drop to add" : title}
      </p>
      <p className="mt-1.5 max-w-sm text-sm leading-snug text-[var(--text-muted)]">
        {disabled
          ? "Unavailable while converting"
          : analyzing
            ? "Reading files…"
            : hint}
      </p>
      {actions ? <div className="drop-zone-actions">{actions}</div> : null}
    </div>
  );
}
