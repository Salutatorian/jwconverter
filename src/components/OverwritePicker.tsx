import { OVERWRITE_POLICIES, type OverwritePolicy } from "../types/conversion";

type OverwritePickerProps = {
  value: OverwritePolicy;
  disabled?: boolean;
  onChange: (policy: OverwritePolicy) => void;
};

export function OverwritePicker({
  value,
  disabled = false,
  onChange,
}: OverwritePickerProps) {
  return (
    <section aria-label="If file exists" className="panel panel-compact">
      <h2 className="panel-title">If file exists</h2>
      <div className="chip-row">
        {OVERWRITE_POLICIES.map((policy) => (
          <button
            key={policy.value}
            type="button"
            className="chip"
            disabled={disabled}
            title={
              policy.value === "rename"
                ? "Keep both files"
                : policy.value === "skip"
                  ? "Leave the existing file"
                  : "Overwrite after a successful convert"
            }
            aria-pressed={value === policy.value}
            onClick={() => onChange(policy.value)}
          >
            {policy.label}
          </button>
        ))}
      </div>
    </section>
  );
}
