import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  AudioInfo,
  DiscoveredAudio,
  OutputFormat,
  OverwritePolicy,
  QualityPreset,
  BitDepthPreset,
} from "../types/conversion";

/** Typed wrappers around Tauri commands. Keep invoke strings in one place. */

export async function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("get_app_info");
}

export async function analyzeFile(path: string): Promise<AudioInfo> {
  return invoke<AudioInfo>("analyze_file", { path });
}

export async function getMediaToolsInfo(): Promise<{
  ffmpegPath: string | null;
  ffprobePath: string | null;
  source: string;
}> {
  return invoke("get_media_tools_info");
}

export async function getDefaultPaths(): Promise<{
  downloadsDir: string | null;
}> {
  return invoke("get_default_paths");
}

export async function discoverAudioPaths(
  paths: string[],
  recursive = true,
): Promise<DiscoveredAudio[]> {
  return invoke<DiscoveredAudio[]>("discover_audio_paths", { paths, recursive });
}

export interface ConversionRequest {
  sourcePath: string;
  destinationDir: string;
  outputFormat: OutputFormat;
  sourceDurationSeconds: number | null;
  relativeSubdir: string | null;
  overwritePolicy: OverwritePolicy;
  qualityPreset: QualityPreset;
  bitDepthPreset: BitDepthPreset;
  preserveTags: boolean;
  preserveCover: boolean;
}

export type PreflightWarningKind = "lossyToLossless" | "bitDepthUpsample";

export interface PreflightWarning {
  kind: PreflightWarningKind;
  count: number;
  message: string;
}

export interface PreflightReport {
  fileCount: number;
  skippedExisting: number;
  sourceBytes: number;
  estimatedOutputBytes: number;
  freeBytes: number | null;
  requiredBytes: number;
  diskBlocked: boolean;
  warnings: PreflightWarning[];
}

export interface PreflightBatchRequest {
  destinationDir: string;
  outputFormat: OutputFormat;
  qualityPreset: QualityPreset;
  bitDepthPreset: BitDepthPreset;
  overwritePolicy: OverwritePolicy;
  items: Array<{
    sourcePath: string;
    relativeSubdir: string | null;
    durationSeconds: number | null;
    sampleRate: number | null;
    channels: number | null;
    fileSizeBytes: number | null;
    codec: string | null;
    format: string | null;
    bitDepth: number | null;
    bitsPerRawSample: number | null;
    sampleFormat: string | null;
  }>;
}

export interface BatchStartResult {
  batchId: string;
  jobIds: string[];
}

export async function startConversion(request: ConversionRequest): Promise<string> {
  return invoke<string>("start_conversion", { request });
}

export async function startBatch(
  requests: ConversionRequest[],
): Promise<BatchStartResult> {
  return invoke<BatchStartResult>("start_batch", { requests });
}

export async function preflightBatch(
  request: PreflightBatchRequest,
): Promise<PreflightReport> {
  return invoke<PreflightReport>("preflight_batch", { request });
}

export async function cancelConversion(jobId: string): Promise<void> {
  return invoke("cancel_conversion", { jobId });
}

export async function cancelBatch(): Promise<void> {
  return invoke("cancel_batch");
}
