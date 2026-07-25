import { useEffect, useState } from "react";
import { WhatsNewModal } from "./components/WhatsNewModal";
import { getAppInfo } from "./lib/tauri";
import {
  pendingWhatsNew,
  setSeenWhatsNewVersion,
  type WhatsNewEntry,
} from "./lib/whatsNew";
import type { AppInfo } from "./types/conversion";
import { ConverterView } from "./views/ConverterView";
import { ImageConverterView } from "./views/ImageConverterView";

type MediaMode = "audio" | "images";

function App() {
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [ipcError, setIpcError] = useState<string | null>(null);
  const [whatsNew, setWhatsNew] = useState<WhatsNewEntry | null>(null);
  const [mode, setMode] = useState<MediaMode>("audio");

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

  function dismissWhatsNew() {
    if (appInfo) {
      setSeenWhatsNewVersion(appInfo.version);
    }
    setWhatsNew(null);
  }

  return (
    <main className="min-h-screen">
      {ipcError ? (
        <p className="px-6 pt-4 text-sm text-red-300" role="alert">
          Could not reach the Rust backend: {ipcError}
        </p>
      ) : null}
      {mode === "audio" ? (
        <ConverterView
          appInfo={appInfo}
          onSwitchToImages={() => {
            setMode("images");
          }}
        />
      ) : (
        <ImageConverterView
          appInfo={appInfo}
          onSwitchToAudio={() => {
            setMode("audio");
          }}
        />
      )}
      {whatsNew ? (
        <WhatsNewModal entry={whatsNew} onDismiss={dismissWhatsNew} />
      ) : null}
    </main>
  );
}

export default App;
