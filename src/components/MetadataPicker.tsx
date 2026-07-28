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
    <section aria-label="Metadata" className="panel panel-compact">
      <h2 className="panel-title">Metadata</h2>
      <div className="mt-2.5 flex flex-wrap gap-x-5 gap-y-2">
        <label className="inline-flex items-center gap-2 text-sm text-[var(--text)]">
          <input
            type="checkbox"
            checked={preserveTags}
            disabled={disabled}
            onChange={(event) => {
              onPreserveTagsChange(event.target.checked);
            }}
          />
          <span>Tags</span>
        </label>
        <label
          className="inline-flex items-center gap-2 text-sm text-[var(--text)]"
          title={
            coverSupported
              ? "Keep embedded album art when supported"
              : "This format doesn’t support cover art"
          }
        >
          <input
            type="checkbox"
            checked={preserveCover && coverSupported}
            disabled={disabled || !coverSupported}
            onChange={(event) => {
              onPreserveCoverChange(event.target.checked);
            }}
          />
          <span>Cover art</span>
        </label>
      </div>
    </section>
  );
}
