import { useEffect, useState } from "react";
import { getAppInfo } from "./lib/tauri";
import type { AppInfo } from "./types/conversion";
import { ConverterView } from "./views/ConverterView";

function App() {
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [ipcError, setIpcError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    getAppInfo()
      .then((info) => {
        if (!cancelled) {
          setAppInfo(info);
        }
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

  return (
    <main className="min-h-screen">
      {ipcError ? (
        <p className="px-6 pt-4 text-sm text-red-300" role="alert">
          Could not reach the Rust backend: {ipcError}
        </p>
      ) : null}
      <ConverterView appInfo={appInfo} />
    </main>
  );
}

export default App;
