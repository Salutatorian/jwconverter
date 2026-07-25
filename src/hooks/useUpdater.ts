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
  checkForUpdates: () => Promise<void>;
  installUpdate: () => Promise<void>;
};

export function useUpdater(): UseUpdaterResult {
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [availableVersion, setAvailableVersion] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [downloadPercent, setDownloadPercent] = useState<number | null>(null);
  const updateRef = useRef<Update | null>(null);
  const checkingRef = useRef(false);

  const checkForUpdates = useCallback(async () => {
    if (checkingRef.current) {
      return;
    }
    checkingRef.current = true;
    setStatus("checking");
    setError(null);

    try {
      const update = await check();
      if (update) {
        updateRef.current = update;
        setAvailableVersion(update.version);
        setStatus("available");
      } else {
        updateRef.current = null;
        setAvailableVersion(null);
        setStatus("upToDate");
      }
    } catch (err: unknown) {
      updateRef.current = null;
      setAvailableVersion(null);
      setStatus("error");
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      checkingRef.current = false;
    }
  }, []);

  const installUpdate = useCallback(async () => {
    const update = updateRef.current;
    if (!update) {
      return;
    }

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
      setStatus("error");
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  useEffect(() => {
    void checkForUpdates();
    const timer = window.setInterval(() => {
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
    checkForUpdates,
    installUpdate,
  };
}
