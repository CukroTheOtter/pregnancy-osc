// The Movement chart: kicks in the last 24 hours, one bar per clock hour.

import type { KickStrength } from "./types";

export const HOUR_MS = 3_600_000;

export interface KickHour {
  startMs: number;
  weak: number;
  strong: number;
  extreme: number;
  total: number;
}

/** 24 hours, oldest first; the last one is the hour you're in now. */
export function kickHours(kicks: { timeMs: number; strength: KickStrength }[], now: number): KickHour[] {
  const top = new Date(now);
  top.setMinutes(0, 0, 0);
  const first = top.getTime() - 23 * HOUR_MS;
  const hours: KickHour[] = Array.from({ length: 24 }, (_, i) => ({ startMs: first + i * HOUR_MS, weak: 0, strong: 0, extreme: 0, total: 0 }));
  for (const k of kicks) {
    const i = Math.floor((k.timeMs - first) / HOUR_MS);
    if (i < 0 || i > 23 || k.timeMs > now) continue;
    hours[i][k.strength]++;
    hours[i].total++;
  }
  return hours;
}

/** The busiest hour (the latest one on a tie), or null with no kicks. */
export function busiest(hours: KickHour[]): KickHour | null {
  let best: KickHour | null = null;
  for (const h of hours) if (h.total && (!best || h.total >= best.total)) best = h;
  return best;
}
