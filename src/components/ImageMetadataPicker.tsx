type ImageMetadataPickerProps = {
  preserveMetadata: boolean;
  disabled?: boolean;
  onPreserveMetadataChange: (value: boolean) => void;
};

export function ImageMetadataPicker({
  preserveMetadata,
  disabled = false,
  onPreserveMetadataChange,
}: ImageMetadataPickerProps) {
  return (
    <section aria-label="Metadata" className="panel">
      <h2 className="panel-title">Metadata</h2>
      <div className="mt-3 flex flex-col gap-2.5">
        <label className="flex items-start gap-2.5 text-sm text-[var(--text)]">
          <input
            type="checkbox"
            className="mt-0.5"
            checked={preserveMetadata}
            disabled={disabled}
            onChange={(event) => {
              onPreserveMetadataChange(event.target.checked);
            }}
          />
          <span>
            <span className="font-medium">Preserve metadata</span>
            <span className="mt-0.5 block text-xs text-[var(--text-muted)]">
              Keep EXIF, ICC profiles, and comments when the output format
              supports them.
            </span>
          </span>
        </label>
      </div>
    </section>
  );
}
