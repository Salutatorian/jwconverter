import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useEffect, useMemo, useState } from "react";
import { ConversionProgress } from "../components/ConversionProgress";
import { DestinationPicker } from "../components/DestinationPicker";
import { DropZone } from "../components/DropZone";
import { ImageFileQueue } from "../components/ImageFileQueue";
import { ImageMetadataPicker } from "../components/ImageMetadataPicker";
import { OverwritePicker } from "../components/OverwritePicker";
import { PreflightModal } from "../components/PreflightModal";
import { useImageBatchConversion } from "../hooks/useImageBatchConversion";
import { useImageFileQueue } from "../hooks/useImageFileQueue";
import {
  getDefaultPaths,
  preflightImageBatch,
  type PreflightReport,
} from "../lib/tauri";
import type { AppInfo, OverwritePolicy } from "../types/conversion";
import {
  IMAGE_EXTENSIONS,
  IMAGE_OUTPUT_FORMATS,
  IMAGE_RESIZE_PRESETS,
  qualityPresetsForFormat,
  showsImageQualityControls,
  type ImageOutputFormat,
  type ImageQualityPreset,
  type ImageResizePreset,
} from "../types/image";

type ImageConverterViewProps = {
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

export function ImageConverterView({
  appInfo,
  onBusyChange,
}: ImageConverterViewProps) {
  const [format, setFormat] = useState<ImageOutputFormat>("jpeg");
  const [qualityPreset, setQualityPreset] =
    useState<ImageQualityPreset>("medium");
  const [resizePreset, setResizePreset] =
    useState<ImageResizePreset>("original");
  const [preserveMetadata, setPreserveMetadata] = useState(true);
  const [overwritePolicy, setOverwritePolicy] =
    useState<OverwritePolicy>("rename");
  const [destination, setDestination] = useState<string | null>(null);
  const [downloadsDir, setDownloadsDir] = useState<string | null>(null);
  const [dragActive, setDragActive] = useState(false);
  const [preflightReport, setPreflightReport] =
    useState<PreflightReport | null>(null);
  const [preflightError, setPreflightError] = useState<string | null>(null);

  const queue = useImageFileQueue();
  const batch = useImageBatchConversion(queue.patchByJobOrPath);

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
      title: "Choose images",
      filters: [
        {
          name: "Images",
          extensions: [...IMAGE_EXTENSIONS],
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
      title: "Choose a photos folder",
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
      resizePreset,
      preserveMetadata,
      assignJobIds: queue.assignJobIds,
    });
  }

  async function handleConvert() {
    if (!destination) {
      return;
    }

    setPreflightError(null);
    try {
      const report = await preflightImageBatch({
        destinationDir: destination,
        outputFormat: format,
        qualityPreset,
        resizePreset,
        overwritePolicy,
        items: convertibleItems.map((item) => ({
          sourcePath: item.path,
          relativeSubdir: item.relativeSubdir,
          width: item.info?.width ?? null,
          height: item.info?.height ?? null,
          fileSizeBytes: item.info?.fileSizeBytes ?? null,
          format: item.info?.format ?? null,
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
          Local image conversion
          {appInfo ? ` · v${appInfo.version}` : ""}
        </p>
      </header>

      <DropZone
        mode="images"
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
              {queue.isAnalyzing ? "Analyzing…" : "Choose images"}
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
            <button
              type="button"
              className="btn btn-primary"
              disabled={!canConvert}
              onClick={() => {
                void handleConvert();
              }}
            >
              {batch.isBusy
                ? "Converting…"
                : convertibleItems.length > 1
                  ? `Convert ${convertibleItems.length}`
                  : "Convert"}
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

      <ImageFileQueue
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
        <section aria-label="Output format" className="panel panel-compact">
          <h2 className="panel-title">Output format</h2>
          <div className="chip-row">
            {IMAGE_OUTPUT_FORMATS.map((item) => (
              <button
                key={item.value}
                type="button"
                className="chip"
                disabled={batch.isBusy}
                aria-pressed={format === item.value}
                onClick={() => {
                  setFormat(item.value);
                  if (item.value !== "webp" && qualityPreset === "lossless") {
                    setQualityPreset("medium");
                  }
                }}
              >
                {item.label}
              </button>
            ))}
          </div>
        </section>

        {showsImageQualityControls(format) ? (
          <section aria-label="Quality" className="panel panel-compact">
            <h2 className="panel-title">
              {format === "png" ? "Compression" : "Quality"}
            </h2>
            <div className="chip-row">
              {qualityPresetsForFormat(format).map((item) => (
                <button
                  key={item.value}
                  type="button"
                  className="chip"
                  disabled={batch.isBusy}
                  aria-pressed={qualityPreset === item.value}
                  onClick={() => setQualityPreset(item.value)}
                >
                  {item.label}
                </button>
              ))}
            </div>
          </section>
        ) : null}

        <section aria-label="Resize" className="panel panel-compact">
          <h2 className="panel-title">Resize</h2>
          <div className="chip-row">
            {IMAGE_RESIZE_PRESETS.map((item) => (
              <button
                key={item.value}
                type="button"
                className="chip"
                disabled={batch.isBusy}
                title="Long edge max — never upscales"
                aria-pressed={resizePreset === item.value}
                onClick={() => setResizePreset(item.value)}
              >
                {item.label}
              </button>
            ))}
          </div>
        </section>

        <ImageMetadataPicker
          preserveMetadata={preserveMetadata}
          disabled={batch.isBusy}
          onPreserveMetadataChange={setPreserveMetadata}
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
    </div>
  );
}
