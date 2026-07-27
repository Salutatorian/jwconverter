export type WhatsNewEntry = {
  version: string;
  changes: string[];
  debugs: string[];
};

/** Shown once after updating to this version (first launch). */
export const WHATS_NEW: WhatsNewEntry[] = [
  {
    version: "0.5.0",
    changes: [
      "macOS DMG on GitHub Releases (Apple Silicon, unsigned — right-click → Open)",
      "CI builds attach Mac (and Linux AppImage) assets to the release tag",
    ],
    debugs: [
      "macOS is not notarized yet; Gatekeeper will warn until Apple Developer ID signing",
      "Mac auto-update is not in latest.json yet — download DMG from Releases",
    ],
  },
  {
    version: "0.4.1",
    changes: [
      "Flat black / white backgrounds — removed the noisy grain overlay",
      "Theme: System (default), Black, or White — follows OS theme when set to System",
    ],
    debugs: [],
  },
  {
    version: "0.4.0",
    changes: [
      "Full UI revamp — cobalt-style shell with left rail for Audio / Images / Settings",
      "Pill controls, near-black canvas, monospace chrome",
      "Attachment file queues kept for clear batch status + retry",
    ],
    debugs: [
      "Settings still only General / Updates / About / Advanced (no fake SaaS pages)",
    ],
  },
  {
    version: "0.3.0",
    changes: [
      "Attachment-style file queue — clear ready / converting / done / error states with retry",
      "Settings sidebar: General, Updates, About, Advanced (media tool paths)",
      "Polished drop zone and convert shell to match the new UI",
      "macOS + Linux builds via GitHub Actions (Windows remains the primary signed release)",
    ],
    debugs: [
      "macOS builds are unsigned — Gatekeeper may require right-click Open until notarization",
      "On Mac/Linux, Images mode may use a system ImageMagick install when no portable tree is bundled",
    ],
  },
  {
    version: "0.2.8",
    changes: [
      "Clearer empty states for Audio and Images (what to drop, format hints)",
      "Honest HEIC messaging: import supported, export not available on this Magick build",
      "Updated README screenshot for the dual-mode app",
      "Extra soak coverage (Unicode paths, cancel mid-convert)",
    ],
    debugs: [
      "HEIC export still blocked until a write-capable ImageMagick is bundled",
    ],
  },
  {
    version: "0.2.7",
    changes: [
      "Image line polish (was briefly 0.2.1–0.2.6) in one release",
      "Resize presets + image preflight; EXIF orientation; clearer RAW errors",
      "WebP Lossless + PNG compression; BMP / GIF / AVIF outputs",
      "Preserve image metadata (default on)",
      "Dual-mode polish: Audio + Images product copy and UX parity",
    ],
    debugs: [
      "HEIC export still unavailable — bundled Magick is HEIC read-only",
    ],
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
