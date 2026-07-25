export type ImageOutputFormat = "jpeg" | "png" | "webp" | "tiff";
export type ImageQualityPreset = "low" | "medium" | "high";

export const IMAGE_OUTPUT_FORMATS: ReadonlyArray<{
  value: ImageOutputFormat;
  label: string;
}> = [
  { value: "jpeg", label: "JPEG" },
  { value: "png", label: "PNG" },
  { value: "webp", label: "WebP" },
  { value: "tiff", label: "TIFF" },
];

export const IMAGE_QUALITY_PRESETS: ReadonlyArray<{
  value: ImageQualityPreset;
  label: string;
}> = [
  { value: "low", label: "Low · 70" },
  { value: "medium", label: "Medium · 85" },
  { value: "high", label: "High · 95" },
];

export const IMAGE_EXTENSIONS = [
  "jpg",
  "jpeg",
  "png",
  "webp",
  "tif",
  "tiff",
  "bmp",
  "gif",
  "heic",
  "heif",
  "avif",
  "cr2",
  "cr3",
  "nef",
  "arw",
  "dng",
  "orf",
  "rw2",
  "raf",
  "pef",
  "srw",
] as const;

export function isImagePath(path: string): boolean {
  const lower = path.toLowerCase();
  return IMAGE_EXTENSIONS.some((ext) => lower.endsWith(`.${ext}`));
}

export function isLossyImageFormat(format: ImageOutputFormat): boolean {
  return format === "jpeg" || format === "webp";
}

export interface ImageInfo {
  path: string;
  filename: string;
  format: string | null;
  width: number | null;
  height: number | null;
  fileSizeBytes: number | null;
}

export interface DiscoveredImage {
  path: string;
  filename: string;
  relativeSubdir: string | null;
}

export interface ImageQueueFileItem {
  localId: string;
  path: string;
  filename: string;
  relativeSubdir: string | null;
  info: ImageInfo | null;
  status: import("./conversion").JobStatus;
  percent: number | null;
  error: string | null;
  outputPath: string | null;
  jobId: string | null;
}
