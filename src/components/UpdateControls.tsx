import type { UseUpdaterResult } from "../hooks/useUpdater";

type UpdateControlsProps = {
  updater: UseUpdaterResult;
};

export function UpdateControls({ updater }: UpdateControlsProps) {
  const {
    status,
    availableVersion,
    error,
    downloadPercent,
    installMode,
    checkForUpdates,
    installUpdate,
  } = updater;

  const isBusy = status === "checking" || status === "downloading";
  const showProgress = status === "checking" || status === "downloading";
  const percent = downloadPercent ?? 0;
  const isIndeterminate = status === "checking" || downloadPercent == null;
  const isManual = installMode === "manual";

  return (
    <div className="flex flex-col gap-2">
      {status === "available" || status === "downloading" ? (
        <p className="text-xs text-[var(--text-muted)]">
          {status === "downloading"
            ? `Downloading v${availableVersion ?? ""}${
                downloadPercent != null ? ` · ${downloadPercent}%` : ""
              }. The app will restart when done.`
            : isManual
              ? `Update available: v${availableVersion}. Download the installer for your OS, then replace this app (Mac/Linux are not auto-installed yet).`
              : `Update available: v${availableVersion}. It also installs automatically the next time you open the app.`}
        </p>
      ) : null}

      {status === "error" && error ? (
        <p className="text-xs text-red-300" role="alert">
          Update failed: {error}
        </p>
      ) : null}

      {status === "available" && error ? (
        <p className="text-xs text-red-300" role="alert">
          Last update attempt failed: {error}
        </p>
      ) : null}

      {status === "upToDate" ? (
        <p className="text-xs text-[var(--text-faint)]">You're up to date</p>
      ) : null}

      {status === "idle" || status === "checking" ? (
        <p className="text-xs text-[var(--text-faint)]">
          {status === "checking"
            ? "Checking for updates…"
            : "Checks on launch and every few hours."}
        </p>
      ) : null}

      {showProgress ? (
        <div className="settings-update-progress">
          <div
            className={
              isIndeterminate
                ? "update-progress-track update-progress-track--compact update-progress-track--indeterminate"
                : "update-progress-track update-progress-track--compact"
            }
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={isIndeterminate ? undefined : percent}
            aria-busy={isIndeterminate || undefined}
            aria-label={
              status === "checking"
                ? "Checking for updates"
                : "Update download progress"
            }
          >
            <div
              className={
                isIndeterminate
                  ? "update-progress-fill update-progress-fill--indeterminate"
                  : "update-progress-fill"
              }
              style={
                isIndeterminate
                  ? undefined
                  : { width: `${Math.min(100, Math.max(0, percent))}%` }
              }
            />
          </div>
          {status === "downloading" ? (
            <p className="settings-update-progress-label">
              {downloadPercent == null ? "Starting…" : `${percent}%`}
            </p>
          ) : null}
        </div>
      ) : null}

      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          className="btn btn-secondary"
          disabled={isBusy}
          onClick={() => {
            void checkForUpdates();
          }}
        >
          {status === "checking" ? "Checking for updates…" : "Check for updates"}
        </button>
        <button
          type="button"
          className="btn btn-primary"
          disabled={status !== "available" || isBusy}
          onClick={() => {
            void installUpdate();
          }}
        >
          {status === "downloading"
            ? "Updating…"
            : isManual
              ? "Download update"
              : "Update"}
        </button>
      </div>
    </div>
  );
}
