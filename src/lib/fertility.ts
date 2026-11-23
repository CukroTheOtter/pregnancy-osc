// Fertility cycle maths, mirroring crates/pregsys-core/src/fertility.rs.

import type { FertileHours, Litter, LitterSpread, Settings } from "./types";
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
export const graceMs = (s: Settings) => {
  const term = termMs(s);
  return Math.max(60_000, Math.floor(s.graceMinutes != null ? Math.min(s.graceMinutes * 60_000, term / 4) : term / 20));
};

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

/** How strong a cycle day is, 0 to 1 (1 on the peak day): for shading. */
export const strength = (day: number) => chance(day, 1);

const DAY_MIN = 1440;
const OFF_DAY = 4;

/** Set hours: where a time of day (minutes after local midnight) falls on the cycle curve (fertility.rs `hours_day`). */
export function hoursDay(minute: number, h: FertileHours): number {
  const start = h.startMin % DAY_MIN;
  const mod = (x: number) => ((x % DAY_MIN) + DAY_MIN) % DAY_MIN;
  const len = mod(h.endMin - start) || DAY_MIN;
  const into = mod(minute - start);
  return into < len ? WINDOW.start + (into / len) * (WINDOW.end - WINDOW.start + 1) : OFF_DAY;
}

/** Minutes the set-hours window lasts (the same start and end is all day). */
export const hoursLen = (h: FertileHours) => ((((h.endMin - h.startMin) % DAY_MIN) + DAY_MIN) % DAY_MIN) || DAY_MIN;

const minuteOf = (t: number) => {
  const d = new Date(t);
  return d.getHours() * 60 + d.getMinutes() + d.getSeconds() / 60;
};

/** The cycle day (or, with set hours, where the time of day falls) at `now` (fertility.rs `day_now`). */
export function dayNow(s: Settings, now: number): number {
  return s.fertileHours ? hoursDay(minuteOf(now), s.fertileHours) : cycleDay(now, s.cycleAnchorMs ?? now, cycleMs(s));
}

/** Real time until the chance is at least "high" (0: it is now), cycle or set hours. */
export function untilFertileNow(s: Settings, now: number): number {
  const h = s.fertileHours;
  if (!h) return untilFertile(now, s.cycleAnchorMs ?? now, cycleMs(s), s.fertilityPeak);
  const good = (t: number) => ["high", "peak"].includes(level(chance(hoursDay(minuteOf(t), h), 1), 1));
  if (good(now)) return 0;
  for (let m = 1; m <= DAY_MIN; m++) {
    const t = Math.floor(now / 60_000) * 60_000 + m * 60_000;
    if (good(t)) return t - now;
  }
  return 0;
}

/** Real time until the chance drops below "high" again (set hours). */
export function untilWindowEnds(h: FertileHours, now: number): number {
  const good = (t: number) => ["high", "peak"].includes(level(chance(hoursDay(minuteOf(t), h), 1), 1));
  for (let m = 1; m <= DAY_MIN; m++) {
    const t = Math.floor(now / 60_000) * 60_000 + m * 60_000;
    if (!good(t)) return t - now;
  }
  return 0;
}

/** "20:00" from minutes after midnight. */
export const hhmm = (m: number) => `${String(Math.floor(m / 60) % 24).padStart(2, "0")}:${String(m % 60).padStart(2, "0")}`;

/** Pieces of a ring (0 to 1 from the top, clockwise) shaded by how fertile they are. */
export interface Shade {
  from: number;
  to: number;
  strength: number;
}

/** The cycle ring: one piece per fertile-ish day. */
export function cycleShades(): Shade[] {
  const out: Shade[] = [];
  for (let d = WINDOW.start - 1; d <= WINDOW.end; d++) out.push({ from: (d - 1) / CYCLE_DAYS, to: d / CYCLE_DAYS, strength: strength(d) });
  return out;
}

/** The 24-hour ring (midnight at the top): the window in slices. */
export function hourShades(h: FertileHours, slices = 24): Shade[] {
  const len = hoursLen(h);
  return Array.from({ length: slices }, (_, i) => {
    const m = h.startMin + ((i + 0.5) / slices) * len;
    return { from: (h.startMin + (i / slices) * len) / DAY_MIN, to: (h.startMin + ((i + 1) / slices) * len) / DAY_MIN, strength: strength(hoursDay(m, h)) };
  });
}

export const MULTIPLES_LABEL = {
  realistic: { name: "Realistic", hint: "Twins about 3%" },
  playful: { name: "Playful", hint: "Twins 10%, triplets 2%" },
  lots: { name: "Lots of babies", hint: "Twins 25%, triplets 8%, quads 2%" },
} as const;

