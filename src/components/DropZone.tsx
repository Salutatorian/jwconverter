type DropZoneProps = {
  disabled?: boolean;
  active?: boolean;
  analyzing?: boolean;
};

export function DropZone({
  disabled = false,
  active = false,
  analyzing = false,
}: DropZoneProps) {
  return (
    <div
      role="region"
      aria-label="Drop audio files or folders here"
      aria-disabled={disabled}
      className={[
        "flex min-h-40 flex-col items-center justify-center border border-dashed px-6 py-9 text-center transition-colors",
        "rounded-[var(--radius)]",
        disabled
          ? "cursor-not-allowed border-[var(--border)] bg-[var(--surface-muted)] opacity-60"
          : active
            ? "border-[var(--accent)] bg-[var(--accent-soft)]"
            : "border-[var(--border-strong)] bg-[var(--surface)]",
      ].join(" ")}
    >
      <p className="text-[0.95rem] font-semibold tracking-tight text-[var(--text)]">
        {analyzing ? "Analyzing…" : "Drop files or folders"}
      </p>
      <p className="mt-1.5 max-w-xs text-sm text-[var(--text-muted)]">
        {disabled
          ? "Unavailable while converting."
          : "Or use the buttons below. Folder structure is preserved."}
      </p>
    </div>
  );
}
