import { useState } from "react";
import { analyzeLink } from "../lib/tauri";
import type { AppInfo } from "../types/conversion";
import type { LinkMediaInfo } from "../types/links";

type LinkConverterViewProps = {
  appInfo: AppInfo | null;
};

function formatDuration(seconds: number | null): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0) {
    return "—";
  }
  const total = Math.round(seconds);
  const m = Math.floor(total / 60);
  const s = total % 60;
  if (m >= 60) {
    const h = Math.floor(m / 60);
    const mm = m % 60;
    return `${h}:${String(mm).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  }
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function LinkConverterView({ appInfo }: LinkConverterViewProps) {
  const [url, setUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<LinkMediaInfo | null>(null);

  async function handleAnalyze() {
    setError(null);
    setInfo(null);
    setBusy(true);
    try {
      const result = await analyzeLink(url);
      setInfo(result);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="app-shell">
      <header className="stage-header">
        <h1 className="stage-title">JW Converter</h1>
        <p className="stage-sub">
          Links · Experimental
          {appInfo ? ` · v${appInfo.version}` : ""}
        </p>
      </header>

      <section className="panel panel-compact" aria-label="Paste media URL">
        <h2 className="panel-title">Paste a public media URL</h2>
        <div className="mt-3 flex flex-col gap-2.5">
          <input
            type="url"
            className="link-url-input"
            placeholder="https://…"
            value={url}
            disabled={busy}
            spellCheck={false}
            autoComplete="off"
            onChange={(event) => setUrl(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && url.trim() && !busy) {
                void handleAnalyze();
              }
            }}
          />
          <div className="action-row">
            <button
              type="button"
              className="btn btn-primary"
              disabled={busy || !url.trim()}
              onClick={() => {
                void handleAnalyze();
              }}
            >
              {busy ? "Analyzing…" : "Analyze"}
            </button>
          </div>
          <p className="text-xs text-[var(--text-muted)]">
            Media is resolved on this computer. Compatibility varies by service.
            Phase 1 inspects metadata only — nothing is downloaded.
          </p>
        </div>
      </section>

      {error ? (
        <p
          className="rounded-[var(--radius)] border border-red-400/30 bg-[var(--danger-soft)] px-4 py-3 text-sm text-[var(--danger)]"
          role="alert"
        >
          {error}
        </p>
      ) : null}

      {info ? (
        <section className="panel panel-compact" aria-label="Link metadata">
          <h2 className="panel-title">Metadata</h2>
          <dl className="link-meta-grid mt-3">
            <div>
              <dt>Title</dt>
              <dd>{info.title ?? "—"}</dd>
            </div>
            <div>
              <dt>Creator</dt>
              <dd>{info.creator ?? "—"}</dd>
            </div>
            <div>
              <dt>Service</dt>
              <dd>{info.service ?? info.extractor ?? "—"}</dd>
            </div>
            <div>
              <dt>Duration</dt>
              <dd>{formatDuration(info.durationSeconds)}</dd>
            </div>
            <div>
              <dt>ID</dt>
              <dd className="mono text-xs">{info.id ?? "—"}</dd>
            </div>
            <div>
              <dt>Type</dt>
              <dd>
                {info.isLive
                  ? "Live"
                  : info.isPlaylist
                    ? `Playlist${info.itemCount != null ? ` · ${info.itemCount} items` : ""}`
                    : "Single item"}
              </dd>
            </div>
          </dl>
          {info.warnings.length > 0 ? (
            <ul className="mt-3 list-disc space-y-1 pl-5 text-xs text-[var(--text-muted)]">
              {info.warnings.map((warning) => (
                <li key={warning}>{warning}</li>
              ))}
            </ul>
          ) : null}
        </section>
      ) : null}
    </div>
  );
}
