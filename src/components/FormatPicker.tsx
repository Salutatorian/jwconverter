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
    <section aria-label="Output format" className="panel panel-compact">
      <h2 className="panel-title">Output format</h2>
      <div className="chip-row">
        {OUTPUT_FORMATS.map((format) => {
          const isDisabled = disabled || !format.enabled;
          return (
            <button
              key={format.value}
              type="button"
              className="chip"
              disabled={isDisabled}
              title={
                format.value === "m4a"
                  ? "AAC in .m4a"
                  : format.value === "aac"
                    ? "Raw AAC (ADTS)"
                    : format.value === "alac"
                      ? "ALAC in .m4a"
                      : format.value === "ogg"
                        ? "Vorbis in .ogg"
                        : undefined
              }
              aria-pressed={value === format.value}
              onClick={() => onChange(format.value)}
            >
              {format.label}
            </button>
          );
        })}
      </div>
    </section>
  );
}
