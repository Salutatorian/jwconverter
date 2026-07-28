import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useCallback, useEffect, useRef, useState } from "react";

const RESCAN_MS = 4 * 60 * 60 * 1000;

export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "upToDate"
  | "downloading"
  | "error";

export type UseUpdaterResult = {
  status: UpdateStatus;
  availableVersion: string | null;
  error: string | null;
  downloadPercent: number | null;
  /** True while the launch-time auto-update overlay should block the app. */
  blockingOverlay: boolean;
  checkForUpdates: () => Promise<void>;
  installUpdate: () => Promise<void>;
  dismissBlockingOverlay: () => void;
};

export function useUpdater(): UseUpdaterResult {
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [availableVersion, setAvailableVersion] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [downloadPercent, setDownloadPercent] = useState<number | null>(null);
  const [blockingOverlay, setBlockingOverlay] = useState(false);
  const updateRef = useRef<Update | null>(null);
  const checkingRef = useRef(false);
  const downloadingRef = useRef(false);
  const statusRef = useRef<UpdateStatus>("idle");
  const autoInstallAttemptedRef = useRef(false);

  useEffect(() => {
    statusRef.current = status;
  }, [status]);

  const installUpdate = useCallback(async () => {
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

      try {
        const update = await check();
        if (downloadingRef.current) {
          return;
        }
        if (update) {
          updateRef.current = update;
          setAvailableVersion(update.version);
          setStatus("available");

          if (options?.fromLaunch && !autoInstallAttemptedRef.current) {
            autoInstallAttemptedRef.current = true;
            setBlockingOverlay(true);
            // Defer so state settles before download starts.
            queueMicrotask(() => {
              void installUpdate();
            });
          }
        } else {
          updateRef.current = null;
          setAvailableVersion(null);
          setStatus("upToDate");
        }
      } catch (err: unknown) {
        // Keep a previously discovered update installable across transient errors.
        if (updateRef.current) {
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
    [installUpdate],
  );

  const dismissBlockingOverlay = useCallback(() => {
    setBlockingOverlay(false);
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
    checkForUpdates: () => checkForUpdates(),
    installUpdate,
    dismissBlockingOverlay,
  };
}
