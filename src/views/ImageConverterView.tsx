import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useEffect, useMemo, useState } from "react";
import { ConversionProgress } from "../components/ConversionProgress";
import { DestinationPicker } from "../components/DestinationPicker";
import { DropZone } from "../components/DropZone";
import { ImageFileQueue } from "../components/ImageFileQueue";
import { OverwritePicker } from "../components/OverwritePicker";
import { SettingsGearButton } from "../components/SettingsGearButton";
import { SettingsPanel } from "../components/SettingsPanel";
import { useImageBatchConversion } from "../hooks/useImageBatchConversion";
import { useImageFileQueue } from "../hooks/useImageFileQueue";
import { useUpdater } from "../hooks/useUpdater";
import { getDefaultPaths } from "../lib/tauri";
import type { AppInfo, OverwritePolicy } from "../types/conversion";
import {
  IMAGE_EXTENSIONS,
  IMAGE_OUTPUT_FORMATS,
  IMAGE_QUALITY_PRESETS,
  isLossyImageFormat,
  type ImageOutputFormat,
  type ImageQualityPreset,
} from "../types/image";

type ImageConverterViewProps = {
  appInfo: AppInfo | null;
  onSwitchToAudio: () => void;
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
  onSwitchToAudio,
}: ImageConverterViewProps) {
  const [format, setFormat] = useState<ImageOutputFormat>("jpeg");
  const [qualityPreset, setQualityPreset] =
    useState<ImageQualityPreset>("medium");
  const [overwritePolicy, setOverwritePolicy] =
    useState<OverwritePolicy>("rename");
  const [destination, setDestination] = useState<string | null>(null);
  const [downloadsDir, setDownloadsDir] = useState<string | null>(null);
  const [dragActive, setDragActive] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);

  const queue = useImageFileQueue();
  const batch = useImageBatchConversion(queue.patchByJobOrPath);
  const updater = useUpdater();
  const updateAvailable =
    updater.status === "available" || updater.status === "downloading";

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

  async function handleConvert() {
    if (!destination) {
      return;
    }
    await batch.convert({
      items: convertibleItems,
      destinationDir: destination,
      outputFormat: format,
      overwritePolicy,
      qualityPreset,
      assignJobIds: queue.assignJobIds,
    });
  }

  return (
    <div className="app-shell">
      <header className="flex items-start gap-3.5">
        <img
          src="/jwc-logo.png"
          alt=""
          className="mt-0.5 h-8 w-auto select-none"
          draggable={false}
        />
        <div className="min-w-0 flex-1">
          <h1 className="text-xl font-semibold tracking-tight text-[var(--text)]">
            JW Converter
          </h1>
          <p className="text-sm text-[var(--text-muted)]">
            Local image conversion
            {appInfo ? (
              <span className="text-[var(--text-faint)]">
                {" "}
                · v{appInfo.version}
              </span>
            ) : null}
          </p>
          <div className="mt-2 flex flex-wrap gap-2">
            <button type="button" className="chip" disabled aria-pressed>
              Images
            </button>
            <button
              type="button"
              className="chip"
              onClick={onSwitchToAudio}
              disabled={batch.isBusy}
            >
              Audio
            </button>
          </div>
        </div>
        <SettingsGearButton
          updateAvailable={updateAvailable}
          onClick={() => {
            setSettingsOpen(true);
          }}
        />
      </header>

      <SettingsPanel
        open={settingsOpen}
        onClose={() => {
          setSettingsOpen(false);
        }}
        appInfo={appInfo}
        updater={updater}
      />

      <DropZone
        disabled={queue.isAnalyzing || batch.isBusy}
        active={dragActive}
        analyzing={queue.isAnalyzing}
      />

      <div className="flex flex-wrap gap-2">
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

      {batch.error ? (
        <p
          className="rounded-[var(--radius)] border border-red-400/30 bg-[var(--danger-soft)] px-4 py-3 text-sm text-[var(--danger)]"
          role="alert"
        >
          {batch.error}
        </p>
      ) : null}

      <ImageFileQueue
        items={queue.items}
        disabled={batch.isBusy}
        onRemove={queue.removeItem}
        onClear={() => {
          if (!batch.isBusy) {
            queue.clear();
            batch.reset();
          }
        }}
      />

      <div className="grid gap-3">
        <section aria-label="Output format" className="panel">
          <h2 className="panel-title">Format</h2>
          <div className="chip-row">
            {IMAGE_OUTPUT_FORMATS.map((item) => (
              <button
                key={item.value}
                type="button"
                className="chip"
                disabled={batch.isBusy}
                aria-pressed={format === item.value}
                onClick={() => setFormat(item.value)}
              >
                {item.label}
              </button>
            ))}
          </div>
        </section>

        {isLossyImageFormat(format) ? (
          <section aria-label="Quality" className="panel">
            <h2 className="panel-title">Quality</h2>
            <div className="chip-row">
              {IMAGE_QUALITY_PRESETS.map((item) => (
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
    </div>
  );
}
