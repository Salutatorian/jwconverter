import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useEffect, useMemo, useState } from "react";
import { BitDepthPicker } from "../components/BitDepthPicker";
import { ConversionProgress } from "../components/ConversionProgress";
import { DestinationPicker } from "../components/DestinationPicker";
import { DropZone } from "../components/DropZone";
import { FileQueue } from "../components/FileQueue";
import { FormatPicker } from "../components/FormatPicker";
import { MetadataPicker } from "../components/MetadataPicker";
import { OverwritePicker } from "../components/OverwritePicker";
import { PreflightModal } from "../components/PreflightModal";
import { PrimaryActionBar } from "../components/PrimaryActionBar";
import { QualityPicker } from "../components/QualityPicker";
import { useBatchConversion } from "../hooks/useBatchConversion";
import { useFileQueue } from "../hooks/useFileQueue";
import { getDefaultPaths, preflightBatch, type PreflightReport } from "../lib/tauri";
import {
  AUDIO_EXTENSIONS,
  isLossyFormat,
  isPcmFormat,
  supportsEmbeddedCover,
  type AppInfo,
  type BitDepthPreset,
  type Mp3EncodingMode,
  type OutputFormat,
  type OverwritePolicy,
  type QualityPreset,
} from "../types/conversion";

type ConverterViewProps = {
  appInfo: AppInfo | null;
  onBusyChange?: (busy: boolean) => void;
};

