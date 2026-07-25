import { useCallback, useState } from "react";
import { analyzeFile, discoverAudioPaths } from "../lib/tauri";
import {
  filenameFromPath,
  type DiscoveredAudio,
  type QueueFileItem,
} from "../types/conversion";

function errorMessage(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return "Something went wrong while analyzing the file.";
}

function newLocalId(): string {
  return crypto.randomUUID();
}

export function useFileQueue() {
  const [items, setItems] = useState<QueueFileItem[]>([]);
  const [isAnalyzing, setIsAnalyzing] = useState(false);

  const analyzeDiscovered = useCallback(async (discovered: DiscoveredAudio) => {
    setItems((current) => {
      if (current.some((item) => item.path === discovered.path)) {
        return current;
      }
      return [
        ...current,
        {
          localId: newLocalId(),
          path: discovered.path,
          filename: discovered.filename || filenameFromPath(discovered.path),
          relativeSubdir: discovered.relativeSubdir,
          info: null,
          status: "analyzing",
          percent: null,
          error: null,
          outputPath: null,
          jobId: null,
        },
      ];
    });

    try {
      const info = await analyzeFile(discovered.path);
      setItems((current) =>
        current.map((item) =>
          item.path === discovered.path
            ? {
                ...item,
                info,
                filename: info.filename,
                status: "ready",
                error: null,
              }
            : item,
        ),
      );
    } catch (caught) {
      setItems((current) =>
        current.map((item) =>
          item.path === discovered.path
            ? {
                ...item,
                status: "failed",
                error: errorMessage(caught),
              }
            : item,
        ),
      );
    }
  }, []);

  const addPaths = useCallback(
    async (paths: string[], recursive = true) => {
      if (paths.length === 0) {
        return;
      }
      setIsAnalyzing(true);
      try {
        const discovered = await discoverAudioPaths(paths, recursive);
        for (const item of discovered) {
          await analyzeDiscovered(item);
        }
      } catch (caught) {
        console.error(caught);
      } finally {
        setIsAnalyzing(false);
      }
    },
    [analyzeDiscovered],
  );

  const removeItem = useCallback((localId: string) => {
    setItems((current) => current.filter((item) => item.localId !== localId));
  }, []);

  const clear = useCallback(() => {
    setItems([]);
  }, []);

  const patchByJobOrPath = useCallback(
    (
      keys: { jobId?: string | null; sourcePath?: string | null },
      patch: Partial<QueueFileItem>,
    ) => {
      setItems((current) =>
        current.map((item) => {
          const matchJob = keys.jobId && item.jobId === keys.jobId;
          const matchPath = keys.sourcePath && item.path === keys.sourcePath;
          if (!matchJob && !matchPath) {
            return item;
          }
          return {
            ...item,
            ...patch,
            jobId: patch.jobId ?? keys.jobId ?? item.jobId,
          };
        }),
      );
    },
    [],
  );

  const assignJobIds = useCallback((paths: string[], jobIds: string[]) => {
    setItems((current) => {
      const next = [...current];
      for (let i = 0; i < paths.length; i += 1) {
        const path = paths[i];
        const jobId = jobIds[i];
        const index = next.findIndex((item) => item.path === path);
        if (index >= 0) {
          next[index] = {
            ...next[index],
            jobId,
            status: "queued",
            percent: 0,
            error: null,
            outputPath: null,
          };
        }
      }
      return next;
    });
  }, []);

  return {
    items,
    setItems,
    isAnalyzing,
    addPaths,
    removeItem,
    clear,
    patchByJobOrPath,
    assignJobIds,
    readyCount: items.filter((item) => item.info && item.status !== "failed").length,
  };
}
