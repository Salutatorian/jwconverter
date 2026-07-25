import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  cancelBatch,
  startBatch,
  type ConversionRequest,
} from "../lib/tauri";
import type {
  JobStatus,
  OutputFormat,
  OverwritePolicy,
  QualityPreset,
  QueueFileItem,
} from "../types/conversion";

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
  skipped: number;
  failed: number;
  cancelled: number;
  remaining: number;
  currentJobId: string | null;
  activeCount: number;
  parallelism: number;
  status: "running" | "completed" | "cancelled";
  message: string | null;
}

type PatchFn = (
  keys: { jobId?: string | null; sourcePath?: string | null },
  patch: Partial<QueueFileItem>,
) => void;

export function useBatchConversion(patchByJobOrPath: PatchFn) {
  const [batch, setBatch] = useState<BatchEvent | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isBusy, setIsBusy] = useState(false);
  const busyRef = useRef(false);

  useEffect(() => {
    let unlistenConversion: UnlistenFn | undefined;
    let unlistenBatch: UnlistenFn | undefined;
    let cancelled = false;

    listen<ConversionEvent>("conversion-event", (event) => {
      const payload = event.payload;
      patchByJobOrPath(
        { jobId: payload.jobId, sourcePath: payload.sourcePath },
        {
          jobId: payload.jobId,
          status: payload.status,
          percent: payload.percent,
          error: payload.status === "failed" ? payload.message : null,
          outputPath: payload.outputPath,
        },
      );
    })
      .then((fn) => {
        if (cancelled) fn();
        else unlistenConversion = fn;
      })
      .catch(() => {});

    listen<BatchEvent>("batch-event", (event) => {
      const payload = event.payload;
      setBatch(payload);
      const running = payload.status === "running";
      busyRef.current = running;
      setIsBusy(running);
      if (payload.status === "completed" || payload.status === "cancelled") {
        busyRef.current = false;
        setIsBusy(false);
      }
    })
      .then((fn) => {
        if (cancelled) fn();
        else unlistenBatch = fn;
      })
      .catch(() => {});

    return () => {
      cancelled = true;
      unlistenConversion?.();
      unlistenBatch?.();
    };
  }, [patchByJobOrPath]);

  const convert = useCallback(
    async (args: {
      items: QueueFileItem[];
      destinationDir: string;
      outputFormat: OutputFormat;
      overwritePolicy: OverwritePolicy;
      qualityPreset: QualityPreset;
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
        setError("No ready files to convert.");
        return;
      }

      setError(null);
      busyRef.current = true;
      setIsBusy(true);

      const requests: ConversionRequest[] = convertible.map((item) => ({
        sourcePath: item.path,
        destinationDir: args.destinationDir,
        outputFormat: args.outputFormat,
        sourceDurationSeconds: item.info?.durationSeconds ?? null,
        relativeSubdir: item.relativeSubdir,
        overwritePolicy: args.overwritePolicy,
        qualityPreset: args.qualityPreset,
      }));

      try {
        const result = await startBatch(requests);
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
      await cancelBatch();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  }, []);

  const reset = useCallback(() => {
    setBatch(null);
    setError(null);
    setIsBusy(false);
  }, []);

  return {
    batch,
    error,
    isBusy,
    convert,
    cancel,
    reset,
  };
}
