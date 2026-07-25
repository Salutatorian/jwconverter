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
        "flex min-h-44 flex-col items-center justify-center rounded-xl border border-dashed px-6 py-10 text-center transition-colors",
        disabled
          ? "cursor-not-allowed border-[var(--border)] bg-[var(--surface-muted)] opacity-70"
          : active
            ? "border-[var(--accent)] bg-[var(--accent-soft)]"
            : "border-[var(--accent)]/40 bg-[var(--surface)]",
      ].join(" ")}
    >
      <p className="text-base font-medium text-[var(--text)]">
        {analyzing ? "Analyzing…" : "Drop audio files or folders here"}
      </p>
      <p className="mt-2 max-w-sm text-sm text-[var(--text-muted)]">
        {disabled
          ? "File import is unavailable while converting."
          : "Folders are scanned recursively. Folder structure is kept in the output."}
      </p>
    </div>
  );
}
