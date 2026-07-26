import { openUrl } from "@tauri-apps/plugin-opener";
import {
  GlobeIcon,
  InfoIcon,
  RefreshCwIcon,
  Settings2Icon,
  WrenchIcon,
  XIcon,
} from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";
import type { UseUpdaterResult } from "../hooks/useUpdater";
import {
  FFMPEG_LICENSING_URL,
  FFMPEG_SOURCE_URL,
  GITHUB_ISSUES_URL,
  GITHUB_RELEASES_URL,
  GITHUB_REPO_URL,
  GYAN_FFMPEG_BUILDS_URL,
} from "../lib/links";
import { getMediaToolsInfo } from "../lib/tauri";
import type { ThemePreference } from "../lib/theme";
import type { AppInfo } from "../types/conversion";
import { UpdateControls } from "./UpdateControls";

type SettingsSection = "general" | "updates" | "about" | "advanced";

type SettingsDialogProps = {
  open: boolean;
  onClose: () => void;
  appInfo: AppInfo | null;
  updater: UseUpdaterResult;
  themePreference: ThemePreference;
  onThemePreferenceChange: (preference: ThemePreference) => void;
};

const NAV: {
  id: SettingsSection;
  label: string;
  icon: typeof Settings2Icon;
}[] = [
  { id: "general", label: "General", icon: Settings2Icon },
  { id: "updates", label: "Updates", icon: RefreshCwIcon },
  { id: "about", label: "About", icon: InfoIcon },
  { id: "advanced", label: "Advanced", icon: WrenchIcon },
];

async function openExternal(url: string) {
  try {
    await openUrl(url);
  } catch {
    window.open(url, "_blank", "noopener,noreferrer");
  }
}

