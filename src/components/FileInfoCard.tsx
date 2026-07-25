import type { AudioInfo } from "../types/conversion";
import { formatDuration, formatFileSize } from "../lib/format";

type FileInfoCardProps = {
  info: AudioInfo | null;
};

export function FileInfoCard({ info }: FileInfoCardProps) {
  if (!info) {
    return (
      <section
        aria-label="File information"
        className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-5"
      >
        <h2 className="text-sm font-semibold tracking-wide text-[var(--text-muted)] uppercase">
          File information
        </h2>
        <p className="mt-3 text-sm text-[var(--text-muted)]">
          No file selected yet.
        </p>
      </section>
    );
  }

  const rows: Array<{ label: string; value: string }> = [
    { label: "Filename", value: info.filename },
    { label: "Format", value: info.format ?? "—" },
    { label: "Codec", value: info.codec ?? "—" },
    { label: "Duration", value: formatDuration(info.durationSeconds) },
    {
      label: "Sample rate",
      value: info.sampleRate ? `${info.sampleRate} Hz` : "—",
    },
    {
      label: "Channels",
      value: info.channels != null ? String(info.channels) : "—",
    },
    { label: "File size", value: formatFileSize(info.fileSizeBytes) },
  ];

  return (
    <section
      aria-label="File information"
      className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-5"
    >
      <h2 className="text-sm font-semibold tracking-wide text-[var(--text-muted)] uppercase">
        File information
      </h2>
      <dl className="mt-4 grid gap-3 sm:grid-cols-2">
        {rows.map((row) => (
          <div key={row.label}>
            <dt className="text-xs text-[var(--text-muted)]">{row.label}</dt>
            <dd className="mt-0.5 truncate text-sm text-[var(--text)]">
              {row.value}
            </dd>
          </div>
        ))}
      </dl>
    </section>
  );
}
