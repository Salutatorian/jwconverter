import { OUTPUT_FORMATS, type OutputFormat } from "../types/conversion";

type FormatPickerProps = {
  value: OutputFormat;
  disabled?: boolean;
  onChange: (format: OutputFormat) => void;
};

export function FormatPicker({
  value,
  disabled = true,
  onChange,
}: FormatPickerProps) {
  return (
    <section
      aria-label="Output format"
      className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-5"
    >
      <h2 className="text-sm font-semibold tracking-wide text-[var(--text-muted)] uppercase">
        Output format
      </h2>
      <div className="mt-4 flex flex-wrap gap-2">
        {OUTPUT_FORMATS.map((format) => {
          const isSelected = value === format.value;
          const isDisabled = disabled || !format.enabled;
          return (
            <button
              key={format.value}
              type="button"
              disabled={isDisabled}
              aria-pressed={isSelected}
              onClick={() => onChange(format.value)}
              className={[
                "rounded-lg border px-3.5 py-2 text-sm font-medium transition-colors",
                isSelected
                  ? "border-[var(--accent)] bg-[var(--accent-soft)] text-[var(--text)]"
                  : "border-[var(--border)] bg-[var(--surface-muted)] text-[var(--text)]",
                isDisabled
                  ? "cursor-not-allowed opacity-50"
                  : "hover:border-[var(--accent)]/60",
              ].join(" ")}
            >
              {format.label}
            </button>
          );
        })}
      </div>
      <p className="mt-3 text-xs text-[var(--text-muted)]">
        Lossy defaults: MP3/AAC 192 kbps, Opus 160 kbps, OGG ~q5. Quality
        presets come later. AAC and ALAC both write .m4a.
      </p>
    </section>
  );
}
