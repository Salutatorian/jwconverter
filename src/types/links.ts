/** Experimental Links DTOs — Phase 1 metadata only. */

export interface LinkMediaInfo {
  originalUrl: string;
  webpageUrl: string | null;
  extractor: string | null;
  service: string | null;
  id: string | null;
  title: string | null;
  creator: string | null;
  durationSeconds: number | null;
  isLive: boolean;
  isPlaylist: boolean;
  itemCount: number | null;
  warnings: string[];
}
