import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import {
  cancelConversion,
  startConversion,
  type ConversionRequest,
} from "../lib/tauri";
import type { JobStatus, OutputFormat } from "../types/conversion";

export interface ConversionEvent {
  jobId: string;
  status: JobStatus;
  percent: number | null;
  message: string | null;
  outputPath: string | null;
}

export function useConversion() {
  const [jobId, setJobId] = useState<string | null>(null);
  const [status, setStatus] = useState<JobStatus>("idle");
  const [percent, setPercent] = useState<number | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [outputPath, setOutputPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const isBusy =
    status === "queued" || status === "converting" || status === "verifying";

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;

    listen<ConversionEvent>("conversion-event", (event) => {
      const payload = event.payload;
      setJobId(payload.jobId);
      setStatus(payload.status);
      if (payload.percent != null) {
        setPercent(payload.percent);
      }
      if (payload.message) {
        setMessage(payload.message);
      }
      if (payload.outputPath) {
        setOutputPath(payload.outputPath);
      }
      if (payload.status === "failed") {
        setError(payload.message ?? "Conversion failed.");
      }
      if (payload.status === "cancelled") {
        setError(null);
      }
      if (payload.status === "completed") {
        setError(null);
      }
    })
      .then((fn) => {
        if (cancelled) {
          fn();
        } else {
          unlisten = fn;
        }
      })
      .catch(() => {
        // Events unavailable outside Tauri.
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const convert = useCallback(
    async (args: {
      sourcePath: string;
      destinationDir: string;
      outputFormat: OutputFormat;
      sourceDurationSeconds: number | null;
    }) => {
      setError(null);
      setMessage(null);
      setOutputPath(null);
      setPercent(0);
      setStatus("queued");

      const request: ConversionRequest = {
        sourcePath: args.sourcePath,
        destinationDir: args.destinationDir,
        outputFormat: args.outputFormat,
        sourceDurationSeconds: args.sourceDurationSeconds,
        relativeSubdir: null,
        overwritePolicy: "rename",
        qualityPreset: "medium",
      };

      try {
        const id = await startConversion(request);
        setJobId(id);
      } catch (caught) {
        setStatus("failed");
        setError(
          caught instanceof Error ? caught.message : String(caught),
        );
      }
    },
    [],
  );

  const cancel = useCallback(async () => {
    if (!jobId) {
      return;
    }
    try {
      await cancelConversion(jobId);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  }, [jobId]);

  const reset = useCallback(() => {
    setJobId(null);
    setStatus("idle");
    setPercent(null);
    setMessage(null);
    setOutputPath(null);
    setError(null);
  }, []);

  return {
    jobId,
    status,
    percent,
    message,
    outputPath,
    error,
    isBusy,
    convert,
    cancel,
    reset,
  };
}
