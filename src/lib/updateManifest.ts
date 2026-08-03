/** Shared latest.json helpers for Windows auto-update + Mac/Linux download reminders. */

export const UPDATE_MANIFEST_URL =
  "https://github.com/Salutatorian/jwconverter/releases/latest/download/latest.json";

export const RELEASES_PAGE_URL =
  "https://github.com/Salutatorian/jwconverter/releases/latest";

export type HostOs = "windows" | "macos" | "linux";

export type UpdateManifest = {
  version: string;
  notes?: string;
  platforms?: Record<string, { url?: string; signature?: string }>;
};

export function detectHostOs(): HostOs {
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes("mac")) {
    return "macos";
  }
  if (ua.includes("linux")) {
    return "linux";
  }
  return "windows";
}

/** Compare dotted versions; returns <0 / 0 / >0 like strcmp. Non-numeric parts treated as 0. */
export function compareVersions(a: string, b: string): number {
  const left = a.trim().replace(/^v/i, "").split(".");
  const right = b.trim().replace(/^v/i, "").split(".");
  const length = Math.max(left.length, right.length);
  for (let index = 0; index < length; index += 1) {
    const l = Number.parseInt(left[index] ?? "0", 10);
    const r = Number.parseInt(right[index] ?? "0", 10);
    const ln = Number.isFinite(l) ? l : 0;
    const rn = Number.isFinite(r) ? r : 0;
    if (ln !== rn) {
      return ln - rn;
    }
  }
  return 0;
}

export function platformAssetUrl(version: string, os: HostOs): string {
  const tag = version.trim().replace(/^v/i, "");
  switch (os) {
    case "macos":
      return `https://github.com/Salutatorian/jwconverter/releases/download/v${tag}/JW.Converter_${tag}_macos_aarch64.dmg`;
    case "linux":
      return `https://github.com/Salutatorian/jwconverter/releases/download/v${tag}/JW.Converter_${tag}_linux_x86_64.AppImage`;
    case "windows":
      return `https://github.com/Salutatorian/jwconverter/releases/download/v${tag}/JW.Converter_${tag}_x64-setup.exe`;
    default: {
      const _exhaustive: never = os;
      return _exhaustive;
    }
  }
}

export async function fetchUpdateManifest(): Promise<UpdateManifest> {
  const response = await fetch(UPDATE_MANIFEST_URL, { cache: "no-store" });
  if (!response.ok) {
    throw new Error(`Could not reach update server (${response.status}).`);
  }
  const data = (await response.json()) as UpdateManifest;
  if (!data.version || typeof data.version !== "string") {
    throw new Error("Update manifest is missing a version.");
  }
  return data;
}
