import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getVersion } from "@tauri-apps/api/app";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  compareVersions,
  detectHostOs,
  fetchUpdateManifest,
  platformAssetUrl,
  RELEASES_PAGE_URL,
} from "../lib/updateManifest";

const RESCAN_MS = 4 * 60 * 60 * 1000;

export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "upToDate"
  | "downloading"
  | "error";

export type UpdateInstallMode = "auto" | "manual";

export type UseUpdaterResult = {
  status: UpdateStatus;
  availableVersion: string | null;
  error: string | null;
  downloadPercent: number | null;
  /** True while the launch-time auto-update overlay should block the app. */
  blockingOverlay: boolean;
  /** Mac/Linux: open the DMG/AppImage download instead of in-app install. */
  installMode: UpdateInstallMode;
  downloadUrl: string | null;
  /** Non-blocking reminder for manual platforms (dismissible). */
  manualReminder: boolean;
  checkForUpdates: () => Promise<void>;
  installUpdate: () => Promise<void>;
  dismissBlockingOverlay: () => void;
  dismissManualReminder: () => void;
};

export function useUpdater(): UseUpdaterResult {
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [availableVersion, setAvailableVersion] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [downloadPercent, setDownloadPercent] = useState<number | null>(null);
  const [blockingOverlay, setBlockingOverlay] = useState(false);
  const [installMode, setInstallMode] = useState<UpdateInstallMode>("auto");
  const [downloadUrl, setDownloadUrl] = useState<string | null>(null);
  const [manualReminder, setManualReminder] = useState(false);
  const updateRef = useRef<Update | null>(null);
  const checkingRef = useRef(false);
  const downloadingRef = useRef(false);
  const statusRef = useRef<UpdateStatus>("idle");
  const autoInstallAttemptedRef = useRef(false);
  const installModeRef = useRef<UpdateInstallMode>("auto");
  const downloadUrlRef = useRef<string | null>(null);

  useEffect(() => {
    statusRef.current = status;
  }, [status]);

  useEffect(() => {
    installModeRef.current = installMode;
  }, [installMode]);

  useEffect(() => {
    downloadUrlRef.current = downloadUrl;
  }, [downloadUrl]);

  const installUpdate = useCallback(async () => {
    if (installModeRef.current === "manual") {
      const url = downloadUrlRef.current ?? RELEASES_PAGE_URL;
      try {
        await openUrl(url);
      } catch (err: unknown) {
        setError(err instanceof Error ? err.message : String(err));
      }
      return;
    }

    const update = updateRef.current;
    if (!update || downloadingRef.current) {
      return;
    }

    downloadingRef.current = true;
    setStatus("downloading");
    setError(null);
    setDownloadPercent(0);

    try {
      let downloaded = 0;
      let contentLength = 0;

      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            contentLength = event.data.contentLength ?? 0;
            downloaded = 0;
            setDownloadPercent(contentLength > 0 ? 0 : null);
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            if (contentLength > 0) {
              setDownloadPercent(
                Math.min(100, Math.round((downloaded / contentLength) * 100)),
              );
            }
            break;
          case "Finished":
            setDownloadPercent(100);
            break;
        }
      });

      await relaunch();
    } catch (err: unknown) {
      downloadingRef.current = false;
      // Keep updateRef so the user can retry without another check.
      setStatus("available");
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const applyManualAvailability = useCallback(
    (version: string, fromLaunch: boolean) => {
      const os = detectHostOs();
      updateRef.current = null;
      setInstallMode("manual");
      setDownloadUrl(platformAssetUrl(version, os));
      setAvailableVersion(version);
      setStatus("available");
      if (fromLaunch) {
        setManualReminder(true);
      }
    },
    [],
  );

  const checkForUpdates = useCallback(
    async (options?: { fromLaunch?: boolean }) => {
      if (checkingRef.current || downloadingRef.current) {
        return;
      }
      if (statusRef.current === "downloading") {
        return;
      }

      checkingRef.current = true;
      setStatus("checking");
      setError(null);
      setDownloadPercent(null);

      const fromLaunch = Boolean(options?.fromLaunch);

      try {
        // Signed in-app package when present in latest.json for this OS (Windows today).
        let update: Update | null = null;
        try {
          update = await check();
        } catch {
          // Fall through to manifest reminder (common on Mac/Linux today).
          update = null;
        }

        if (downloadingRef.current) {
          return;
        }

        if (update) {
          updateRef.current = update;
          setInstallMode("auto");
          setDownloadUrl(null);
          setManualReminder(false);
          setAvailableVersion(update.version);
          setStatus("available");

          if (fromLaunch && !autoInstallAttemptedRef.current) {
            autoInstallAttemptedRef.current = true;
            setBlockingOverlay(true);
            queueMicrotask(() => {
              void installUpdate();
            });
          }
          return;
        }

        // No signed package for this OS — still remind when latest.json is newer.
        const [currentVersion, manifest] = await Promise.all([
          getVersion(),
          fetchUpdateManifest(),
        ]);
        if (downloadingRef.current) {
          return;
        }

        if (compareVersions(manifest.version, currentVersion) > 0) {
          applyManualAvailability(manifest.version, fromLaunch);
          return;
        }

        updateRef.current = null;
        setInstallMode("auto");
        setDownloadUrl(null);
        setAvailableVersion(null);
        setManualReminder(false);
        setStatus("upToDate");
      } catch (err: unknown) {
        // Keep a previously discovered update installable across transient errors.
        if (updateRef.current || installModeRef.current === "manual") {
          setStatus("available");
          setError(err instanceof Error ? err.message : String(err));
        } else {
          setAvailableVersion(null);
          setStatus("error");
          setError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        checkingRef.current = false;
      }
    },
    [applyManualAvailability, installUpdate],
  );

  const dismissBlockingOverlay = useCallback(() => {
    setBlockingOverlay(false);
  }, []);

  const dismissManualReminder = useCallback(() => {
    setManualReminder(false);
  }, []);

  useEffect(() => {
    void checkForUpdates({ fromLaunch: true });
    const timer = window.setInterval(() => {
      // Mid-session rescans never auto-block; Settings / next launch handle it.
      void checkForUpdates();
    }, RESCAN_MS);
    return () => {
      window.clearInterval(timer);
    };
  }, [checkForUpdates]);

  return {
    status,
    availableVersion,
    error,
    downloadPercent,
    blockingOverlay,
    installMode,
    downloadUrl,
    manualReminder,
    checkForUpdates: () => checkForUpdates(),
    installUpdate,
    dismissBlockingOverlay,
    dismissManualReminder,
  };
}
