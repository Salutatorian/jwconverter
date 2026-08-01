type PrimaryActionBarProps = {
  label: string;
  busyLabel?: string;
  busy?: boolean;
  disabled?: boolean;
  hint?: string | null;
  onAction: () => void;
};

/** Sticky primary CTA so Convert / Download is never buried in the page. */
export function PrimaryActionBar({
  label,
  busyLabel,
  busy = false,
  disabled = false,
  hint = null,
  onAction,
}: PrimaryActionBarProps) {
  return (
    <div className="primary-action-bar" role="region" aria-label="Primary action">
      <button
        type="button"
        className="btn btn-primary btn-primary-hero"
        disabled={disabled || busy}
        onClick={onAction}
      >
        {busy ? (busyLabel ?? label) : label}
      </button>
      {hint ? <p className="primary-action-hint">{hint}</p> : null}
    </div>
  );
}
