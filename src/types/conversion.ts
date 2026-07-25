/** Shared conversion DTOs — keep aligned with Rust engine types. */

export type JobStatus =
  | "idle"
  | "analyzing"
  | "ready"
  | "queued"
  | "converting"
  | "verifying"
  | "completed"
  | "failed"
  | "cancelled"
  | "skipped";

/** Supported output formats. */
export type OutputFormat =
  | "wav"
  | "flac"
  | "mp3"
  | "aac"
  | "opus"
  | "ogg"
  | "alac"
  | "aiff";

export type OverwritePolicy = "rename" | "skip" | "replace";

export type QualityPreset = "low" | "medium" | "high";

/** PCM bit depth for WAV / AIFF only. */
export type BitDepthPreset = "original" | "16" | "24" | "float32";

export const LOSSY_FORMATS: ReadonlySet<OutputFormat> = new Set([
  "mp3",
  "aac",
  "opus",
  "ogg",
]);

export const PCM_FORMATS: ReadonlySet<OutputFormat> = new Set(["wav", "aiff"]);

export function isLossyFormat(format: OutputFormat): boolean {
  return LOSSY_FORMATS.has(format);
}

export function isPcmFormat(format: OutputFormat): boolean {
  return PCM_FORMATS.has(format);
}

export function supportsEmbeddedCover(format: OutputFormat): boolean {
  return (
    format === "mp3" ||
    format === "flac" ||
    format === "aac" ||
    format === "alac" ||
    format === "ogg" ||
    format === "opus"
  );
}

export interface AudioInfo {
  path: string;
  filename: string;
  format: string | null;
  codec: string | null;
  durationSeconds: number | null;
  sampleRate: number | null;
  channels: number | null;
  fileSizeBytes: number | null;
  bitDepth: number | null;
  sampleFormat: string | null;
  bitrate: number | null;
  channelLayout: string | null;
  bitsPerRawSample: number | null;
}

export interface AppInfo {
  name: string;
  version: string;
  phase: string;
}

export interface DiscoveredAudio {
  path: string;
  filename: string;
  relativeSubdir: string | null;
}

export interface QueueFileItem {
  localId: string;
  path: string;
  filename: string;
  relativeSubdir: string | null;
  info: AudioInfo | null;
  status: JobStatus;
  percent: number | null;
  error: string | null;
  outputPath: string | null;
  jobId: string | null;
}

export const OUTPUT_FORMATS: ReadonlyArray<{
  value: OutputFormat;
  label: string;
  enabled: boolean;
}> = [
  { value: "flac", label: "FLAC", enabled: true },
  { value: "wav", label: "WAV", enabled: true },
  { value: "mp3", label: "MP3", enabled: true },
  { value: "aac", label: "AAC / M4A", enabled: true },
  { value: "opus", label: "Opus", enabled: true },
  { value: "ogg", label: "OGG", enabled: true },
  { value: "alac", label: "ALAC", enabled: true },
  { value: "aiff", label: "AIFF", enabled: true },
];

export const OVERWRITE_POLICIES: ReadonlyArray<{
  value: OverwritePolicy;
  label: string;
}> = [
  { value: "rename", label: "Rename" },
  { value: "skip", label: "Skip" },
  { value: "replace", label: "Replace" },
];

export const QUALITY_PRESETS: ReadonlyArray<{
  value: QualityPreset;
  label: string;
}> = [
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
];

export const BIT_DEPTH_PRESETS: ReadonlyArray<{
  value: BitDepthPreset;
  label: string;
}> = [
  { value: "original", label: "Original" },
  { value: "16", label: "16-bit" },
  { value: "24", label: "24-bit" },
  { value: "float32", label: "32-bit float" },
];

export const AUDIO_EXTENSIONS = [
  "wav",
  "flac",
  "mp3",
  "m4a",
  "aac",
  "ogg",
  "opus",
  "aiff",
  "aif",
  "wma",
  "caf",
] as const;

export function isAudioPath(path: string): boolean {
  const lower = path.toLowerCase();
  return AUDIO_EXTENSIONS.some((ext) => lower.endsWith(`.${ext}`));
}

export function filenameFromPath(path: string): string {
  const normalized = path.replace(/\//g, "\\");
  const index = normalized.lastIndexOf("\\");
  return index >= 0 ? normalized.slice(index + 1) : path;
}
