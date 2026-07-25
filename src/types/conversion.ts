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

/** Formats for early milestones. More come later. */
export type OutputFormat = "wav" | "flac" | "mp3";

export type OverwritePolicy = "rename" | "skip" | "replace";

export interface AudioInfo {
  path: string;
  filename: string;
  format: string | null;
  codec: string | null;
  durationSeconds: number | null;
  sampleRate: number | null;
  channels: number | null;
  fileSizeBytes: number | null;
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
];

export const OVERWRITE_POLICIES: ReadonlyArray<{
  value: OverwritePolicy;
  label: string;
}> = [
  { value: "rename", label: "Rename" },
  { value: "skip", label: "Skip" },
  { value: "replace", label: "Replace" },
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
