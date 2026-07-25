/** Display helpers for durations and file sizes. */

export function formatDuration(seconds: number | null | undefined): string {
  if (seconds == null || Number.isNaN(seconds)) {
    return "—";
  }
  const total = Math.max(0, Math.floor(seconds));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  if (h > 0) {
    return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  }
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function formatFileSize(bytes: number | null | undefined): string {
  if (bytes == null || Number.isNaN(bytes)) {
    return "—";
  }
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  const units = ["KB", "MB", "GB", "TB"] as const;
  let value = bytes / 1024;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[unitIndex]}`;
}

export function formatBitrate(bps: number | null | undefined): string {
  if (bps == null || Number.isNaN(bps) || bps <= 0) {
    return "—";
  }
  const kbps = bps / 1000;
  if (kbps >= 1000) {
    return `${(kbps / 1000).toFixed(1)} Mbps`;
  }
  return `${Math.round(kbps)} kbps`;
}

export function formatChannelLabel(
  channels: number | null | undefined,
  layout: string | null | undefined,
): string {
  if (layout && layout.trim().length > 0) {
    const normalized = layout.trim().toLowerCase();
    if (normalized === "mono") return "Mono";
    if (normalized === "stereo") return "Stereo";
    return layout;
  }
  if (channels == null) return "—";
  if (channels === 1) return "Mono";
  if (channels === 2) return "Stereo";
  return `${channels} ch`;
}

/** Compact source summary for queue rows. */
export function formatSourceSummary(info: {
  format?: string | null;
  codec?: string | null;
  bitDepth?: number | null;
  sampleRate?: number | null;
  channels?: number | null;
  channelLayout?: string | null;
  bitrate?: number | null;
  fileSizeBytes?: number | null;
  sampleFormat?: string | null;
}): string {
  const parts: string[] = [];
  const formatLabel = (info.format ?? info.codec ?? "").toUpperCase();
  if (formatLabel) {
    parts.push(formatLabel.split(",")[0] ?? formatLabel);
  }
  if (info.bitDepth != null) {
    const floatish = (info.sampleFormat ?? "").toLowerCase().includes("flt");
    parts.push(floatish ? `${info.bitDepth}-bit float` : `${info.bitDepth}-bit`);
  }
  if (info.sampleRate != null) {
    parts.push(
      info.sampleRate >= 1000
        ? `${(info.sampleRate / 1000).toFixed(info.sampleRate % 1000 === 0 ? 0 : 1)} kHz`
        : `${info.sampleRate} Hz`,
    );
  }
  const channels = formatChannelLabel(info.channels, info.channelLayout);
  if (channels !== "—") {
    parts.push(channels);
  }
  if (info.bitrate != null) {
    parts.push(formatBitrate(info.bitrate));
  }
  if (info.fileSizeBytes != null) {
    parts.push(formatFileSize(info.fileSizeBytes));
  }
  return parts.join(" · ");
}
