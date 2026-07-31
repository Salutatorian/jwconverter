import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useRef, useState } from "react";
import { BitDepthPicker } from "../components/BitDepthPicker";
import { DestinationPicker } from "../components/DestinationPicker";
import { OverwritePicker } from "../components/OverwritePicker";
import { QualityPicker } from "../components/QualityPicker";
import {
  analyzeLink,
  cancelLinkDownload,
  getDefaultPaths,
  startLinkDownload,
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
import { qualityPresetLabel } from "../types/conversion";
import type {
  LinkAudioFormat,
  LinkDownloadEvent,
  LinkMediaInfo,
  LinkMediaMode,
} from "../types/links";

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

function audioFormatToOutput(format: LinkAudioFormat): OutputFormat | null {
  switch (format) {
    case "mp3":
      return "mp3";
    case "m4a":
      return "m4a";
    case "opus":
      return "opus";
    case "flac":
      return "flac";
    case "wav":
      return "wav";
    default:
      return null;
  }
}

export function LinkConverterView({ appInfo }: LinkConverterViewProps) {
  const [url, setUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<LinkMediaInfo | null>(null);
  const [destination, setDestination] = useState<string | null>(null);
  const [downloadsDir, setDownloadsDir] = useState<string | null>(null);
  const [overwritePolicy, setOverwritePolicy] =
    useState<OverwritePolicy>("rename");
  const [mode, setMode] = useState<LinkMediaMode>("video");
  const [videoHeight, setVideoHeight] = useState<number | null>(null);
  const [audioFormat, setAudioFormat] = useState<LinkAudioFormat>("original");
  const [qualityPreset, setQualityPreset] = useState<QualityPreset>("medium");
  const [mp3EncodingMode, setMp3EncodingMode] =
    useState<Mp3EncodingMode>("cbr");
  const [bitDepthPreset, setBitDepthPreset] =
    useState<BitDepthPreset>("original");
  const [jobId, setJobId] = useState<string | null>(null);
  const jobIdRef = useRef<string | null>(null);
  const [status, setStatus] = useState<JobStatus>("idle");
  const [percent, setPercent] = useState<number | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [outputPath, setOutputPath] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    getDefaultPaths()
      .then((paths) => {
        if (cancelled || !paths.downloadsDir) {
          return;
        }
        setDownloadsDir(paths.downloadsDir);
        setDestination((current) => current ?? paths.downloadsDir);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    listen<LinkDownloadEvent>("link-download-event", (event) => {
      if (event.payload.jobId !== jobIdRef.current) {
        return;
      }
      setStatus(event.payload.status);
      setPercent(event.payload.percent);
      setMessage(event.payload.message);
      if (event.payload.outputPath) {
        setOutputPath(event.payload.outputPath);
      }
      if (event.payload.error) {
        setError(event.payload.error);
      }
    })
      .then((dispose) => {
        if (cancelled) {
          dispose();
        } else {
          unlisten = dispose;
        }
      })
      .catch(() => {});
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  async function handleAnalyze() {
    setError(null);
    setInfo(null);
    setBusy(true);
    try {
      const result = await analyzeLink(url);
      setInfo(result);
      setVideoHeight(null);
      setStatus("ready");
      jobIdRef.current = null;
      setJobId(null);
      setPercent(null);
      setMessage(null);
      setOutputPath(null);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  const downloading = ["queued", "converting", "verifying"].includes(status);
  const downloadBlocked = !info || info.isLive || info.isPlaylist;
  const outputFormat = audioFormatToOutput(audioFormat);
  const showQuality =
    mode === "audio" &&
    (audioFormat === "mp3" || audioFormat === "m4a" || audioFormat === "opus");
  const showBitDepth = mode === "audio" && audioFormat === "wav";
  const showLossyWarning =
    mode === "audio" &&
    (audioFormat === "flac" || audioFormat === "wav") &&
    Boolean(info?.sourceAudioLikelyLossy);

  async function handleChooseFolder() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Choose download destination",
    });
    if (typeof selected === "string") {
      setDestination(selected);
    }
  }

  async function handleDownload() {
    if (!info || !destination || downloading || downloadBlocked) {
      return;
    }
    setError(null);
    setOutputPath(null);
    setStatus("queued");
    setPercent(0);
    setMessage("Preparing download");
    try {
      const startedJobId = crypto.randomUUID();
      jobIdRef.current = startedJobId;
      setJobId(startedJobId);
      await startLinkDownload({
        jobId: startedJobId,
        url: url.trim(),
        destinationDir: destination,
        overwritePolicy,
        mode,
        videoQuality: videoHeight == null ? "best" : { height: videoHeight },
        audioFormat,
        qualityPreset,
        mp3EncodingMode,
        bitDepthPreset,
      });
    } catch (err: unknown) {
      jobIdRef.current = null;
      setJobId(null);
      setStatus("failed");
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleCancel() {
    if (!jobId) {
      return;
    }
    try {
      await cancelLinkDownload(jobId);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
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
            disabled={busy || downloading}
            spellCheck={false}
            autoComplete="off"
            onChange={(event) => setUrl(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && url.trim() && !busy && !downloading) {
                void handleAnalyze();
              }
            }}
          />
          <div className="action-row">
            <button
              type="button"
              className="btn btn-primary"
              disabled={busy || downloading || !url.trim()}
              onClick={() => {
                void handleAnalyze();
              }}
            >
              {busy ? "Analyzing…" : "Analyze"}
            </button>
          </div>
          <p className="text-xs text-[var(--text-muted)]">
            Media is resolved and downloaded on this computer. Compatibility
            varies by service.
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
        <>
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

          <section className="panel panel-compact" aria-label="Download options">
            <h2 className="panel-title">Download options</h2>
            <div className="mt-3 grid gap-3">
              <div>
                <p className="text-xs text-[var(--text-muted)]">Media</p>
                <div className="chip-row">
                  {(["video", "audio"] as const).map((value) => (
                    <button
                      key={value}
                      type="button"
                      className="chip"
                      disabled={downloading}
                      aria-pressed={mode === value}
                      onClick={() => setMode(value)}
                    >
                      {value === "video" ? "Video" : "Audio"}
                    </button>
                  ))}
                </div>
              </div>
              {mode === "video" ? (
                <div>
                  <p className="text-xs text-[var(--text-muted)]">Quality</p>
                  <div className="chip-row">
                    <button
                      type="button"
                      className="chip"
                      disabled={downloading}
                      aria-pressed={videoHeight == null}
                      onClick={() => setVideoHeight(null)}
                    >
                      Best
                    </button>
                    {info.videoOptions.map((option) => (
                      <button
                        key={option.id}
                        type="button"
                        className="chip"
                        disabled={downloading}
                        aria-pressed={videoHeight === option.height}
                        onClick={() => setVideoHeight(option.height)}
                      >
                        {option.label}
                      </button>
                    ))}
                  </div>
                </div>
              ) : (
                <div>
                  <p className="text-xs text-[var(--text-muted)]">Audio format</p>
                  <div className="chip-row">
                    {(
                      [
                        "original",
                        "mp3",
                        "m4a",
                        "opus",
                        "flac",
                        "wav",
                      ] as const
                    ).map((value) => (
                      <button
                        key={value}
                        type="button"
                        className="chip"
                        disabled={downloading}
                        aria-pressed={audioFormat === value}
                        onClick={() => setAudioFormat(value)}
                      >
                        {value === "original" ? "Original" : value.toUpperCase()}
                      </button>
                    ))}
                  </div>
                  {audioFormat === "original" ? (
                    <p className="mt-2 text-xs text-[var(--text-muted)]">
                      Download / remux — uses the best available source audio
                      without unnecessary transcoding
                      {info.bestAudioCodec
                        ? ` (${info.bestAudioCodec}${info.bestAudioExt ? ` · ${info.bestAudioExt}` : ""})`
                        : ""}
                      .
                    </p>
                  ) : (
                    <p className="mt-2 text-xs text-[var(--text-muted)]">
                      Transcoded using FFmpeg
                      {outputFormat
                        ? ` · ${qualityPresetLabel(outputFormat, qualityPreset, mp3EncodingMode)}`
                        : ""}
                      .
                    </p>
                  )}
                </div>
              )}
            </div>
          </section>

          {showQuality && outputFormat ? (
            <QualityPicker
              format={outputFormat}
              value={qualityPreset}
              mp3EncodingMode={mp3EncodingMode}
              disabled={downloading}
              onChange={setQualityPreset}
              onMp3EncodingModeChange={setMp3EncodingMode}
            />
          ) : null}

          {showBitDepth ? (
            <BitDepthPicker
              value={bitDepthPreset}
              disabled={downloading}
              onChange={setBitDepthPreset}
            />
          ) : null}

          {showLossyWarning ? (
            <p
              className="rounded-[var(--radius)] border border-[var(--border)] bg-[var(--surface-2)] px-4 py-3 text-sm text-[var(--text-muted)]"
              role="status"
            >
              Converting lossy source audio to {audioFormat.toUpperCase()} will
              not restore discarded detail — the output will usually be larger.
            </p>
          ) : null}

          <OverwritePicker
            value={overwritePolicy}
            disabled={downloading}
            onChange={setOverwritePolicy}
          />
          <DestinationPicker
            destination={destination}
            disabled={downloading}
            onChooseFolder={() => {
              void handleChooseFolder();
            }}
            canUseDownloads={Boolean(downloadsDir)}
            onUseDownloads={() => {
              if (downloadsDir) {
                setDestination(downloadsDir);
              }
            }}
          />
          <section
            className="panel panel-compact"
            aria-label="Link download progress"
          >
            <div className="action-row">
              <button
                type="button"
                className="btn btn-primary"
                disabled={!destination || downloadBlocked || downloading}
                onClick={() => {
                  void handleDownload();
                }}
              >
                Download
              </button>
              {downloading ? (
                <button
                  type="button"
                  className="btn btn-secondary"
                  onClick={() => {
                    void handleCancel();
                  }}
                >
                  Cancel
                </button>
              ) : null}
            </div>
            {message ? (
              <p className="mt-3 text-sm text-[var(--text-muted)]">
                {message}
                {percent != null ? ` · ${Math.round(percent)}%` : ""}
              </p>
            ) : null}
            {outputPath ? (
              <p className="mt-2 break-all text-xs text-[var(--text-muted)]">
                Saved to {outputPath}
              </p>
            ) : null}
          </section>
        </>
      ) : null}
    </div>
  );
}
