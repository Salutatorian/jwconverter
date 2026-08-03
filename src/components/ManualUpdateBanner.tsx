type ManualUpdateBannerProps = {
  version: string;
  onDownload: () => void;
  onDismiss: () => void;
  onOpenSettings: () => void;
};

export function ManualUpdateBanner({
  version,
  onDownload,
  onDismiss,
  onOpenSettings,
}: ManualUpdateBannerProps) {
  return (
    <div className="manual-update-banner" role="status">
      <p className="manual-update-banner-text">
        Update available: <strong>v{version}</strong> — download for your OS
        (Mac/Linux install manually for now).
      </p>
      <div className="manual-update-banner-actions">
        <button type="button" className="btn btn-primary" onClick={onDownload}>
          Download update
        </button>
        <button
          type="button"
          className="btn btn-secondary"
          onClick={onOpenSettings}
        >
          Settings
        </button>
        <button type="button" className="btn btn-secondary" onClick={onDismiss}>
          Later
        </button>
      </div>
    </div>
  );
}
