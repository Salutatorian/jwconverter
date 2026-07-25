import { OVERWRITE_POLICIES, type OverwritePolicy } from "../types/conversion";

type OverwritePickerProps = {
  value: OverwritePolicy;
  disabled?: boolean;
  onChange: (policy: OverwritePolicy) => void;
};

export function OverwritePicker({
  value,
  disabled = true,
  onChange,
}: OverwritePickerProps) {
  return (
    <section
      aria-label="If file exists"
      className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-5"
    >
      <h2 className="text-sm font-semibold tracking-wide text-[var(--text-muted)] uppercase">
        If file exists
      </h2>
      <div className="mt-4 flex flex-wrap gap-2">
        {OVERWRITE_POLICIES.map((policy) => {
          const isSelected = value === policy.value;
          const isDisabled = disabled;
          return (
            <button
              key={policy.value}
              type="button"
              disabled={isDisabled}
              aria-pressed={isSelected}
              onClick={() => onChange(policy.value)}
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
              {policy.label}
            </button>
          );
        })}
      </div>
      <p className="mt-3 text-xs text-[var(--text-muted)]">
        Rename keeps both files. Skip leaves the existing file. Replace
        overwrites it after a successful convert.
      </p>
    </section>
  );
}
