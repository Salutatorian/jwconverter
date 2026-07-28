import {
  MP3_ENCODING_MODES,
  qualityPresetLabel,
  type Mp3EncodingMode,
  type OutputFormat,
  type QualityPreset,
} from "../types/conversion";

type QualityPickerProps = {
  format: OutputFormat;
  value: QualityPreset;
  mp3EncodingMode: Mp3EncodingMode;
  disabled?: boolean;
  onChange: (preset: QualityPreset) => void;
  onMp3EncodingModeChange: (mode: Mp3EncodingMode) => void;
};

const PRESETS: QualityPreset[] = ["low", "medium", "high"];

export function QualityPicker({
  format,
  value,
  mp3EncodingMode,
  disabled = false,
  onChange,
  onMp3EncodingModeChange,
}: QualityPickerProps) {
  const showMp3Mode = format === "mp3";

  return (
    <section aria-label="Quality" className="panel panel-compact">
      <h2 className="panel-title">Quality</h2>
      {showMp3Mode ? (
        <div className="mt-2">
          <p className="mb-1.5 text-xs text-[var(--text-muted)]">Encoding</p>
          <div className="chip-row">
            {MP3_ENCODING_MODES.map((mode) => (
              <button
                key={mode.value}
                type="button"
                className="chip"
                disabled={disabled}
                aria-pressed={mp3EncodingMode === mode.value}
                onClick={() => onMp3EncodingModeChange(mode.value)}
              >
                {mode.label}
              </button>
            ))}
          </div>
        </div>
      ) : null}
      <div className={showMp3Mode ? "mt-3" : undefined}>
        {showMp3Mode ? (
          <p className="mb-1.5 text-xs text-[var(--text-muted)]">Preset</p>
        ) : null}
        <div className="chip-row">
          {PRESETS.map((preset) => (
            <button
              key={preset}
              type="button"
              className="chip"
              disabled={disabled}
              aria-pressed={value === preset}
              onClick={() => onChange(preset)}
            >
              {qualityPresetLabel(format, preset, mp3EncodingMode)}
            </button>
          ))}
        </div>
      </div>
    </section>
  );
}
