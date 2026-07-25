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
    <section
      aria-label="Quality"
      className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-5"
    >
      <h2 className="text-sm font-semibold tracking-wide text-[var(--text-muted)] uppercase">
        Quality
      </h2>
      <div className="mt-4 flex flex-wrap gap-2">
        {QUALITY_PRESETS.map((preset) => {
          const isSelected = value === preset.value;
          return (
            <button
              key={preset.value}
              type="button"
              disabled={disabled}
              aria-pressed={isSelected}
              onClick={() => onChange(preset.value)}
              className={[
                "rounded-lg border px-3.5 py-2 text-sm font-medium transition-colors",
                isSelected
                  ? "border-[var(--accent)] bg-[var(--accent-soft)] text-[var(--text)]"
                  : "border-[var(--border)] bg-[var(--surface-muted)] text-[var(--text)]",
                disabled
                  ? "cursor-not-allowed opacity-50"
                  : "hover:border-[var(--accent)]/60",
              ].join(" ")}
            >
              {preset.label}
            </button>
          );
        })}
      </div>
      <p className="mt-3 text-xs text-[var(--text-muted)]">
        Medium matches the previous defaults (e.g. MP3/AAC 192 kbps). Applies
        only to lossy formats.
      </p>
    </section>
  );
}
