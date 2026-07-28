import { BIT_DEPTH_PRESETS, type BitDepthPreset } from "../types/conversion";

type BitDepthPickerProps = {
  value: BitDepthPreset;
  disabled?: boolean;
  onChange: (preset: BitDepthPreset) => void;
};

export function BitDepthPicker({
  value,
  disabled = false,
  onChange,
}: BitDepthPickerProps) {
  return (
    <section aria-label="Bit depth" className="panel panel-compact">
      <h2 className="panel-title">Bit depth</h2>
      <div className="chip-row">
        {BIT_DEPTH_PRESETS.map((preset) => (
          <button
            key={preset.value}
            type="button"
            className="chip"
            disabled={disabled}
            title={
              preset.value === "original"
                ? "Keep source depth when possible"
                : undefined
            }
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
