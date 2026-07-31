import type {
  BitDepthPreset,
  JobStatus,
  Mp3EncodingMode,
  OverwritePolicy,
  QualityPreset,
} from "./conversion";

/** Links mode DTOs. */

export interface VideoOption {
  id: string;
  label: string;
  height: number;
  width: number | null;
  fps: number | null;
  container: string | null;
}

export interface LinkPlaylistEntry {
  id: string | null;
  url: string;
  title: string | null;
  durationSeconds: number | null;
  isLive: boolean;
}

export interface LinkMediaInfo {
  originalUrl: string;
  webpageUrl: string | null;
  extractor: string | null;
  service: string | null;
  id: string | null;
  title: string | null;
  creator: string | null;
  durationSeconds: number | null;
  isLive: boolean;
  isPlaylist: boolean;
  itemCount: number | null;
  entries: LinkPlaylistEntry[];
  warnings: string[];
  videoOptions: VideoOption[];
  bestAudioCodec: string | null;
  bestAudioExt: string | null;
  sourceAudioLikelyLossy: boolean;
}

export type LinkMediaMode = "video" | "audio";
export type LinkVideoQuality = "best" | { height: number };
export type LinkAudioFormat =
  | "original"
  | "mp3"
  | "m4a"
  | "opus"
  | "flac"
  | "wav";

export interface LinkDownloadRequest {
  jobId?: string;
  url: string;
  destinationDir: string;
  overwritePolicy: OverwritePolicy;
  mode: LinkMediaMode;
  videoQuality: LinkVideoQuality;
  audioFormat: LinkAudioFormat;
  qualityPreset?: QualityPreset;
  mp3EncodingMode?: Mp3EncodingMode;
  bitDepthPreset?: BitDepthPreset;
  liveMaxMinutes?: number | null;
  cookiesPath?: string | null;
  downloadSubtitles: boolean;
  saveThumbnail: boolean;
  embedThumbnail: boolean;
}

export interface LinkBatchRequest {
  destinationDir: string;
  overwritePolicy: OverwritePolicy;
  mode: LinkMediaMode;
  videoQuality: LinkVideoQuality;
  audioFormat: LinkAudioFormat;
  qualityPreset?: QualityPreset;
  mp3EncodingMode?: Mp3EncodingMode;
  bitDepthPreset?: BitDepthPreset;
  liveMaxMinutes?: number | null;
  cookiesPath?: string | null;
  downloadSubtitles: boolean;
  saveThumbnail: boolean;
  embedThumbnail: boolean;
  /** Optional zip / batch label (playlist title, etc.). */
  batchTitle?: string | null;
  items: Array<{
    url: string;
    title?: string | null;
    durationSeconds?: number | null;
    isLive?: boolean | null;
    jobId?: string;
  }>;
}

export interface LinkDownloadEvent {
  jobId: string;
  status: JobStatus;
  percent: number | null;
  message: string;
  outputPath: string | null;
  error: string | null;
}

export interface LinkBatchEvent {
  batchId: string;
  completed: number;
  total: number;
  failed: number;
  cancelled: number;
  skipped: number;
  remaining: number;
  currentJobId: string | null;
  activeCount: number;
  parallelism: number;
  status: "running" | "completed" | "cancelled";
  message: string | null;
  zipPath?: string | null;
}

export interface LinkHistoryItem {
  jobId: string;
  service: string | null;
  title: string | null;
  status: string;
  outputPath: string | null;
  errorCategory: string | null;
  url: string | null;
}
