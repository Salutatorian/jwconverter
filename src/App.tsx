import { useEffect, useState } from "react";
import { AppShell, type MediaMode } from "./components/AppShell";
import { SettingsPanel } from "./components/SettingsPanel";
import { UpdateOverlay } from "./components/UpdateOverlay";
import { WhatsNewModal } from "./components/WhatsNewModal";
import { useTheme } from "./hooks/useTheme";
import { useUpdater } from "./hooks/useUpdater";
import { getAppInfo } from "./lib/tauri";
import {
  pendingWhatsNew,
  setSeenWhatsNewVersion,
  type WhatsNewEntry,
} from "./lib/whatsNew";
import type { AppInfo } from "./types/conversion";
import { ConverterView } from "./views/ConverterView";
import { ImageConverterView } from "./views/ImageConverterView";
import { LinkConverterView } from "./views/LinkConverterView";

function App() {
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [ipcError, setIpcError] = useState<string | null>(null);
  const [whatsNew, setWhatsNew] = useState<WhatsNewEntry | null>(null);
  const [mode, setMode] = useState<MediaMode>("audio");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [modeLocked, setModeLocked] = useState(false);
  const updater = useUpdater();
  const theme = useTheme();
  const updateAvailable =
    updater.status === "available" || updater.status === "downloading";
  const showUpdateOverlay = updater.blockingOverlay;
  const showLinks = Boolean(appInfo?.linksExperimental);

  useEffect(() => {
    let cancelled = false;
    getAppInfo()
      .then((info) => {
        if (cancelled) {
          return;
        }
        setAppInfo(info);
        setWhatsNew(pendingWhatsNew(info.version));
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setIpcError(error instanceof Error ? error.message : String(error));
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!showLinks && mode === "links") {
      setMode("audio");
    }
  }, [showLinks, mode]);

  function dismissWhatsNew() {
    if (appInfo) {
      setSeenWhatsNewVersion(appInfo.version);
    }
    setWhatsNew(null);
  }

  return (
    <main className="h-full min-h-screen">
      {ipcError ? (
        <p className="px-6 pt-4 text-sm text-[var(--danger)]" role="alert">
          Could not reach the Rust backend: {ipcError}
        </p>
      ) : null}

      <AppShell
        mode={mode}
        onModeChange={setMode}
        modeLocked={modeLocked || showUpdateOverlay}
        updateAvailable={updateAvailable}
        onOpenSettings={() => {
          setSettingsOpen(true);
        }}
        version={appInfo?.version ?? null}
        showLinks={showLinks}
      >
        {mode === "audio" ? (
          <ConverterView appInfo={appInfo} onBusyChange={setModeLocked} />
        ) : mode === "images" ? (
          <ImageConverterView appInfo={appInfo} onBusyChange={setModeLocked} />
        ) : (
          <LinkConverterView appInfo={appInfo} />
        )}
      </AppShell>

      <SettingsPanel
        open={settingsOpen && !showUpdateOverlay}
        onClose={() => {
          setSettingsOpen(false);
        }}
        appInfo={appInfo}
        updater={updater}
        themePreference={theme.preference}
        onThemePreferenceChange={theme.setPreference}
      />

      {whatsNew && !showUpdateOverlay ? (
        <WhatsNewModal entry={whatsNew} onDismiss={dismissWhatsNew} />
      ) : null}

      {showUpdateOverlay ? <UpdateOverlay updater={updater} /> : null}
    </main>
  );
}

export default App;
