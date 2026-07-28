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
    <section aria-label="Metadata" className="panel panel-compact">
      <h2 className="panel-title">Metadata</h2>
      <div className="mt-2.5">
        <label
          className="inline-flex items-center gap-2 text-sm text-[var(--text)]"
          title="Keep EXIF / ICC when the format supports it"
        >
          <input
            type="checkbox"
            checked={preserveMetadata}
            disabled={disabled}
            onChange={(event) => {
              onPreserveMetadataChange(event.target.checked);
            }}
          />
          <span>Preserve metadata</span>
        </label>
      </div>
    </section>
  );
}
