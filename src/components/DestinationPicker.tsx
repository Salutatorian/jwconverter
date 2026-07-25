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
    <section aria-label="Output destination" className="panel">
      <h2 className="panel-title">Destination</h2>
      <p
        className="mt-3 truncate text-sm text-[var(--text)]"
        title={destination ?? undefined}
      >
        {destination ?? "No folder selected"}
      </p>
      <div className="chip-row">
        <button
          type="button"
          className="btn btn-secondary"
          disabled={disabled}
          onClick={onChooseFolder}
        >
          Choose folder
        </button>
        {onUseDownloads ? (
          <button
            type="button"
            className="btn btn-secondary"
            disabled={disabled || !canUseDownloads}
            onClick={onUseDownloads}
          >
            Downloads
          </button>
        ) : null}
        {onUseSourceFolder ? (
          <button
            type="button"
            className="btn btn-secondary"
            disabled={disabled || !canUseSourceFolder}
            onClick={onUseSourceFolder}
          >
            Same as source
          </button>
        ) : null}
      </div>
    </section>
  );
}
