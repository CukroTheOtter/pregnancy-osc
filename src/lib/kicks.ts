// The Movement chart: kicks in this (or the last) VRChat session, or the
// last 24 hours when there's no session to show.

import type { KickStrength } from "./types";

export const HOUR_MS = 3_600_000;
const MIN_MS = 60_000;

export interface KickHour {
  startMs: number;
  weak: number;
  strong: number;
  extreme: number;
  total: number;
}

type Kick = { timeMs: number; strength: KickStrength };

/** 24 hours, oldest first; the last one is the hour you're in now. */
export function kickHours(kicks: Kick[], now: number): KickHour[] {
  const top = new Date(now);
  top.setMinutes(0, 0, 0);
  return fill(kicks, top.getTime() - 23 * HOUR_MS, HOUR_MS, 24, now);
}

/** Bar sizes, smallest first: a session gets the smallest that fits in 24 bars. */
const STEPS = [5, 10, 15, 30, 60, 120, 180, 240, 360, 720, 1440].map((m) => m * MIN_MS);

/** A session's kicks in bars of an even size (whole clock minutes/hours), oldest first. */
export function kickBins(kicks: Kick[], fromMs: number, toMs: number): { bins: KickHour[]; binMs: number } {
  const span = Math.max(1, toMs - fromMs);
  const binMs = STEPS.find((s) => span / s <= 24) ?? STEPS[STEPS.length - 1];
  // Line the bars up with the local clock (an hour bar starts at :00).
  const off = new Date(fromMs).getTimezoneOffset() * MIN_MS;
  const first = Math.floor((fromMs - off) / binMs) * binMs + off;
  const count = Math.max(1, Math.ceil((toMs - first) / binMs));
  const inside = kicks.filter((k) => k.timeMs >= fromMs);
  return { bins: fill(inside, first, binMs, count, toMs), binMs };
}

function fill(kicks: Kick[], first: number, binMs: number, count: number, end: number): KickHour[] {
  const bins: KickHour[] = Array.from({ length: count }, (_, i) => ({ startMs: first + i * binMs, weak: 0, strong: 0, extreme: 0, total: 0 }));
  for (const k of kicks) {
    const i = Math.floor((k.timeMs - first) / binMs);
    if (i < 0 || i >= count || k.timeMs > end) continue;
    bins[i][k.strength]++;
    bins[i].total++;
  }
  return bins;
}

/** An SVG path through the points, smoothly curved (it never overshoots below a flat stretch by much). */
export function smoothPath(pts: [number, number][]): string {
  if (!pts.length) return "";
  const r = (n: number) => Math.round(n * 10) / 10;
  if (pts.length === 1) return `M${r(pts[0][0] - 4)},${r(pts[0][1])} L${r(pts[0][0] + 4)},${r(pts[0][1])}`;
  let d = `M${r(pts[0][0])},${r(pts[0][1])}`;
  for (let i = 0; i < pts.length - 1; i++) {
    const [p0, p1, p2, p3] = [pts[i - 1] ?? pts[i], pts[i], pts[i + 1], pts[i + 2] ?? pts[i + 1]];
    const c1 = [p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6];
    const c2 = [p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6];
    d += ` C${r(c1[0])},${r(c1[1])} ${r(c2[0])},${r(c2[1])} ${r(p2[0])},${r(p2[1])}`;
  }
  return d;
}

/** The busiest bar (the latest one on a tie), or null with no kicks. */
export function busiest(hours: KickHour[]): KickHour | null {
  let best: KickHour | null = null;
  for (const h of hours) if (h.total && (!best || h.total >= best.total)) best = h;
  return best;
}
