import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  AudioInfo,
  DiscoveredAudio,
  OutputFormat,
  OverwritePolicy,
  QualityPreset,
  BitDepthPreset,
  Mp3EncodingMode,
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
  magickPath: string | null;
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
  mp3EncodingMode: Mp3EncodingMode;
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
  mp3EncodingMode: Mp3EncodingMode;
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

export interface ImageConversionRequest {
  sourcePath: string;
  destinationDir: string;
  outputFormat: import("../types/image").ImageOutputFormat;
  relativeSubdir: string | null;
  overwritePolicy: OverwritePolicy;
  qualityPreset: import("../types/image").ImageQualityPreset;
  resizePreset: import("../types/image").ImageResizePreset;
  preserveMetadata: boolean;
}

export interface ImagePreflightBatchRequest {
  destinationDir: string;
  outputFormat: import("../types/image").ImageOutputFormat;
  qualityPreset: import("../types/image").ImageQualityPreset;
  resizePreset: import("../types/image").ImageResizePreset;
  overwritePolicy: OverwritePolicy;
  items: Array<{
    sourcePath: string;
    relativeSubdir: string | null;
    width: number | null;
    height: number | null;
    fileSizeBytes: number | null;
    format: string | null;
  }>;
}

export async function discoverImagePaths(
  paths: string[],
  recursive = true,
): Promise<import("../types/image").DiscoveredImage[]> {
  return invoke("discover_image_paths", { paths, recursive });
}

export async function analyzeImage(
  path: string,
): Promise<import("../types/image").ImageInfo> {
  return invoke("analyze_image", { path });
}

export async function preflightImageBatch(
  request: ImagePreflightBatchRequest,
): Promise<PreflightReport> {
  return invoke<PreflightReport>("preflight_image_batch", { request });
}

export async function startImageBatch(
  requests: ImageConversionRequest[],
): Promise<BatchStartResult> {
  return invoke<BatchStartResult>("start_image_batch", { requests });
}

export async function cancelImageBatch(): Promise<void> {
  return invoke("cancel_image_batch");
}

export async function analyzeLink(
  url: string,
  cookiesPath?: string | null,
): Promise<import("../types/links").LinkMediaInfo> {
  return invoke("analyze_link", { url, cookiesPath });
}

export async function startLinkDownload(
  request: import("../types/links").LinkDownloadRequest,
): Promise<string> {
  return invoke<string>("start_link_download", { request });
}

export async function enqueueLinkDownloads(
  request: import("../types/links").LinkBatchRequest,
): Promise<BatchStartResult> {
  return invoke<BatchStartResult>("enqueue_link_downloads", { request });
}

export async function cancelLinkBatch(): Promise<void> {
  return invoke("cancel_link_batch");
}

export async function cancelLinkDownload(jobId: string): Promise<void> {
  return invoke("cancel_link_download", { jobId });
}

export async function isLinkBatchRunning(): Promise<boolean> {
  return invoke<boolean>("is_link_batch_running");
}

export async function getYtdlpVersion(): Promise<string> {
  return invoke<string>("get_ytdlp_version");
}

export async function updateYtdlp(): Promise<string> {
  return invoke<string>("update_ytdlp");
}

export async function listLinkHistory(): Promise<
  import("../types/links").LinkHistoryItem[]
> {
  return invoke("list_link_history");
}

export async function clearLinkHistory(): Promise<void> {
  return invoke("clear_link_history");
}
