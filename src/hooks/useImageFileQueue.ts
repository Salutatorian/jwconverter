import { useCallback, useState } from "react";
import { analyzeImage, discoverImagePaths } from "../lib/tauri";
import { filenameFromPath } from "../types/conversion";
import type {
  DiscoveredImage,
  ImageQueueFileItem,
} from "../types/image";

function errorMessage(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return "Something went wrong while analyzing the image.";
}

function newLocalId(): string {
  return crypto.randomUUID();
}

export function useImageFileQueue() {
  const [items, setItems] = useState<ImageQueueFileItem[]>([]);
  const [isAnalyzing, setIsAnalyzing] = useState(false);

  const analyzeDiscovered = useCallback(async (discovered: DiscoveredImage) => {
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
      const info = await analyzeImage(discovered.path);
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
        const discovered = await discoverImagePaths(paths, recursive);
        for (const item of discovered) {
          await analyzeDiscovered(item);
        }
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

  const assignJobIds = useCallback((paths: string[], jobIds: string[]) => {
    setItems((current) =>
      current.map((item) => {
        const index = paths.indexOf(item.path);
        if (index < 0) {
          return item;
        }
        return {
          ...item,
          jobId: jobIds[index] ?? item.jobId,
          status: "queued",
          percent: 0,
          error: null,
        };
      }),
    );
  }, []);

  const patchByJobOrPath = useCallback(
    (
      jobId: string | null,
      sourcePath: string | null,
      patch: Partial<ImageQueueFileItem>,
    ) => {
      setItems((current) =>
        current.map((item) => {
          const match =
            (jobId != null && item.jobId === jobId) ||
            (sourcePath != null && item.path === sourcePath);
          return match ? { ...item, ...patch } : item;
        }),
      );
    },
    [],
  );

  return {
    items,
    isAnalyzing,
    addPaths,
    removeItem,
    clear,
    assignJobIds,
    patchByJobOrPath,
  };
}
