import { useEffect, useId, useRef } from "react";
import type { WhatsNewEntry } from "../lib/whatsNew";

type WhatsNewModalProps = {
  entry: WhatsNewEntry;
  onDismiss: () => void;
};

export function WhatsNewModal({ entry, onDismiss }: WhatsNewModalProps) {
  const titleId = useId();
  const okRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    okRef.current?.focus();

    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape" || event.key === "Enter") {
        onDismiss();
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [onDismiss]);

  return (
    <div className="modal-overlay" role="presentation">
      <button
        type="button"
        className="modal-backdrop"
        aria-label="Dismiss what's new"
        onClick={onDismiss}
      />
      <section
        className="modal-card"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <h2 id={titleId} className="text-lg font-semibold text-[var(--text)]">
          What's new in v{entry.version}
        </h2>

        {entry.changes.length > 0 ? (
          <div className="mt-4">
            <h3 className="panel-title">Changes</h3>
            <ul className="whats-new-list">
              {entry.changes.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
          </div>
        ) : null}

        {entry.debugs.length > 0 ? (
          <div className="mt-4">
            <h3 className="panel-title">Debugs</h3>
            <ul className="whats-new-list">
              {entry.debugs.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
          </div>
        ) : null}

        <div className="mt-5 flex justify-end">
          <button
            ref={okRef}
            type="button"
            className="btn btn-primary"
            onClick={onDismiss}
          >
            Okay
          </button>
        </div>
      </section>
    </div>
  );
}
