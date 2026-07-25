import { QUALITY_PRESETS, type QualityPreset } from "../types/conversion";

type QualityPickerProps = {
  value: QualityPreset;
  disabled?: boolean;
  onChange: (preset: QualityPreset) => void;
};

export function QualityPicker({
  value,
  disabled = false,
  onChange,
}: QualityPickerProps) {
  return (
    <section aria-label="Quality" className="panel">
      <h2 className="panel-title">Quality</h2>
      <div className="chip-row">
        {QUALITY_PRESETS.map((preset) => (
          <button
            key={preset.value}
            type="button"
            className="chip"
            disabled={disabled}
            aria-pressed={value === preset.value}
            onClick={() => onChange(preset.value)}
          >
            {preset.label}
          </button>
        ))}
      </div>
    </section>
  );
}