/** The most babies a pregnancy can have (pregsys-core `fertility::MAX_BABIES`). */
export const MAX_BABIES = 8;

export const SPREAD_LABEL: Record<LitterSpread, string> = { little: "A little", some: "Some", lots: "A lot" };

/** The chance of each litter size, 1 to 8 (index 0 is 1 baby): a bell around the usual size (fertility.rs `litter_odds`). */
export function litterOdds(l: Litter): number[] {
  const usual = Math.min(MAX_BABIES, Math.max(1, Math.round(l.usual)));
  const sd = l.spread === "little" ? 0.5 : l.spread === "lots" ? 1.5 : 0.9;
  const w = Array.from({ length: MAX_BABIES }, (_, i) => Math.exp(-((i + 1 - usual) ** 2) / (2 * sd * sd)));
  const total = w.reduce((a, b) => a + b, 0);
  return w.map((x) => x / total);
}

/** A litter's size from a random number in [0, 1) (fertility.rs `roll_litter`). */
export function rollLitter(l: Litter, roll: number): number {
  let acc = 0;
  const odds = litterOdds(l);
  for (let i = 0; i < odds.length; i++) {
    acc += odds[i];
    if (roll < acc) return i + 1;
  }
  return MAX_BABIES;
}

/** "3 · 24%  4 · 44%  5 · 24%": the likely litter sizes (2% and up). */
export function litterOddsText(l: Litter): string {
  return litterOdds(l)
    .map((p, i) => ({ n: i + 1, p }))
    .filter((x) => x.p >= 0.02)
    .map((x) => `${x.n}: ${Math.round(x.p * 100)}%`)
    .join(" · ");
}

export function babyWord(n: number): string {
  return n === 1 ? "One baby" : n === 2 ? "Twins!" : n === 3 ? "Triplets!" : n === 4 ? "Quadruplets!" : `${n} babies!`;
}

/** "Litter of 6!" for a litter-mode pregnancy (one is still "One baby"). */
export function litterWord(n: number): string {
  return n === 1 ? "One baby" : `Litter of ${n}!`;
}

/** An egg pregnancy (the avatar's Eggs switch, pregsys-core `Pregnancy::eggs`). */
export const isEggs = (p: { kind?: string | null }) => p.kind === "egg";

/** Laid eggs that haven't hatched yet (pregsys-core `Pregnancy::incubating`). */
export const isIncubating = (p: { kind?: string | null; born: boolean; hatchedMs?: number | null }) => p.born && isEggs(p) && p.hatchedMs == null;

export function eggWord(n: number): string {
  return n === 1 ? "One egg" : `${n} eggs!`;
}

/** A pregnancy rolled in litter mode (pregsys-core `Pregnancy::litter`). */
export const isLitter = (p: { litter?: number | null }) => p.litter != null;

/** babyWord, litterWord for a litter, or eggWord for an egg pregnancy (eggs win). */
export function kidWord(p: { kind?: string | null; litter?: number | null }, n: number): string {
  return isEggs(p) ? eggWord(n) : isLitter(p) ? litterWord(n) : babyWord(n);
}

/** "Twins born", "Litter of 6 born", "One baby born", "2 eggs laid", "Egg laid" (no "!"). */
export function birthTitle(p: { kind?: string | null; babyCount: number; litter?: number | null }): string {
  if (isEggs(p)) return p.babyCount > 1 ? `${p.babyCount} eggs laid` : "Egg laid";
  return `${kidWord(p, p.babyCount).replace("!", "")} born`;
}

/** One emoji each, or counts past 4: "👧×4 👦×2". */
export function genderEmoji(genders: string[]): string {
  if (genders.length <= 4) return genders.map((g) => (g === "girl" ? "👧" : "👦")).join("");
  const girls = genders.filter((g) => g === "girl").length;
  const boys = genders.length - girls;
  return [girls ? `👧×${girls}` : "", boys ? `👦×${boys}` : ""].filter(Boolean).join(" ");
}

/** "A girl!", "A girl and a boy!", "2 girls and a boy!" */
export function genderText(genders: string[]): string {
  const girls = genders.filter((g) => g === "girl").length;
  const boys = genders.length - girls;
  const part = (n: number, word: string) => (n === 0 ? "" : n === 1 ? `a ${word}` : `${n} ${word}s`);
  const text = [part(girls, "girl"), part(boys, "boy")].filter(Boolean).join(" and ");
  return text ? text.charAt(0).toUpperCase() + text.slice(1) + "!" : "";
}
