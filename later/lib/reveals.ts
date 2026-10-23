// Ultrasound reveal schedule, milestones and "size of a ..." per week.
// Everything is in pregnancy weeks, so it scales with accelerated timelines.

import type { Pregnancy, SpoilerMode } from "./types";
import type { TraitKey } from "./species";
import { week } from "./timeline";

export interface RevealDef {
  key: string;
  week: number;
  traits: TraitKey[];
  label: string;
  hint: string;
}

export const REVEALS: RevealDef[] = [
  { key: "sex", week: 18, traits: ["sex"], label: "Boy or girl?", hint: "Something about who they are" },
  { key: "ears", week: 20, traits: ["ears", "tail"], label: "Ears and tail", hint: "Something on top, something behind" },
  { key: "coat", week: 22, traits: ["covering", "wings"], label: "Coat and wings", hint: "How soft they'll be" },
  { key: "pattern", week: 24, traits: ["pattern", "furColor"], label: "Pattern and color", hint: "Their colors" },
  { key: "face", week: 27, traits: ["muzzle"], label: "Little face", hint: "A peek at their face" },
  { key: "eyes", week: 32, traits: ["eyeColor"], label: "Eye color", hint: "Those eyes" },
  { key: "species", week: 36, traits: [], label: "Species surprise", hint: "Something rare, maybe..." },
];

export function revealFor(trait: TraitKey): RevealDef | undefined {
  return REVEALS.find((r) => r.traits.includes(trait));
}

/** Reveals whose week has come but haven't been seen yet (missed ones included). */
export function dueReveals(p: Pregnancy, now: number): RevealDef[] {
  if (p.endedMs) return [];
  const w = week(p.timeline, now);
  return REVEALS.filter((r) => w >= r.week && p.reveals[r.key] == null);
}

export function nextReveal(p: Pregnancy, now: number): RevealDef | undefined {
  const w = week(p.timeline, now);
  return REVEALS.find((r) => r.week > w);
}

export function isRevealed(p: Pregnancy, key: string, spoiler: SpoilerMode): boolean {
  return spoiler === "all" || p.reveals[key] != null;
}

export function isTraitVisible(p: Pregnancy, trait: TraitKey, spoiler: SpoilerMode, locked: boolean): boolean {
  if (locked || spoiler === "all") return true;
  const r = revealFor(trait);
  return !!r && p.reveals[r.key] != null;
}

export interface Milestone {
  key: string;
  week: number;
  title: string;
  text: string;
}

export const MILESTONES: Milestone[] = [
  { key: "w4", week: 4, title: "Week 4", text: "It's official: a tiny poppy seed is on the way!" },
  { key: "w8", week: 8, title: "Week 8", text: "A little raspberry with a tiny heartbeat." },
  { key: "tri2", week: 13, title: "Second trimester!", text: "Feeling a bit more like yourself again." },
  { key: "w16", week: 16, title: "First flutters", text: "You might feel the first tiny wiggles soon." },
  { key: "half", week: 20, title: "Halfway there!", text: "Twenty weeks down. Time for a scan!" },
  { key: "tri3", week: 27, title: "Third trimester!", text: "The home stretch begins." },
  { key: "full", week: 37, title: "Full term", text: "Baby could arrive any day now." },
  { key: "due", week: 40, title: "Due date!", text: "Today's the day (or close to it)." },
  { key: "over", week: 41, title: "Overdue", text: "Still cozy in there. Any day now!" },
];

/** Milestones reached but not announced yet. */
export function newMilestones(p: Pregnancy, now: number): Milestone[] {
  if (p.endedMs) return [];
  const w = week(p.timeline, now);
  return MILESTONES.filter((m) => w >= m.week && !p.milestones.includes(m.key));
}

const SIZES = [
  "a tiny wish", "a tiny wish", "a tiny wish", "a tiny wish",
  "a poppy seed", "a sesame seed", "a lentil", "a blueberry", "a raspberry", "a cherry",
  "a strawberry", "a lime", "a plum", "a peach", "a lemon", "an apple", "an avocado",
  "a pear", "a bell pepper", "a mango", "a banana", "a carrot", "a papaya", "a grapefruit",
  "a cantaloupe", "a cauliflower", "a lettuce", "a cabbage", "an eggplant", "a butternut squash",
  "a cucumber", "a coconut", "a pineapple", "a honeydew melon", "a cantaloupe", "a romaine lettuce",
  "a papaya", "a swiss chard", "a leek", "a mini watermelon", "a pumpkin",
];

export function sizeOf(weekNumber: number, egg = false): string {
  const w = Math.max(0, Math.min(SIZES.length - 1, Math.floor(weekNumber)));
  if (egg) {
    const eggs = ["a speck", "a pea", "a grape", "a quail egg", "a chicken egg", "a duck egg", "a goose egg", "an ostrich egg"];
    return eggs[Math.min(eggs.length - 1, Math.floor((w / 40) * eggs.length))];
  }
  return SIZES[w];
}

export const TRIMESTER_COLORS: Record<1 | 2 | 3, string> = {
  1: "var(--tri1)",
  2: "var(--tri2)",
  3: "var(--tri3)",
};
