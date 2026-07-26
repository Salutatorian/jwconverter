import { ImageIcon, Music2Icon } from "lucide-react";

type DropZoneProps = {
  mode: "audio" | "images";
  disabled?: boolean;
  active?: boolean;
  analyzing?: boolean;
};

export function DropZone({
  mode,
  disabled = false,
  active = false,
  analyzing = false,
}: DropZoneProps) {
  const ariaLabel =
    mode === "images"
      ? "Drop image files or folders here"
      : "Drop audio files or folders here";
  const title =
    mode === "images" ? "drop photos to convert" : "drop audio to convert";
  const hint =
    mode === "images"
      ? "JPEG · PNG · WebP · TIFF · BMP · GIF · AVIF · RAW · folders. HEIC import only."
      : "FLAC · WAV · MP3 · M4A · AAC · Opus · OGG · ALAC · AIFF · folders.";
  const Icon = mode === "images" ? ImageIcon : Music2Icon;

  return (
    <div
      role="region"
      aria-label={ariaLabel}
      aria-disabled={disabled}
      className={[
        "drop-zone flex min-h-48 flex-col items-center justify-center border border-dashed px-6 py-10 text-center transition-all duration-200",
        "rounded-[var(--radius)]",
        disabled
          ? "cursor-not-allowed border-[var(--border)] bg-[var(--surface-muted)] opacity-50"
          : active
            ? "scale-[1.01] border-[var(--text)] bg-[var(--accent-soft)]"
            : "border-[var(--border-strong)] bg-[var(--surface)] hover:border-[var(--text-muted)]",
      ].join(" ")}
    >
      <div
        className={[
          "mb-4 flex size-12 items-center justify-center rounded-2xl border transition-colors",
          active
            ? "border-[var(--text)] bg-[var(--accent)] text-[var(--accent-contrast)]"
            : "border-[var(--border)] bg-[var(--bg)] text-[var(--text-muted)]",
        ].join(" ")}
        aria-hidden
      >
        <Icon className="size-5" />
      </div>
      <p className="mono text-[0.95rem] font-medium tracking-tight text-[var(--text)]">
        {analyzing ? "analyzing…" : active ? "drop to add" : title}
      </p>
      <p className="mono mt-2 max-w-md text-xs leading-relaxed text-[var(--text-muted)]">
        {disabled
          ? "unavailable while converting"
          : analyzing
            ? "reading files…"
            : `or use the buttons below · ${hint}`}
      </p>
    </div>
  );
}
