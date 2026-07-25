import { OUTPUT_FORMATS, type OutputFormat } from "../types/conversion";

type FormatPickerProps = {
  value: OutputFormat;
  disabled?: boolean;
  onChange: (format: OutputFormat) => void;
};

export function FormatPicker({
  value,
  disabled = false,
  onChange,
}: FormatPickerProps) {
  return (
    <section aria-label="Output format" className="panel">
      <h2 className="panel-title">Format</h2>
      <div className="chip-row">
        {OUTPUT_FORMATS.map((format) => {
          const isDisabled = disabled || !format.enabled;
          return (
            <button
              key={format.value}
              type="button"
              className="chip"
              disabled={isDisabled}
              aria-pressed={value === format.value}
              onClick={() => onChange(format.value)}
            >
              {format.label}
            </button>
          );
        })}
      </div>
      <p className="panel-hint">AAC and ALAC both write .m4a files.</p>
    </section>
  );
}
