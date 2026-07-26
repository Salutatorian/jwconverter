export type WhatsNewEntry = {
  version: string;
  changes: string[];
  debugs: string[];
};

/** Shown once after updating to this version (first launch). */
export const WHATS_NEW: WhatsNewEntry[] = [
  {
    version: "0.2.6",
    changes: [
      "Product copy reflects Audio + Images (README, Settings, installer)",
      "Images mode: Update available CTA, mode-aware drop zone, queue Remove parity",
      "Consistent Audio | Images chip order across modes",
    ],
    debugs: [],
  },
  {
    version: "0.2.5",
    changes: [
      "Preserve image metadata (EXIF / ICC / comments) — on by default",
      "Optional strip when you want a clean output file",
    ],
    debugs: [
      "Soak: Unicode paths, long names, and corrupt decode handling look solid",
    ],
  },
  {
    version: "0.2.4",
    changes: [
      "New image outputs: BMP, GIF (still), and AVIF",
      "AVIF quality presets (Low / Medium / High)",
    ],
    debugs: [
      "HEIC export still unavailable — bundled Magick is HEIC read-only",
    ],
  },
  {
    version: "0.2.3",
    changes: [
      "WebP Lossless encoding option",
      "PNG compression presets (Fast / Balanced / Small)",
      "Quality controls shown for JPEG, WebP, and PNG with format-aware labels",
    ],
    debugs: [],
  },
  {
    version: "0.2.2",
    changes: [
      "Images honor EXIF orientation (phone photos no longer land sideways)",
      "Clearer errors when a camera RAW file can't be decoded",
    ],
    debugs: [],
  },
  {
    version: "0.2.1",
    changes: [
      "Image resize presets (original or max long-edge 2048 / 1920 / 1280 / 1024)",
      "Image preflight: size estimate, disk space gate, lossy→PNG/TIFF honesty warning",
    ],
    debugs: [],
  },
  {
    version: "0.2.0",
    changes: [
      "Images mode: convert JPEG, PNG, WebP, TIFF (and common RAW inputs)",
      "Audio / Images switch — separate pipelines, same local-first safety",
      "Bundled ImageMagick with locked-down policy.xml",
    ],
    debugs: [],
  },
  {
    version: "0.1.15",
    changes: [
      "Audio foundation complete for the 0.1 line (next major: images in 0.2)",
      "FFmpeg licensing pack: build identity, GPL notice, and source offer",
      "THIRD_PARTY_FFMPEG.txt shipped with the installer; About links in Settings",
    ],
    debugs: [
      "Documented exact Gyan 8.1 full GPL build used in release installers",
    ],
  },
  {
    version: "0.1.11",
    changes: [
      "MP3 encoding mode: CBR or VBR (V5 / V2 / V0)",
      "Quality presets show concrete bitrates or VBR grades",
    ],
    debugs: [],
  },
  {
    version: "0.1.10",
    changes: [
      "Separate outputs: M4A (AAC) vs AAC (ADTS raw .aac)",
      "Clearer format labels (ALAC M4A, OGG Vorbis, and more)",
      "Broader input extensions for folder import and file picker",
    ],
    debugs: [],
  },
  {
    version: "0.1.9",
    changes: [
      "Before Convert: honesty warnings for lossy→lossless and bit-depth upsampling",
      "Rough batch size estimate (source vs ~output) and destination free space",
      "Hard block when estimated output won't fit on the destination drive",
    ],
    debugs: [],
  },
  {
    version: "0.1.8",
    changes: [
      "Preserve tags (title, artist, album, and more) when converting — on by default",
      "Preserve embedded cover art when the destination format supports it",
      "Metadata panel with toggles; cover option disabled for WAV / AIFF",
    ],
    debugs: [],
  },
  {
    version: "0.1.7",
    changes: [
      "WAV / AIFF bit depth: Original (default), 16-bit, 24-bit, 32-bit float",
      "Richer source details in the file list (bit depth, rate, channels, bitrate, size)",
      "Content Security Policy enabled",
    ],
    debugs: [
      "Hardened Replace rollback when a copy partially fails",
      "Safer restore of the previous file if promote fails",
    ],
  },
  {
    version: "0.1.6",
    changes: [
      "Solid Settings panel (no see-through background or heavy shadow)",
      "What's New popup after you update and reopen",
      "Clearer update status text: “Checking for updates…”",
    ],
    debugs: [
      "Blocked Replace from overwriting the source file",
      "Hardened folder paths so outputs can't escape the destination",
      "Safer Replace (keeps the old file if the write fails)",
      "Fixed cancel / progress / double-Convert races",
      "Updater no longer clears an available update on a failed recheck",
      "Uninstall only wipes app data folders (not the whole product tree)",
      "FFmpeg/FFprobe limited to local file protocols",
    ],
  },
];

const STORAGE_KEY = "jwconverter.whatsNewSeenVersion";

export function getSeenWhatsNewVersion(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

export function setSeenWhatsNewVersion(version: string): void {
  try {
    localStorage.setItem(STORAGE_KEY, version);
  } catch {
    // Ignore storage failures — popup may show again next launch.
  }
}

/** Newest entry the user hasn't acknowledged yet (by semver-ish string compare). */
export function pendingWhatsNew(currentVersion: string): WhatsNewEntry | null {
  const seen = getSeenWhatsNewVersion();
  if (seen === currentVersion) {
    return null;
  }

  const match = WHATS_NEW.find((entry) => entry.version === currentVersion);
  return match ?? null;
}
