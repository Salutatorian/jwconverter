import type {
  BitDepthPreset,
  JobStatus,
  Mp3EncodingMode,
  OverwritePolicy,
  QualityPreset,
} from "./conversion";

/** Experimental Links DTOs. */

export interface VideoOption {
  id: string;
  label: string;
  height: number;
  width: number | null;
  fps: number | null;
  container: string | null;
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
}

export interface LinkDownloadEvent {
  jobId: string;
  status: JobStatus;
  percent: number | null;
  message: string;
  outputPath: string | null;
  error: string | null;
}
