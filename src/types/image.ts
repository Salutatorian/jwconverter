export type ImageOutputFormat =
  | "jpeg"
  | "png"
  | "webp"
  | "tiff"
  | "bmp"
  | "gif"
  | "avif";
export type ImageQualityPreset = "low" | "medium" | "high" | "lossless";
export type ImageResizePreset =
  | "original"
  | "2048"
  | "1920"
  | "1280"
  | "1024";

export const IMAGE_OUTPUT_FORMATS: ReadonlyArray<{
  value: ImageOutputFormat;
  label: string;
}> = [
  { value: "jpeg", label: "JPEG" },
  { value: "png", label: "PNG" },
  { value: "webp", label: "WebP" },
  { value: "avif", label: "AVIF" },
  { value: "tiff", label: "TIFF" },
  { value: "bmp", label: "BMP" },
  { value: "gif", label: "GIF" },
];

export function qualityPresetsForFormat(
  format: ImageOutputFormat,
): ReadonlyArray<{ value: ImageQualityPreset; label: string }> {
  switch (format) {
    case "jpeg":
    case "avif":
      return [
        { value: "low", label: "Low · 70" },
        { value: "medium", label: "Medium · 85" },
        { value: "high", label: "High · 95" },
      ];
    case "webp":
      return [
        { value: "low", label: "Low · 70" },
        { value: "medium", label: "Medium · 85" },
        { value: "high", label: "High · 95" },
        { value: "lossless", label: "Lossless" },
      ];
    case "png":
      return [
        { value: "low", label: "Fast · 90" },
        { value: "medium", label: "Balanced · 75" },
        { value: "high", label: "Small · 50" },
      ];
    case "tiff":
    case "bmp":
    case "gif":
      return [];
  }
}

export function showsImageQualityControls(format: ImageOutputFormat): boolean {
  return (
    format === "jpeg" ||
    format === "png" ||
    format === "webp" ||
    format === "avif"
  );
}

/** Kept for callers that only need the shared Low/Med/High set. */
export const IMAGE_QUALITY_PRESETS = qualityPresetsForFormat("jpeg");

export const IMAGE_RESIZE_PRESETS: ReadonlyArray<{
  value: ImageResizePreset;
  label: string;
}> = [
  { value: "original", label: "Original" },
  { value: "2048", label: "Max 2048" },
  { value: "1920", label: "Max 1920" },
  { value: "1280", label: "Max 1280" },
  { value: "1024", label: "Max 1024" },
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

export function isLossyImageFormat(
  format: ImageOutputFormat,
  quality: ImageQualityPreset = "medium",
): boolean {
  if (format === "jpeg" || format === "gif" || format === "avif") {
    return true;
  }
  if (format === "webp") {
    return quality !== "lossless";
  }
  return false;
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
