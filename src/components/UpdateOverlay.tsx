import type { UseUpdaterResult } from "../hooks/useUpdater";

type UpdateOverlayProps = {
  updater: UseUpdaterResult;
};

export function UpdateOverlay({ updater }: UpdateOverlayProps) {
  const {
    status,
    availableVersion,
    error,
    downloadPercent,
    installUpdate,
    dismissBlockingOverlay,
  } = updater;

  const percent = downloadPercent ?? 0;
  const isDownloading = status === "downloading";
  const isRestarting = isDownloading && downloadPercent === 100;
  const failed = Boolean(error) && status === "available";

  return (
    <div className="update-overlay" role="alertdialog" aria-modal="true" aria-labelledby="update-overlay-title">
      <div className="update-overlay-card">
        <p className="update-overlay-kicker">JW Converter</p>
        <h2 id="update-overlay-title" className="update-overlay-title">
          {failed ? "Update failed" : isRestarting ? "Restarting…" : "Updating…"}
        </h2>
        {availableVersion ? (
          <p className="update-overlay-version">v{availableVersion}</p>
        ) : null}

        {!failed ? (
          <>
            <div
              className="update-progress-track"
              role="progressbar"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={percent}
              aria-label="Update download progress"
            >
              <div
                className="update-progress-fill"
                style={{ width: `${Math.min(100, Math.max(0, percent))}%` }}
              />
            </div>
            <p className="update-progress-percent">
              {downloadPercent == null && isDownloading
                ? "Starting…"
                : `${percent}%`}
            </p>
          </>
        ) : null}

        {failed ? (
          <div className="update-overlay-error">
            <p role="alert">{error}</p>
            <div className="update-overlay-actions">
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => {
                  void installUpdate();
                }}
              >
                Retry
              </button>
              <button
                type="button"
                className="btn btn-secondary"
                onClick={dismissBlockingOverlay}
              >
                Continue without updating
              </button>
            </div>
          </div>
        ) : null}
      </div>
    </div>
  );
}
