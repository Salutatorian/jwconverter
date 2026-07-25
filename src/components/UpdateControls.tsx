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
    checkForUpdates,
    installUpdate,
  } = updater;

  const isBusy = status === "checking" || status === "downloading";

  return (
    <div className="flex flex-col gap-2">
      {status === "available" || status === "downloading" ? (
        <p className="text-xs text-[var(--text-muted)]">
          {status === "downloading"
            ? `Downloading v${availableVersion ?? ""}${
                downloadPercent != null ? ` · ${downloadPercent}%` : ""
              }. The app will restart when done.`
            : `Update available: v${availableVersion}. Click Update to install — the app will restart.`}
        </p>
      ) : null}

      {status === "error" && error ? (
        <p className="text-xs text-red-300" role="alert">
          Update check failed: {error}
        </p>
      ) : null}

      {status === "upToDate" ? (
        <p className="text-xs text-[var(--text-faint)]">You're up to date</p>
      ) : null}

      {status === "idle" || status === "checking" ? (
        <p className="text-xs text-[var(--text-faint)]">
          {status === "checking"
            ? "Checking GitHub Releases…"
            : "Checks on launch and every few hours."}
        </p>
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
          {status === "checking" ? "Checking…" : "Check for updates"}
        </button>
        <button
          type="button"
          className="btn btn-primary"
          disabled={status !== "available" || isBusy}
          onClick={() => {
            void installUpdate();
          }}
        >
          {status === "downloading" ? "Updating…" : "Update"}
        </button>
      </div>
    </div>
  );
}