export function SettingsDialog({
  open,
  onClose,
  appInfo,
  updater,
  themePreference,
  onThemePreferenceChange,
}: SettingsDialogProps) {
  const titleId = useId();
  const closeRef = useRef<HTMLButtonElement>(null);
  const [section, setSection] = useState<SettingsSection>("general");
  const [tools, setTools] = useState<{
    ffmpegPath: string | null;
    ffprobePath: string | null;
    magickPath: string | null;
    source: string;
  } | null>(null);

  useEffect(() => {
    if (!open) {
      return;
    }
    closeRef.current?.focus();

    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onClose();
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [open, onClose]);

  useEffect(() => {
    if (!open || section !== "advanced") {
      return;
    }
    let cancelled = false;
    getMediaToolsInfo()
      .then((info) => {
        if (!cancelled) {
          setTools(info);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setTools(null);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [open, section]);

  if (!open) {
    return null;
  }

  const sectionLabel = NAV.find((item) => item.id === section)?.label ?? "Settings";

  return (
    <div className="modal-overlay" role="presentation">
      <button
        type="button"
        className="modal-backdrop"
        aria-label="Close settings"
        onClick={onClose}
      />
      <section
        className="settings-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <aside className="settings-nav" aria-label="Settings sections">
          {NAV.map((item) => {
            const Icon = item.icon;
            const active = section === item.id;
            return (
              <button
                key={item.id}
                type="button"
                className={[
                  "settings-nav-item",
                  active ? "settings-nav-item-active" : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
                aria-current={active ? "page" : undefined}
                onClick={() => setSection(item.id)}
              >
                <Icon className="size-4 shrink-0 opacity-80" aria-hidden />
                <span>{item.label}</span>
              </button>
            );
          })}
        </aside>

        <div className="settings-pane">
          <header className="settings-pane-header">
            <p id={titleId} className="settings-breadcrumb">
              Settings <span aria-hidden>›</span>{" "}
              <strong>{sectionLabel}</strong>
            </p>
            <button
              ref={closeRef}
              type="button"
              className="settings-close"
              aria-label="Close settings"
              onClick={onClose}
            >
              <XIcon className="size-4" />
            </button>
          </header>

          <div className="settings-pane-body">
            {section === "general" ? (
              <div className="flex flex-col gap-5">
                <div className="flex flex-col gap-3">
                  <h3 className="text-base font-semibold text-[var(--text)]">
                    Appearance
                  </h3>
                  <p className="text-sm leading-relaxed text-[var(--text-muted)]">
                    Flat black or white. Default follows your system theme.
                  </p>
                  <div
                    className="chip-row"
                    role="group"
                    aria-label="Color theme"
                  >
                    {(
                      [
                        { value: "system", label: "System" },
                        { value: "dark", label: "Black" },
                        { value: "light", label: "White" },
                      ] as const
                    ).map((option) => (
                      <button
                        key={option.value}
                        type="button"
                        className="chip"
                        aria-pressed={themePreference === option.value}
                        onClick={() => {
                          onThemePreferenceChange(option.value);
                        }}
                      >
                        {option.label}
                      </button>
                    ))}
                  </div>
                </div>

                <div className="flex flex-col gap-3">
                  <h3 className="text-base font-semibold text-[var(--text)]">
                    Local conversion
                  </h3>
                  <p className="text-sm leading-relaxed text-[var(--text-muted)]">
                    JW Converter runs entirely on your machine. Switch between
                    Audio and Images from the rail. Sources are never modified;
                    outputs go to the destination you choose.
                  </p>
                </div>
              </div>
            ) : null}

            {section === "updates" ? (
              <div className="flex flex-col gap-3">
                <h3 className="text-base font-semibold text-[var(--text)]">
                  Updates
                </h3>
                <UpdateControls updater={updater} />
              </div>
            ) : null}

            {section === "about" ? (
              <div className="flex flex-col gap-4">
                <div>
                  <h3 className="text-base font-semibold text-[var(--text)]">
                    JW Converter
                    {appInfo ? ` · v${appInfo.version}` : null}
                  </h3>
                  <p className="mt-2 text-sm leading-relaxed text-[var(--text-muted)]">
                    Local-first audio and image conversion. No accounts, no
                    cloud upload, no telemetry. HEIC can be imported; HEIC
                    export needs a future ImageMagick build with write support.
                  </p>
                </div>

                <div>
                  <p className="panel-title">Links</p>
                  <div className="mt-2 flex flex-wrap gap-2">
                    <button
                      type="button"
                      className="btn btn-secondary"
                      onClick={() => {
                        void openExternal(GITHUB_REPO_URL);
                      }}
                    >
                      <GlobeIcon className="mr-1.5 inline size-3.5 opacity-70" />
                      GitHub
                    </button>
                    <button
                      type="button"
                      className="btn btn-secondary"
                      onClick={() => {
                        void openExternal(GITHUB_RELEASES_URL);
                      }}
                    >
                      Releases
                    </button>
                    <button
                      type="button"
                      className="btn btn-secondary"
                      onClick={() => {
                        void openExternal(GITHUB_ISSUES_URL);
                      }}
                    >
                      Issues
                    </button>
                  </div>
                </div>

                <div>
                  <p className="panel-title">Licensing</p>
                  <p className="mt-2 text-sm leading-relaxed text-[var(--text-muted)]">
                    Uses FFmpeg / FFprobe (GPL builds) for audio and ImageMagick
                    for images. See THIRD_PARTY_*.txt in the install folder for
                    build identity and source offers.
                  </p>
                  <div className="mt-2 flex flex-wrap gap-2">
                    <button
                      type="button"
                      className="btn btn-secondary"
                      onClick={() => {
                        void openExternal(FFMPEG_LICENSING_URL);
                      }}
                    >
                      FFmpeg licensing
                    </button>
                    <button
                      type="button"
                      className="btn btn-secondary"
                      onClick={() => {
                        void openExternal(FFMPEG_SOURCE_URL);
                      }}
                    >
                      FFmpeg source
                    </button>
                    <button
                      type="button"
                      className="btn btn-secondary"
                      onClick={() => {
                        void openExternal(GYAN_FFMPEG_BUILDS_URL);
                      }}
                    >
                      Gyan builds
                    </button>
                  </div>
                </div>
              </div>
            ) : null}

            {section === "advanced" ? (
              <div className="flex flex-col gap-3">
                <h3 className="text-base font-semibold text-[var(--text)]">
                  Media tools
                </h3>
                <p className="text-sm text-[var(--text-muted)]">
                  Read-only paths resolved for this install. Source:{" "}
                  <span className="text-[var(--text)]">
                    {tools?.source ?? "…"}
                  </span>
                </p>
                <dl className="settings-kv">
                  <div>
                    <dt>FFmpeg</dt>
                    <dd>{tools?.ffmpegPath ?? "Not found"}</dd>
                  </div>
                  <div>
                    <dt>FFprobe</dt>
                    <dd>{tools?.ffprobePath ?? "Not found"}</dd>
                  </div>
                  <div>
                    <dt>ImageMagick</dt>
                    <dd>{tools?.magickPath ?? "Not found"}</dd>
                  </div>
                </dl>
              </div>
            ) : null}
          </div>
        </div>
      </section>
    </div>
  );
}

/** @deprecated Prefer SettingsDialog — kept for existing imports. */
export { SettingsDialog as SettingsPanel };
