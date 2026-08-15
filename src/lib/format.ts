export function formatDuration(sec?: number | null): string {
  if (sec == null || !Number.isFinite(sec) || sec < 0) return "";
  const s = Math.round(sec);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = s % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(r).padStart(2, "0")}`;
  return `${m}:${String(r).padStart(2, "0")}`;
}

export function letterTile(title: string): string {
  const c = title.trim().charAt(0);
  return c ? c.toUpperCase() : "?";
}

export function resumePosition(positionSec: number, durationSec?: number | null): number {
  if (!Number.isFinite(positionSec) || positionSec < 5) return 0;
  if (durationSec != null && Number.isFinite(durationSec)) {
    if (positionSec > durationSec || positionSec > durationSec - 10) return 0;
  }
  return positionSec;
}
