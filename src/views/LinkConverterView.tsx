import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useState } from "react";
import { BitDepthPicker } from "../components/BitDepthPicker";
import { DestinationPicker } from "../components/DestinationPicker";
import { OverwritePicker } from "../components/OverwritePicker";
import { QualityPicker } from "../components/QualityPicker";
import {
  analyzeLink,
  cancelLinkBatch,
  cancelLinkDownload,
  clearLinkHistory,
  enqueueLinkDownloads,
  getDefaultPaths,
  getYtdlpVersion,
  listLinkHistory,
  updateYtdlp,
} from "../lib/tauri";
import type {
  AppInfo,
  BitDepthPreset,
  JobStatus,
  Mp3EncodingMode,
  OutputFormat,
  OverwritePolicy,
  QualityPreset,
} from "../types/conversion";
import type {
  LinkAudioFormat,
  LinkBatchEvent,
  LinkDownloadEvent,
  LinkHistoryItem,
  LinkMediaInfo,
  LinkMediaMode,
  LinkPlaylistEntry,
} from "../types/links";

type LinkConverterViewProps = { appInfo: AppInfo | null };

type QueueItem = {
  jobId: string;
  url: string;
  status: JobStatus;
  percent: number | null;
  message: string;
  outputPath: string | null;
  error: string | null;
};

function formatDuration(seconds: number | null): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0) return "—";
  const total = Math.round(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const remaining = total % 60;
  return hours
    ? `${hours}:${String(minutes).padStart(2, "0")}:${String(remaining).padStart(2, "0")}`
    : `${minutes}:${String(remaining).padStart(2, "0")}`;
}

function audioFormatToOutput(format: LinkAudioFormat): OutputFormat | null {
  switch (format) {
    case "mp3": return "mp3";
    case "m4a": return "m4a";
    case "opus": return "opus";
    case "flac": return "flac";
    case "wav": return "wav";
    default: return null;
  }
}

function validUrls(value: string): string[] {
  return value.split(/\r?\n/).map((url) => url.trim()).filter(Boolean);
}

