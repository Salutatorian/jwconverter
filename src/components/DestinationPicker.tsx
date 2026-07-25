type DestinationPickerProps = {
  destination: string | null;
  disabled?: boolean;
  onChooseFolder: () => void;
  onUseDownloads?: () => void;
  canUseDownloads?: boolean;
  onUseSourceFolder?: () => void;
  canUseSourceFolder?: boolean;
};

export function DestinationPicker({
  destination,
  disabled = false,
  onChooseFolder,
  onUseDownloads,
  canUseDownloads = false,
  onUseSourceFolder,
  canUseSourceFolder = false,
}: DestinationPickerProps) {
  return (
    <section
      aria-label="Output destination"
      className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-5"
    >
      <h2 className="text-sm font-semibold tracking-wide text-[var(--text-muted)] uppercase">
        Destination
      </h2>
      <p className="mt-3 truncate text-sm text-[var(--text)]" title={destination ?? undefined}>
        {destination ?? "No folder selected"}
      </p>
      <div className="mt-4 flex flex-wrap gap-2">
        <button
          type="button"
          disabled={disabled}
          onClick={onChooseFolder}
          className="rounded-lg border border-[var(--border)] bg-[var(--surface-muted)] px-3.5 py-2 text-sm font-medium text-[var(--text)] disabled:cursor-not-allowed disabled:opacity-50"
        >
          Choose folder
        </button>
        {onUseDownloads ? (
          <button
            type="button"
            disabled={disabled || !canUseDownloads}
            onClick={onUseDownloads}
            className="rounded-lg border border-[var(--border)] bg-[var(--surface-muted)] px-3.5 py-2 text-sm font-medium text-[var(--text)] disabled:cursor-not-allowed disabled:opacity-50"
          >
            Downloads
          </button>
        ) : null}
        {onUseSourceFolder ? (
          <button
            type="button"
            disabled={disabled || !canUseSourceFolder}
            onClick={onUseSourceFolder}
            className="rounded-lg border border-[var(--border)] bg-[var(--surface-muted)] px-3.5 py-2 text-sm font-medium text-[var(--text)] disabled:cursor-not-allowed disabled:opacity-50"
          >
            Same as source
          </button>
        ) : null}
      </div>
      <p className="mt-3 text-xs text-[var(--text-muted)]">
        Defaults to your Downloads folder. If the output name already exists, a
        new name like song (1).flac is used. Originals are never overwritten.
      </p>
    </section>
  );
}