function parentDir(path: string): string | null {
  const normalized = path.replace(/\//g, "\\");
  const index = normalized.lastIndexOf("\\");
  if (index <= 0) {
    return null;
  }
  return normalized.slice(0, index);
}

export function ConverterView({ appInfo, onBusyChange }: ConverterViewProps) {
  const [format, setFormat] = useState<OutputFormat>("flac");
  const [qualityPreset, setQualityPreset] = useState<QualityPreset>("medium");
  const [mp3EncodingMode, setMp3EncodingMode] =
    useState<Mp3EncodingMode>("cbr");
  const [bitDepthPreset, setBitDepthPreset] =
    useState<BitDepthPreset>("original");
  const [preserveTags, setPreserveTags] = useState(true);
  const [preserveCover, setPreserveCover] = useState(true);
  const [overwritePolicy, setOverwritePolicy] =
    useState<OverwritePolicy>("rename");
  const [destination, setDestination] = useState<string | null>(null);
  const [downloadsDir, setDownloadsDir] = useState<string | null>(null);
  const [dragActive, setDragActive] = useState(false);
  const [preflightReport, setPreflightReport] =
    useState<PreflightReport | null>(null);
  const [preflightError, setPreflightError] = useState<string | null>(null);

  const queue = useFileQueue();
  const batch = useBatchConversion(queue.patchByJobOrPath);

  useEffect(() => {
    onBusyChange?.(batch.isBusy);
    return () => {
      onBusyChange?.(false);
    };
  }, [batch.isBusy, onBusyChange]);

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

    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (batch.isBusy) {
          return;
        }
        if (event.payload.type === "enter" || event.payload.type === "over") {
          setDragActive(true);
          return;
        }
        if (event.payload.type === "leave") {
          setDragActive(false);
          return;
        }
        if (event.payload.type === "drop") {
          setDragActive(false);
          void queue.addPaths(event.payload.paths, true);
        }
      })
      .then((fn) => {
        if (cancelled) {
          fn();
        } else {
          unlisten = fn;
        }
      })
      .catch(() => {});

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [batch.isBusy, queue.addPaths]);

  const convertibleItems = useMemo(
    () =>
      queue.items.filter(
        (item) =>
          item.info != null &&
          (item.status === "ready" || item.status === "failed"),
      ),
    [queue.items],
  );

  const canConvert = Boolean(
    convertibleItems.length > 0 &&
      destination &&
      !queue.isAnalyzing &&
      !batch.isBusy,
  );

  const currentItem = queue.items.find(
    (item) =>
      item.status === "converting" ||
      item.status === "verifying" ||
      item.status === "queued",
  );

  const lastCompleted = [...queue.items]
    .reverse()
    .find((item) => item.status === "completed" && item.outputPath);

  const batchSummary = batch.batch
    ? `${batch.batch.completed} done · ${batch.batch.skipped} skipped · ${batch.batch.failed} failed · ${batch.batch.cancelled} cancelled · ${batch.batch.remaining} left`
    : null;

  const progressVisible =
    batch.isBusy ||
    batch.batch != null ||
    queue.items.some((item) =>
      [
        "queued",
        "converting",
        "verifying",
        "completed",
        "skipped",
        "failed",
        "cancelled",
      ].includes(item.status),
    );

  async function handleChooseFiles() {
    const selected = await open({
      multiple: true,
      directory: false,
      title: "Choose audio files",
      filters: [
        {
          name: "Audio",
          extensions: [...AUDIO_EXTENSIONS],
        },
      ],
    });

    if (selected == null) {
      return;
    }

    const paths = Array.isArray(selected) ? selected : [selected];
    await queue.addPaths(paths, true);
  }

  async function handleChooseInputFolder() {
    const selected = await open({
      multiple: false,
      directory: true,
      title: "Choose a music folder",
    });

    if (typeof selected === "string" && selected.length > 0) {
      await queue.addPaths([selected], true);
    }
  }

  async function handleChooseFolder() {
    const selected = await open({
      multiple: false,
      directory: true,
      title: "Choose output folder",
    });

    if (typeof selected === "string" && selected.length > 0) {
      setDestination(selected);
    }
  }

  async function startConvert() {
    if (!destination) {
      return;
    }
    await batch.convert({
      items: convertibleItems,
      destinationDir: destination,
      outputFormat: format,
      overwritePolicy,
      qualityPreset,
      mp3EncodingMode,
      bitDepthPreset,
      preserveTags,
      preserveCover: preserveCover && supportsEmbeddedCover(format),
      assignJobIds: queue.assignJobIds,
    });
  }

  async function handleConvert() {
    if (!destination) {
      return;
    }

    setPreflightError(null);
    try {
      const report = await preflightBatch({
        destinationDir: destination,
        outputFormat: format,
        qualityPreset,
        mp3EncodingMode,
        bitDepthPreset,
        overwritePolicy,
        items: convertibleItems.map((item) => ({
          sourcePath: item.path,
          relativeSubdir: item.relativeSubdir,
          durationSeconds: item.info?.durationSeconds ?? null,
          sampleRate: item.info?.sampleRate ?? null,
          channels: item.info?.channels ?? null,
          fileSizeBytes: item.info?.fileSizeBytes ?? null,
          codec: item.info?.codec ?? null,
          format: item.info?.format ?? null,
          bitDepth: item.info?.bitDepth ?? null,
          bitsPerRawSample: item.info?.bitsPerRawSample ?? null,
          sampleFormat: item.info?.sampleFormat ?? null,
        })),
      });

      if (report.diskBlocked || report.warnings.length > 0) {
        setPreflightReport(report);
        return;
      }

      await startConvert();
    } catch (error) {
      setPreflightError(
        error instanceof Error ? error.message : String(error),
      );
    }
  }

  async function handlePreflightContinue() {
    setPreflightReport(null);
    await startConvert();
  }

  return (
    <div className="app-shell">
      <header className="stage-header">
        <h1 className="stage-title">JW Converter</h1>
        <p className="stage-sub">
          Local audio conversion
          {appInfo ? ` · v${appInfo.version}` : ""}
        </p>
      </header>

      <DropZone
        mode="audio"
        disabled={queue.isAnalyzing || batch.isBusy}
        active={dragActive}
        analyzing={queue.isAnalyzing}
        actions={
          <div className="action-row action-row-center">
            <button
              type="button"
              className="btn btn-secondary"
              disabled={queue.isAnalyzing || batch.isBusy}
              onClick={() => {
                void handleChooseFiles();
              }}
            >
              {queue.isAnalyzing ? "Analyzing…" : "Choose files"}
            </button>
            <button
              type="button"
              className="btn btn-secondary"
              disabled={queue.isAnalyzing || batch.isBusy}
              onClick={() => {
                void handleChooseInputFolder();
              }}
            >
              Choose folder
            </button>
          </div>
        }
      />

      {batch.error ? (
        <p
          className="rounded-[var(--radius)] border border-red-400/30 bg-[var(--danger-soft)] px-4 py-3 text-sm text-[var(--danger)]"
          role="alert"
        >
          {batch.error}
        </p>
      ) : null}

      {preflightError ? (
        <p
          className="rounded-[var(--radius)] border border-red-400/30 bg-[var(--danger-soft)] px-4 py-3 text-sm text-[var(--danger)]"
          role="alert"
        >
          {preflightError}
        </p>
      ) : null}

      <FileQueue
        items={queue.items}
        disabled={batch.isBusy}
        onRemove={queue.removeItem}
        onRetry={queue.retryItem}
        onClear={() => {
          if (!batch.isBusy) {
            queue.clear();
            batch.reset();
          }
        }}
      />

      <div className="grid gap-2.5">
        <FormatPicker
          value={format}
          disabled={batch.isBusy}
          onChange={setFormat}
        />
        {isLossyFormat(format) ? (
          <QualityPicker
            format={format}
            value={qualityPreset}
            mp3EncodingMode={mp3EncodingMode}
            disabled={batch.isBusy}
            onChange={setQualityPreset}
            onMp3EncodingModeChange={setMp3EncodingMode}
          />
        ) : null}
        {isPcmFormat(format) ? (
          <BitDepthPicker
            value={bitDepthPreset}
            disabled={batch.isBusy}
            onChange={setBitDepthPreset}
          />
        ) : null}
        <MetadataPicker
          preserveTags={preserveTags}
          preserveCover={preserveCover}
          coverSupported={supportsEmbeddedCover(format)}
          disabled={batch.isBusy}
          onPreserveTagsChange={setPreserveTags}
          onPreserveCoverChange={setPreserveCover}
        />
        <OverwritePicker
          value={overwritePolicy}
          disabled={batch.isBusy}
          onChange={setOverwritePolicy}
        />
        <DestinationPicker
          destination={destination}
          disabled={batch.isBusy}
          onChooseFolder={() => {
            void handleChooseFolder();
          }}
          canUseDownloads={Boolean(downloadsDir)}
          onUseDownloads={() => {
            if (downloadsDir) {
              setDestination(downloadsDir);
            }
          }}
          canUseSourceFolder={
            convertibleItems.length === 1 &&
            !convertibleItems[0]?.relativeSubdir
          }
          onUseSourceFolder={() => {
            const only = convertibleItems[0];
            if (only) {
              const folder = parentDir(only.path);
              if (folder) {
                setDestination(folder);
              }
            }
          }}
        />
      </div>

      <ConversionProgress
        visible={progressVisible}
        status={batch.batch?.status ?? currentItem?.status ?? "idle"}
        percent={currentItem?.percent ?? null}
        message={batch.batch?.message ?? null}
        outputPath={lastCompleted?.outputPath ?? null}
        batchSummary={batchSummary}
        cancellable={batch.isBusy}
        onCancel={() => {
          void batch.cancel();
        }}
      />

      {preflightReport ? (
        <PreflightModal
          report={preflightReport}
          onCancel={() => {
            setPreflightReport(null);
          }}
          onContinue={
            preflightReport.diskBlocked
              ? undefined
              : () => {
                  void handlePreflightContinue();
                }
          }
        />
      ) : null}

      <PrimaryActionBar
        label={
          convertibleItems.length > 1
            ? `Convert ${convertibleItems.length}`
            : "Convert"
        }
        busyLabel="Converting…"
        busy={batch.isBusy}
        disabled={!canConvert}
        hint={
          !destination
            ? "Choose a destination folder first"
            : convertibleItems.length === 0
              ? "Add audio files to convert"
              : null
        }
        onAction={() => {
          void handleConvert();
        }}
      />
    </div>
  );
}
