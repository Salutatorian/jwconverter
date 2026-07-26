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
    mode === "images" ? "Add photos to convert" : "Add audio to convert";
  const hint =
    mode === "images"
      ? "JPEG, PNG, WebP, TIFF, BMP, GIF, AVIF, RAW — or whole folders. HEIC import supported (export not available yet)."
      : "FLAC, WAV, MP3, M4A, AAC, Opus, OGG, ALAC, AIFF — or whole folders. Structure is preserved.";

  return (
    <div
      role="region"
      aria-label={ariaLabel}
      aria-disabled={disabled}
      className={[
        "flex min-h-44 flex-col items-center justify-center border border-dashed px-6 py-9 text-center transition-colors",
        "rounded-[var(--radius)]",
        disabled
          ? "cursor-not-allowed border-[var(--border)] bg-[var(--surface-muted)] opacity-60"
          : active
            ? "border-[var(--accent)] bg-[var(--accent-soft)]"
            : "border-[var(--border-strong)] bg-[var(--surface)]",
      ].join(" ")}
    >
      <p className="text-[0.95rem] font-semibold tracking-tight text-[var(--text)]">
        {analyzing ? "Analyzing…" : active ? "Drop to add" : title}
      </p>
      <p className="mt-1.5 max-w-sm text-sm text-[var(--text-muted)]">
        {disabled
          ? "Unavailable while converting."
          : analyzing
            ? "Reading files…"
            : `Or use the buttons below. ${hint}`}
      </p>
    </div>
  );
}
