import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  cancelImageBatch,
  startImageBatch,
  type ImageConversionRequest,
} from "../lib/tauri";
import type { JobStatus, OverwritePolicy } from "../types/conversion";
import type {
  ImageOutputFormat,
  ImageQualityPreset,
  ImageQueueFileItem,
  ImageResizePreset,
} from "../types/image";

export interface ConversionEvent {
  jobId: string;
  sourcePath: string | null;
  status: JobStatus;
  percent: number | null;
  message: string | null;
  outputPath: string | null;
}

export interface BatchEvent {
  batchId: string;
  total: number;
  completed: number;
  failed: number;
  cancelled: number;
  skipped: number;
  remaining: number;
  currentJobId: string | null;
  activeCount: number;
  parallelism: number;
  status: "running" | "completed" | "cancelled";
  message: string | null;
}

type PatchFn = (
  jobId: string | null,
  sourcePath: string | null,
  patch: Partial<ImageQueueFileItem>,
) => void;

export function useImageBatchConversion(patchByJobOrPath: PatchFn) {
  const [isBusy, setIsBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [batch, setBatch] = useState<BatchEvent | null>(null);
  const busyRef = useRef(false);

  useEffect(() => {
    let unlistenConversion: UnlistenFn | undefined;
    let unlistenBatch: UnlistenFn | undefined;
    let cancelled = false;

    listen<ConversionEvent>("conversion-event", (event) => {
      const payload = event.payload;
      patchByJobOrPath(payload.jobId, payload.sourcePath, {
        status: payload.status,
        percent: payload.percent,
        error: payload.message,
        outputPath: payload.outputPath,
      });
    }).then((fn) => {
      if (cancelled) fn();
      else unlistenConversion = fn;
    });

    listen<BatchEvent>("batch-event", (event) => {
      const payload = event.payload;
      setBatch(payload);
      if (payload.status === "completed" || payload.status === "cancelled") {
        busyRef.current = false;
        setIsBusy(false);
      }
    }).then((fn) => {
      if (cancelled) fn();
      else unlistenBatch = fn;
    });

    return () => {
      cancelled = true;
      unlistenConversion?.();
      unlistenBatch?.();
    };
  }, [patchByJobOrPath]);

  const convert = useCallback(
    async (args: {
      items: ImageQueueFileItem[];
      destinationDir: string;
      outputFormat: ImageOutputFormat;
      overwritePolicy: OverwritePolicy;
      qualityPreset: ImageQualityPreset;
      resizePreset: ImageResizePreset;
      preserveMetadata: boolean;
      assignJobIds: (paths: string[], jobIds: string[]) => void;
    }) => {
      if (busyRef.current) {
        return;
      }

      const convertible = args.items.filter(
        (item) =>
          item.info != null &&
          (item.status === "ready" || item.status === "failed"),
      );
      if (convertible.length === 0) {
        setError("No ready images to convert.");
        return;
      }

      setError(null);
      busyRef.current = true;
      setIsBusy(true);

      const requests: ImageConversionRequest[] = convertible.map((item) => ({
        sourcePath: item.path,
        destinationDir: args.destinationDir,
        outputFormat: args.outputFormat,
        relativeSubdir: item.relativeSubdir,
        overwritePolicy: args.overwritePolicy,
        qualityPreset: args.qualityPreset,
        resizePreset: args.resizePreset,
        preserveMetadata: args.preserveMetadata,
      }));

      try {
        const result = await startImageBatch(requests);
        args.assignJobIds(
          convertible.map((item) => item.path),
          result.jobIds,
        );
      } catch (caught) {
        busyRef.current = false;
        setIsBusy(false);
        setError(caught instanceof Error ? caught.message : String(caught));
      }
    },
    [],
  );

  const cancel = useCallback(async () => {
    try {
      await cancelImageBatch();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  }, []);

  const reset = useCallback(() => {
    setBatch(null);
    setError(null);
  }, []);

  return { isBusy, error, batch, convert, cancel, reset };
}
