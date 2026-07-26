export type ThemePreference = "system" | "dark" | "light";

const STORAGE_KEY = "jwconverter.theme";

export function readThemePreference(): ThemePreference {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === "system" || raw === "dark" || raw === "light") {
      return raw;
    }
  } catch {
    // ignore
  }
  return "system";
}

export function writeThemePreference(preference: ThemePreference) {
  try {
    localStorage.setItem(STORAGE_KEY, preference);
  } catch {
    // ignore
  }
}

export function resolveTheme(
  preference: ThemePreference,
  matchesLight?: boolean,
): "dark" | "light" {
  if (preference === "dark" || preference === "light") {
    return preference;
  }
  const light =
    matchesLight ??
    window.matchMedia("(prefers-color-scheme: light)").matches;
  return light ? "light" : "dark";
}

/** Apply preference to <html>. Call before first paint when possible. */
export function applyThemePreference(preference: ThemePreference) {
  const root = document.documentElement;
  root.dataset.theme = preference;
  const resolved = resolveTheme(preference);
  root.style.colorScheme = resolved;
}
