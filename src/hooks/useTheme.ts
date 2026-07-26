import { useEffect, useState } from "react";
import {
  applyThemePreference,
  readThemePreference,
  writeThemePreference,
  type ThemePreference,
} from "../lib/theme";

export function useTheme() {
  const [preference, setPreferenceState] = useState<ThemePreference>(() =>
    readThemePreference(),
  );

  useEffect(() => {
    applyThemePreference(preference);

    if (preference !== "system") {
      return;
    }

    const media = window.matchMedia("(prefers-color-scheme: light)");
    function onChange() {
      applyThemePreference("system");
    }
    media.addEventListener("change", onChange);
    return () => {
      media.removeEventListener("change", onChange);
    };
  }, [preference]);

  function setPreference(next: ThemePreference) {
    writeThemePreference(next);
    setPreferenceState(next);
    applyThemePreference(next);
  }

  return { preference, setPreference };
}
