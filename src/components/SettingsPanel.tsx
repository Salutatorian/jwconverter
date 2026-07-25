import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useId, useRef } from "react";
import type { UseUpdaterResult } from "../hooks/useUpdater";
import {
  FFMPEG_LICENSING_URL,
  FFMPEG_SOURCE_URL,
  GITHUB_ISSUES_URL,
  GITHUB_RELEASES_URL,
  GITHUB_REPO_URL,
  GYAN_FFMPEG_BUILDS_URL,
} from "../lib/links";
import type { AppInfo } from "../types/conversion";
import { UpdateControls } from "./UpdateControls";

type SettingsPanelProps = {
  open: boolean;
  onClose: () => void;
  appInfo: AppInfo | null;
  updater: UseUpdaterResult;
};

async function openExternal(url: string) {
  try {
    await openUrl(url);
  } catch {
    window.open(url, "_blank", "noopener,noreferrer");
  }
}

export function SettingsPanel({
  open,
  onClose,
  appInfo,
  updater,
}: SettingsPanelProps) {
  const titleId = useId();
  const closeRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) {
      return;
    }
    closeRef.current?.focus();

    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onClose();
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [open, onClose]);

  if (!open) {
    return null;
  }

  return (
    <div className="modal-overlay" role="presentation">
      <button
        type="button"
        className="modal-backdrop"
        aria-label="Close settings"
        onClick={onClose}
      />
      <section
        className="modal-card"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <div className="flex items-start justify-between gap-3">
          <div>
            <h2 id={titleId} className="text-lg font-semibold text-[var(--text)]">
              Settings
            </h2>
            <p className="mt-0.5 text-sm text-[var(--text-muted)]">
              Updates, links, and about
            </p>
          </div>
          <button
            ref={closeRef}
            type="button"
            className="btn btn-ghost"
            onClick={onClose}
          >
            Close
          </button>
        </div>

        <div className="modal-section">
          <h3 className="panel-title">Updates</h3>
          <div className="mt-3">
            <UpdateControls updater={updater} />
          </div>
        </div>

        <div className="modal-section">
          <h3 className="panel-title">Links</h3>
          <div className="mt-3 flex flex-wrap gap-2">
            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => {
                void openExternal(GITHUB_REPO_URL);
              }}
            >
              GitHub
            </button>
            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => {
                void openExternal(GITHUB_RELEASES_URL);
              }}
            >
              Releases
            </button>
            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => {
                void openExternal(GITHUB_ISSUES_URL);
              }}
            >
              Issues
            </button>
          </div>
          <p className="panel-hint">
            Opens in your browser. Releases is where installers are published.
          </p>
        </div>

        <div className="modal-section">
          <h3 className="panel-title">About</h3>
          <p className="mt-3 text-sm text-[var(--text-muted)]">
            JW Converter
            {appInfo ? ` · v${appInfo.version}` : null}
          </p>
          <p className="panel-hint">
            Local-first audio conversion. No accounts, no cloud upload, no
            telemetry. Sources are never modified.
          </p>
          <p className="mt-3 text-sm text-[var(--text-muted)]">
            Uses FFmpeg / FFprobe (Gyan full build, GPL). See the bundled
            THIRD_PARTY_FFMPEG.txt in the install folder for build identity and
            source offer.
          </p>
          <div className="mt-3 flex flex-wrap gap-2">
            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => {
                void openExternal(FFMPEG_LICENSING_URL);
              }}
            >
              FFmpeg licensing
            </button>
            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => {
                void openExternal(FFMPEG_SOURCE_URL);
              }}
            >
              FFmpeg source
            </button>
            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => {
                void openExternal(GYAN_FFMPEG_BUILDS_URL);
              }}
            >
              Gyan builds
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}
