// Timeline math, mirroring crates/pregsys-core/src/timeline.rs.
// Progress is always computed from the start time, never stored.

import type { DurationMode, Timeline } from "./types";

export const DAY_MS = 86_400_000;
export const TERM_DAYS = 280;
export const TERM_WEEKS = 40;

export function durationFor(mode: DurationMode, speed: number, customMs?: number | null): number {
  const term = TERM_DAYS * DAY_MS;
  if (mode === "realtime") return term;
  if (mode === "accelerated") return Math.floor(term / Math.max(speed, 0.001));
  return Math.max(customMs ?? term, 60_000);
}

export function newTimeline(startMs: number, mode: DurationMode, speed: number, customMs?: number | null): Timeline {
  const s = mode === "realtime" ? 1 : speed;
  return { startMs, durationMs: durationFor(mode, s, customMs), mode, speed: s, pausedAtMs: null, pausedTotalMs: 0 };
}

export function elapsedMs(t: Timeline, now: number): number {
  const end = t.pausedAtMs ?? now;
  return Math.max(0, end - t.startMs - t.pausedTotalMs);
}

export function progress(t: Timeline, now: number): number {
  return t.durationMs <= 0 ? 1 : elapsedMs(t, now) / t.durationMs;
}

export function week(t: Timeline, now: number): number {
  return progress(t, now) * TERM_WEEKS;
}

export function dueMs(t: Timeline, now: number): number {
  const running = t.pausedAtMs != null ? now - t.pausedAtMs : 0;
  return t.startMs + t.pausedTotalMs + running + t.durationMs;
}

/** Real time at which a given progress is reached (ignores future pauses). */
export function timeAtProgress(t: Timeline, now: number, p: number): number {
  const running = t.pausedAtMs != null ? now - t.pausedAtMs : 0;
  return t.startMs + t.pausedTotalMs + running + p * t.durationMs;
}

export function pause(t: Timeline, now: number): Timeline {
  return t.pausedAtMs != null ? t : { ...t, pausedAtMs: now };
}

export function resume(t: Timeline, now: number): Timeline {
  if (t.pausedAtMs == null) return t;
  return { ...t, pausedTotalMs: t.pausedTotalMs + Math.max(0, now - t.pausedAtMs), pausedAtMs: null };
}

/** Change speed without a jump: progress stays, only time left changes. */
export function retime(t: Timeline, now: number, mode: DurationMode, speed: number, customMs?: number | null): Timeline {
  const p = progress(t, now);
  const s = mode === "realtime" ? 1 : speed;
  const durationMs = durationFor(mode, s, customMs);
  const end = t.pausedAtMs ?? now;
  return { ...t, mode, speed: s, durationMs, startMs: Math.round(end - t.pausedTotalMs - p * durationMs) };
}

export function setProgress(t: Timeline, now: number, p: number): Timeline {
  const end = t.pausedAtMs ?? now;
  return { ...t, startMs: Math.round(end - t.pausedTotalMs - Math.max(0, p) * t.durationMs) };
}

export function trimester(p: number): 1 | 2 | 3 {
  const w = p * TERM_WEEKS;
  return w < 13 ? 1 : w < 27 ? 2 : 3;
}

/** "3 days 4 h", "12 min", "now". */
export function humanDuration(ms: number): string {
  const abs = Math.abs(ms);
  const m = Math.floor(abs / 60_000);
  const h = Math.floor(m / 60);
  const d = Math.floor(h / 24);
  if (d >= 60) return `${Math.round(d / 7)} weeks`;
  if (d >= 1) return `${d} day${d === 1 ? "" : "s"}${h % 24 ? ` ${h % 24} h` : ""}`;
  if (h >= 1) return `${h} h${m % 60 ? ` ${m % 60} min` : ""}`;
  if (m >= 1) return `${m} min`;
  return "now";
}

export function speedLabel(t: Timeline): string {
  if (t.mode === "realtime") return "Real time (about 9 months)";
  if (t.mode === "accelerated") {
    const daysPerWeek = 7 / t.speed;
    if (Math.abs(t.speed - 7) < 1e-6) return "1 real day = 1 week";
    if (daysPerWeek >= 1) return `${+daysPerWeek.toFixed(2)} real days = 1 week`;
    return `${+(daysPerWeek * 24).toFixed(2)} real hours = 1 week`;
  }
  return `Custom: full term in ${humanDuration(t.durationMs)}`;
}
