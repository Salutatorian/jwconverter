type MetadataPickerProps = {
  preserveTags: boolean;
  preserveCover: boolean;
  coverSupported: boolean;
  disabled?: boolean;
  onPreserveTagsChange: (value: boolean) => void;
  onPreserveCoverChange: (value: boolean) => void;
};

export function MetadataPicker({
  preserveTags,
  preserveCover,
  coverSupported,
  disabled = false,
  onPreserveTagsChange,
  onPreserveCoverChange,
}: MetadataPickerProps) {
  return (
    <section aria-label="Metadata" className="panel">
      <h2 className="panel-title">Metadata</h2>
      <div className="mt-3 flex flex-col gap-2.5">
        <label className="flex items-start gap-2.5 text-sm text-[var(--text)]">
          <input
            type="checkbox"
            className="mt-0.5"
            checked={preserveTags}
            disabled={disabled}
            onChange={(event) => {
              onPreserveTagsChange(event.target.checked);
            }}
          />
          <span>
            <span className="font-medium">Preserve tags</span>
            <span className="mt-0.5 block text-xs text-[var(--text-muted)]">
              Title, artist, album, and other container metadata when possible.
            </span>
          </span>
        </label>
        <label className="flex items-start gap-2.5 text-sm text-[var(--text)]">
          <input
            type="checkbox"
            className="mt-0.5"
            checked={preserveCover && coverSupported}
            disabled={disabled || !coverSupported}
            onChange={(event) => {
              onPreserveCoverChange(event.target.checked);
            }}
          />
          <span>
            <span className="font-medium">Preserve cover artwork</span>
            <span className="mt-0.5 block text-xs text-[var(--text-muted)]">
              {coverSupported
                ? "Keep embedded album art when the destination format supports it."
                : "This format doesn't support embedded cover art."}
            </span>
          </span>
        </label>
      </div>
    </section>
  );
}
