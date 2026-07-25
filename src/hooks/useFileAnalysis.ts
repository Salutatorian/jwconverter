import { useCallback, useState } from "react";
import { analyzeFile } from "../lib/tauri";
import type { AudioInfo, JobStatus } from "../types/conversion";

function errorMessage(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return "Something went wrong while analyzing the file.";
}

export function useFileAnalysis() {
  const [info, setInfo] = useState<AudioInfo | null>(null);
  const [status, setStatus] = useState<JobStatus>("idle");
  const [error, setError] = useState<string | null>(null);

  const analyze = useCallback(async (path: string) => {
    setStatus("analyzing");
    setError(null);

    try {
      const result = await analyzeFile(path);
      setInfo(result);
      setStatus("ready");
    } catch (caught) {
      setInfo(null);
      setStatus("failed");
      setError(errorMessage(caught));
    }
  }, []);

  const clear = useCallback(() => {
    setInfo(null);
    setStatus("idle");
    setError(null);
  }, []);

  return {
    info,
    status,
    error,
    isAnalyzing: status === "analyzing",
    analyze,
    clear,
  };
}
