export type WhatsNewEntry = {
  version: string;
  changes: string[];
  debugs: string[];
};

/** Shown once after updating to this version (first launch). */
export const WHATS_NEW: WhatsNewEntry[] = [
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
