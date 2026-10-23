// Fertility cycle maths, mirroring crates/pregsys-core/src/fertility.rs.

import type { Settings } from "./types";
import { durationFor } from "./timeline";

export const CYCLE_DAYS = 28;
export const BABY_REVEAL_WEEK = 12;
export const GENDER_REVEAL_WEEK = 20;

const CURVE = [
  0.03, 0.03, 0.03, 0.03, 0.03, 0.03, 0.03, // 1-7
  0.08, 0.15, 0.35, 0.6, 0.85, 1.0, 0.9, // 8-14
  0.5, 0.2, 0.05, 0.03, 0.03, 0.03, 0.03, // 15-21
  0.03, 0.03, 0.03, 0.03, 0.03, 0.03, 0.03, // 22-28
];

export type Level = "low" | "medium" | "high" | "peak";

export const termMs = (s: Settings) => durationFor(s.defaultMode, s.defaultSpeed, s.defaultCustomMs);
export const cycleMs = (s: Settings) => Math.max(60_000, Math.floor(termMs(s) / 10));
export const graceMs = (s: Settings) => Math.max(60_000, Math.floor(termMs(s) / 20));

export function cycleDay(now: number, anchor: number, cycle: number): number {
  const into = (((now - anchor) % cycle) + cycle) % cycle;
  return 1 + (into / cycle) * CYCLE_DAYS;
}

export function chance(day: number, peak: number): number {
  const i = Math.min(28, Math.max(1, Math.floor(day))) - 1;
  return Math.min(1, Math.max(0, CURVE[i] * peak));
}

export function level(c: number, peak: number): Level {
  const r = peak > 0 ? c / peak : 0;
  return r >= 0.8 ? "peak" : r >= 0.45 ? "high" : r >= 0.12 ? "medium" : "low";
}

export const LEVEL_LABEL: Record<Level, string> = {
  low: "Low chance",
  medium: "Medium chance",
  high: "High chance",
  peak: "Peak fertility 💗",
};

/** Real time until the next day that's at least "high". */
export function untilFertile(now: number, anchor: number, cycle: number, peak: number): number {
  const dayMs = cycle / CYCLE_DAYS;
  for (let i = 0; i <= CYCLE_DAYS; i++) {
    const t = now + i * dayMs;
    const d = cycleDay(t, anchor, cycle);
    if (level(chance(d, peak), peak) === "high" || level(chance(d, peak), peak) === "peak") {
      // Snap to the start of that cycle day.
      return i === 0 ? 0 : t - ((d - Math.floor(d)) * dayMs) - now;
    }
  }
  return 0;
}

/** Days 9 to 16 are the fertile window (for drawing the ring). */
export const WINDOW = { start: 9, end: 16 };

export const MULTIPLES_LABEL = {
  realistic: { name: "Realistic", hint: "Twins about 3%" },
  playful: { name: "Playful", hint: "Twins 10%, triplets 2%" },
  lots: { name: "Lots of babies", hint: "Twins 25%, triplets 8%, quads 2%" },
} as const;

export function babyWord(n: number): string {
  return n === 1 ? "One baby" : n === 2 ? "Twins!" : n === 3 ? "Triplets!" : n === 4 ? "Quadruplets!" : `${n} babies!`;
}

export function genderEmoji(genders: string[]): string {
  return genders.map((g) => (g === "girl" ? "👧" : "👦")).join("");
}

/** "A girl!", "A girl and a boy!", "2 girls and a boy!" */
export function genderText(genders: string[]): string {
  const girls = genders.filter((g) => g === "girl").length;
  const boys = genders.length - girls;
  const part = (n: number, word: string) => (n === 0 ? "" : n === 1 ? `a ${word}` : `${n} ${word}s`);
  const text = [part(girls, "girl"), part(boys, "boy")].filter(Boolean).join(" and ");
  return text ? text.charAt(0).toUpperCase() + text.slice(1) + "!" : "";
}
