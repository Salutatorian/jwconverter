import {
  ImageIcon,
  InfoIcon,
  Link2Icon,
  Music2Icon,
  SettingsIcon,
} from "lucide-react";
import type { ReactNode } from "react";

export type MediaMode = "audio" | "images" | "links";

type AppShellProps = {
  mode: MediaMode;
  onModeChange: (mode: MediaMode) => void;
  modeLocked?: boolean;
  updateAvailable?: boolean;
  onOpenSettings: () => void;
  version?: string | null;
  /** Dev / experimental Links rail (Phase 1). */
  showLinks?: boolean;
  children: ReactNode;
};

export function AppShell({
  mode,
  onModeChange,
  modeLocked = false,
  updateAvailable = false,
  onOpenSettings,
  version,
  showLinks = false,
  children,
}: AppShellProps) {
  return (
    <div className="app-frame">
      <aside className="app-rail" aria-label="App navigation">
        <div className="app-rail-brand" title="JW Converter">
          <img src="/jwc-logo.png" alt="" draggable={false} />
        </div>

        <nav className="app-rail-nav">
          <button
            type="button"
            className={[
              "rail-btn",
              mode === "audio" ? "rail-btn-active" : "",
            ]
              .filter(Boolean)
              .join(" ")}
            aria-pressed={mode === "audio"}
            disabled={modeLocked && mode !== "audio"}
            onClick={() => onModeChange("audio")}
          >
            <Music2Icon aria-hidden />
            <span>audio</span>
          </button>
          <button
            type="button"
            className={[
              "rail-btn",
              mode === "images" ? "rail-btn-active" : "",
            ]
              .filter(Boolean)
              .join(" ")}
            aria-pressed={mode === "images"}
            disabled={modeLocked && mode !== "images"}
            onClick={() => onModeChange("images")}
          >
            <ImageIcon aria-hidden />
            <span>images</span>
          </button>
          {showLinks ? (
            <button
              type="button"
              className={[
                "rail-btn",
                mode === "links" ? "rail-btn-active" : "",
              ]
                .filter(Boolean)
                .join(" ")}
              aria-pressed={mode === "links"}
              disabled={modeLocked && mode !== "links"}
              onClick={() => onModeChange("links")}
              title="Experimental"
            >
              <Link2Icon aria-hidden />
              <span>links</span>
            </button>
          ) : null}
        </nav>

        <div className="app-rail-footer">
          <button
            type="button"
            className="rail-btn"
            onClick={onOpenSettings}
            aria-label={
              updateAvailable ? "Settings — update available" : "Settings"
            }
          >
            <span className="rail-btn-icon-wrap">
              <SettingsIcon aria-hidden />
              {updateAvailable ? <span className="rail-dot" /> : null}
            </span>
            <span>settings</span>
          </button>
          {version ? (
            <p className="app-rail-version" title={`v${version}`}>
              <InfoIcon aria-hidden className="size-3 opacity-50" />
              v{version}
            </p>
          ) : null}
        </div>
      </aside>

      <div className="app-stage">{children}</div>
    </div>
  );
}