export function LinkConverterView({ appInfo }: LinkConverterViewProps) {
  const [urlsText, setUrlsText] = useState("");
  const [info, setInfo] = useState<LinkMediaInfo | null>(null);
  const [selectedEntries, setSelectedEntries] = useState<Set<number>>(new Set());
  const [destination, setDestination] = useState<string | null>(null);
  const [downloadsDir, setDownloadsDir] = useState<string | null>(null);
  const [overwritePolicy, setOverwritePolicy] = useState<OverwritePolicy>("rename");
  const [mode, setMode] = useState<LinkMediaMode>("video");
  const [videoHeight, setVideoHeight] = useState<number | null>(null);
  const [audioFormat, setAudioFormat] = useState<LinkAudioFormat>("original");
  const [qualityPreset, setQualityPreset] = useState<QualityPreset>("medium");
  const [mp3EncodingMode, setMp3EncodingMode] = useState<Mp3EncodingMode>("cbr");
  const [bitDepthPreset, setBitDepthPreset] = useState<BitDepthPreset>("original");
  const [liveMaxMinutes, setLiveMaxMinutes] = useState(15);
  const [cookiesPath, setCookiesPath] = useState<string | null>(null);
  const [downloadSubtitles, setDownloadSubtitles] = useState(false);
  const [saveThumbnail, setSaveThumbnail] = useState(false);
  const [embedThumbnail, setEmbedThumbnail] = useState(false);
  const [queue, setQueue] = useState<Record<string, QueueItem>>({});
  const [batchId, setBatchId] = useState<string | null>(null);
  const [batchMessage, setBatchMessage] = useState<string | null>(null);
  const [history, setHistory] = useState<LinkHistoryItem[]>([]);
  const [ytdlpVersion, setYtdlpVersion] = useState<string | null>(null);
  const [busy, setBusy] = useState<"analyze" | "enqueue" | "update" | null>(null);
  const [error, setError] = useState<string | null>(null);

  const urls = useMemo(() => validUrls(urlsText), [urlsText]);
  const queueItems = Object.values(queue);
  const activeQueue = queueItems.some(({ status }) =>
    ["queued", "converting", "verifying"].includes(status),
  );
  const outputFormat = audioFormatToOutput(audioFormat);
  const showQuality = mode === "audio" && ["mp3", "m4a", "opus"].includes(audioFormat);
  const showBitDepth = mode === "audio" && audioFormat === "wav";

  async function refreshHistory() {
    try {
      setHistory(await listLinkHistory());
    } catch {
      // History is optional while the experimental backend is unavailable.
    }
  }

  useEffect(() => {
    let cancelled = false;
    void getDefaultPaths().then((paths) => {
      if (!cancelled && paths.downloadsDir) {
        setDownloadsDir(paths.downloadsDir);
        setDestination((current) => current ?? paths.downloadsDir);
      }
    }).catch(() => {});
    void getYtdlpVersion().then((version) => {
      if (!cancelled) setYtdlpVersion(version);
    }).catch(() => {});
    void refreshHistory();
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];
    void Promise.all([
      listen<LinkDownloadEvent>("link-download-event", ({ payload }) => {
        setQueue((current) => ({
          ...current,
          [payload.jobId]: {
            ...(current[payload.jobId] ?? { jobId: payload.jobId, url: "" }),
            ...payload,
          },
        }));
        if (["completed", "failed", "cancelled", "skipped"].includes(payload.status)) {
          void refreshHistory();
        }
      }),
      listen<LinkBatchEvent>("link-batch-event", ({ payload }) => {
        setBatchMessage(`${payload.message} · ${payload.completed}/${payload.total}`);
      }),
    ]).then((listeners) => {
      if (disposed) listeners.forEach((dispose) => dispose());
      else unlisteners.push(...listeners);
    }).catch(() => {});
    return () => {
      disposed = true;
      unlisteners.forEach((dispose) => dispose());
    };
  }, []);

  async function handleAnalyze() {
    const primaryUrl = urls[0];
    if (!primaryUrl) return;
    setBusy("analyze");
    setError(null);
    try {
      const result = await analyzeLink(primaryUrl, cookiesPath);
      setInfo(result);
      setVideoHeight(null);
      setSelectedEntries(new Set(result.entries.map((_, index) => index)));
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(null);
    }
  }

  async function chooseCookies() {
    const selected = await open({
      directory: false,
      multiple: false,
      title: "Choose browser cookies file",
    });
    if (typeof selected === "string") setCookiesPath(selected);
  }

  async function chooseDestination() {
    const selected = await open({ directory: true, multiple: false, title: "Choose download destination" });
    if (typeof selected === "string") setDestination(selected);
  }

  function playlistUrls(entries: LinkPlaylistEntry[]): string[] {
    return entries.flatMap((entry, index) =>
      selectedEntries.has(index) && entry.url ? [entry.url] : [],
    );
  }

  async function handleEnqueue(selectedUrls = urls) {
    if (!destination || !selectedUrls.length || activeQueue) return;
    setBusy("enqueue");
    setError(null);
    try {
      const items = selectedUrls.map((url) => {
        const playlistEntry = info?.entries.find((entry) => entry.url === url);
        const isPrimary = url === info?.originalUrl;
        return {
          url,
          title: playlistEntry?.title ?? (isPrimary ? info?.title : null),
          durationSeconds:
            playlistEntry?.durationSeconds ??
            (isPrimary ? info?.durationSeconds : null),
          isLive: playlistEntry?.isLive ?? (isPrimary ? info?.isLive : null),
        };
      });
      const anyLive = items.some((item) => item.isLive);
      const result = await enqueueLinkDownloads({
        destinationDir: destination,
        overwritePolicy,
        mode,
        videoQuality: videoHeight == null ? "best" : { height: videoHeight },
        audioFormat,
        qualityPreset,
        mp3EncodingMode,
        bitDepthPreset,
        liveMaxMinutes: anyLive || info?.isLive ? liveMaxMinutes : null,
        cookiesPath,
        downloadSubtitles,
        saveThumbnail,
        embedThumbnail: mode === "audio" && embedThumbnail,
        items,
      });
      setBatchId(result.batchId);
      setBatchMessage(
        `Queued ${result.jobIds.length} download${result.jobIds.length === 1 ? "" : "s"}`,
      );
      setQueue(
        Object.fromEntries(
          result.jobIds.map((jobId, index) => [
            jobId,
            {
              jobId,
              url: selectedUrls[index] ?? "",
              status: "queued" as JobStatus,
              percent: 0,
              message: "Preparing download",
              outputPath: null,
              error: null,
            },
          ]),
        ),
      );
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(null);
    }
  }

  async function handleUpdate() {
    setBusy("update");
    setError(null);
    try {
      setYtdlpVersion(await updateYtdlp());
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(null);
    }
  }

  const disabled = busy !== null || activeQueue;

  return (
    <div className="app-shell">
      <header className="stage-header">
        <h1 className="stage-title">JW Converter</h1>
        <p className="stage-sub">Links{appInfo ? ` · v${appInfo.version}` : ""}</p>
      </header>

      <section className="panel panel-compact" aria-label="Media URLs">
        <div className="flex items-center justify-between gap-3">
          <h2 className="panel-title">Public media URLs</h2>
          <span className="text-xs text-[var(--text-muted)]">
            yt-dlp {ytdlpVersion ?? "unavailable"}
          </span>
        </div>
        <textarea
          className="link-url-textarea mt-3"
          placeholder={"https://…\nhttps://…"}
          value={urlsText}
          disabled={disabled}
          spellCheck={false}
          autoComplete="off"
          rows={4}
          onChange={(event) => setUrlsText(event.target.value)}
        />
        <div className="action-row mt-3">
          <button
            type="button"
            className="btn btn-primary"
            disabled={!urls.length || disabled}
            onClick={() => {
              void handleAnalyze();
            }}
          >
            {busy === "analyze" ? "Analyzing…" : "Analyze first URL"}
          </button>
          <button
            type="button"
            className="btn btn-secondary"
            disabled={!urls.length || !destination || disabled}
            onClick={() => {
              void handleEnqueue();
            }}
          >
            {info?.isLive ? `Record ${liveMaxMinutes} min` : "Add URLs to queue"}
          </button>
          <button
            type="button"
            className="btn btn-secondary"
            disabled={busy === "update"}
            onClick={() => {
              void handleUpdate();
            }}
          >
            {busy === "update" ? "Updating…" : "Update downloader"}
          </button>
        </div>
        <p className="mt-2 text-xs text-[var(--text-muted)]">
          One URL per line. Compatibility varies by service.
        </p>
      </section>

      {error ? <p className="rounded-[var(--radius)] border border-red-400/30 bg-[var(--danger-soft)] px-4 py-3 text-sm text-[var(--danger)]" role="alert">{error}</p> : null}

      {info ? <section className="panel panel-compact" aria-label="Link metadata">
        <h2 className="panel-title">Metadata</h2>
        <dl className="link-meta-grid mt-3">
          <div><dt>Title</dt><dd>{info.title ?? "—"}</dd></div>
          <div><dt>Creator</dt><dd>{info.creator ?? "—"}</dd></div>
          <div><dt>Service</dt><dd>{info.service ?? info.extractor ?? "—"}</dd></div>
          <div><dt>Duration</dt><dd>{formatDuration(info.durationSeconds)}</dd></div>
        </dl>
        {info.warnings.map((warning) => <p key={warning} className="mt-2 text-xs text-[var(--text-muted)]">{warning}</p>)}
      </section> : null}

      {info?.isPlaylist && info.entries.length ? <section className="panel panel-compact" aria-label="Playlist selection">
        <div className="flex items-center justify-between gap-3">
          <h2 className="panel-title">Playlist items</h2>
          <div className="flex gap-2 text-xs">
            <button type="button" className="btn btn-secondary" disabled={disabled} onClick={() => setSelectedEntries(new Set(info.entries.map((_, i) => i)))}>Select all</button>
            <button type="button" className="btn btn-secondary" disabled={disabled} onClick={() => setSelectedEntries(new Set())}>Select none</button>
          </div>
        </div>
        <div className="mt-3 max-h-64 overflow-y-auto">
          {info.entries.map((entry, index) => <label key={`${entry.id ?? entry.url ?? index}`} className="flex items-center gap-3 border-b border-[var(--border)] py-2 text-sm">
            <input type="checkbox" checked={selectedEntries.has(index)} disabled={disabled}
              onChange={() => setSelectedEntries((current) => { const next = new Set(current); next.has(index) ? next.delete(index) : next.add(index); return next; })} />
            <span className="min-w-0 flex-1 truncate">{entry.title ?? entry.url ?? `Item ${index + 1}`}</span>
            <span className="text-xs text-[var(--text-muted)]">{formatDuration(entry.durationSeconds)}</span>
          </label>)}
        </div>
        <div className="action-row mt-3">
          <button type="button" className="btn btn-primary" disabled={!destination || !playlistUrls(info.entries).length || disabled}
            onClick={() => void handleEnqueue(playlistUrls(info.entries))}>Enqueue selected</button>
        </div>
      </section> : null}

      <section className="panel panel-compact" aria-label="Download options">
        <h2 className="panel-title">Download options</h2>
        <div className="mt-3 grid gap-3">
          <div><p className="text-xs text-[var(--text-muted)]">Media</p><div className="chip-row">
            {(["video", "audio"] as const).map((value) => <button key={value} type="button" className="chip" disabled={disabled} aria-pressed={mode === value} onClick={() => setMode(value)}>{value === "video" ? "Video" : "Audio"}</button>)}
          </div></div>
          {mode === "video" ? <div><p className="text-xs text-[var(--text-muted)]">Quality</p><div className="chip-row">
            <button type="button" className="chip" disabled={disabled} aria-pressed={videoHeight == null} onClick={() => setVideoHeight(null)}>Best</button>
            {(info?.videoOptions ?? []).map((option) => <button key={option.id} type="button" className="chip" disabled={disabled} aria-pressed={videoHeight === option.height} onClick={() => setVideoHeight(option.height)}>{option.label}</button>)}
          </div></div> : <div><p className="text-xs text-[var(--text-muted)]">Audio format</p><div className="chip-row">
            {(["original", "mp3", "m4a", "opus", "flac", "wav"] as const).map((value) => <button key={value} type="button" className="chip" disabled={disabled} aria-pressed={audioFormat === value} onClick={() => setAudioFormat(value)}>{value === "original" ? "Original" : value.toUpperCase()}</button>)}
          </div></div>}
          {info?.isLive ? <div><p className="text-xs text-[var(--text-muted)]">Record up to</p><div className="chip-row">{[5, 15, 30, 60].map((minutes) => <button key={minutes} type="button" className="chip" disabled={disabled} aria-pressed={liveMaxMinutes === minutes} onClick={() => setLiveMaxMinutes(minutes)}>{minutes} min</button>)}</div></div> : null}
          <div><p className="text-xs text-[var(--text-muted)]">Cookies (optional)</p><div className="action-row mt-1"><button type="button" className="btn btn-secondary" disabled={disabled} onClick={() => void chooseCookies()}>Choose cookies file</button>{cookiesPath ? <><span className="truncate text-xs text-[var(--text-muted)]">{cookiesPath}</span><button type="button" className="btn btn-secondary" disabled={disabled} onClick={() => setCookiesPath(null)}>Clear</button></> : null}</div></div>
          <div className="flex flex-wrap gap-x-5 gap-y-2 text-sm">
            <label><input type="checkbox" checked={downloadSubtitles} disabled={disabled} onChange={(event) => setDownloadSubtitles(event.target.checked)} /> Download subtitles</label>
            <label><input type="checkbox" checked={saveThumbnail} disabled={disabled} onChange={(event) => setSaveThumbnail(event.target.checked)} /> Save thumbnail</label>
            {mode === "audio" ? <label><input type="checkbox" checked={embedThumbnail} disabled={disabled} onChange={(event) => setEmbedThumbnail(event.target.checked)} /> Embed thumbnail</label> : null}
          </div>
        </div>
      </section>

      {showQuality && outputFormat ? <QualityPicker format={outputFormat} value={qualityPreset} mp3EncodingMode={mp3EncodingMode} disabled={disabled} onChange={setQualityPreset} onMp3EncodingModeChange={setMp3EncodingMode} /> : null}
      {showBitDepth ? <BitDepthPicker value={bitDepthPreset} disabled={disabled} onChange={setBitDepthPreset} /> : null}
      <OverwritePicker value={overwritePolicy} disabled={disabled} onChange={setOverwritePolicy} />
      <DestinationPicker destination={destination} disabled={disabled} onChooseFolder={() => void chooseDestination()} canUseDownloads={Boolean(downloadsDir)} onUseDownloads={() => setDestination(downloadsDir)} />

      <section className="panel panel-compact" aria-label="Link download queue">
        <div className="flex items-center justify-between gap-3"><h2 className="panel-title">Queue</h2>{batchId && activeQueue ? <button type="button" className="btn btn-secondary" onClick={() => void cancelLinkBatch()}>Cancel queue</button> : null}</div>
        {batchMessage ? <p className="mt-2 text-sm text-[var(--text-muted)]">{batchMessage}</p> : null}
        {queueItems.length ? <div className="mt-3 space-y-2">{queueItems.map((item) => <div key={item.jobId} className="rounded-[var(--radius)] border border-[var(--border)] px-3 py-2 text-sm">
          <div className="flex justify-between gap-3"><span className="min-w-0 truncate">{item.url || item.jobId}</span><span className="shrink-0 text-[var(--text-muted)]">{item.status}{item.percent != null ? ` · ${Math.round(item.percent)}%` : ""}</span></div>
          <p className="mt-1 text-xs text-[var(--text-muted)]">{item.error ?? item.message}</p>
          {["queued", "converting", "verifying"].includes(item.status) ? (
            <button
              type="button"
              className="btn btn-secondary mt-2"
              onClick={() => {
                void cancelLinkDownload(item.jobId).catch((err: unknown) => {
                  setError(err instanceof Error ? err.message : String(err));
                });
              }}
            >
              Cancel
            </button>
          ) : null}
        </div>)}</div> : <p className="mt-2 text-sm text-[var(--text-muted)]">No downloads queued.</p>}
      </section>

      <section className="panel panel-compact" aria-label="Link download history">
        <div className="flex items-center justify-between gap-3"><h2 className="panel-title">History</h2><button type="button" className="btn btn-secondary" disabled={!history.length} onClick={() => void clearLinkHistory().then(refreshHistory)}>Clear</button></div>
        {history.length ? <div className="mt-3 space-y-2">{history.map((item) => <div key={item.jobId} className="text-sm"><p>{item.title ?? item.url ?? "Untitled link"}</p><p className="text-xs text-[var(--text-muted)]">{item.status}{item.outputPath ? ` · ${item.outputPath}` : ""}</p></div>)}</div> : <p className="mt-2 text-sm text-[var(--text-muted)]">No download history yet.</p>}
      </section>
    </div>
  );
}
