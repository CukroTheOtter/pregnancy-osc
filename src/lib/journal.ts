// The pregnancy journal: what happened (worked out from the pregnancy itself)
// plus the notes you write. Scans are drawn by the app (Ultrasound.svelte), so
// an entry only keeps a short "scan:<babies>:<week>:<seed>" tag, never a photo.

import { BABY_REVEAL_WEEK, GENDER_REVEAL_WEEK, babyWord, genderText } from "./fertility";
import { progress, timeAtProgress, TERM_WEEKS } from "./timeline";
import { rng } from "./scanstats";
import { hidesMe, shownName } from "./privacy";
import type { AppData, Pregnancy } from "./types";

/** Rare scans: something cute caught on camera (see Ultrasound.svelte). */
export type ScanVariant = "wave" | "yawn" | "shy" | "hiccups" | "heart" | "highfive";

export interface Scan {
  babies: number;
  week: number;
  seed: number;
  variant?: ScanVariant;
}

export interface JournalItem {
  /** Stable key (a note's id, or the milestone's name). */
  key: string;
  timeMs: number;
  emoji: string;
  title: string;
  text: string;
  scan: Scan | null;
  /** A polaroid instead of a scan (journal moments). */
  card: Card | null;
  /** Your own note (can be deleted). */
  note: boolean;
  /** A rare one (shown with a sparkle). */
  rare?: boolean;
  /** The pregnancy week it belongs to (keeps the order when times are the same). */
  week?: number;
  /** Sorts by this instead of the week when times are the same (the start comes first). */
  order?: number;
}

/** A journal moment's polaroid (MomentCard.svelte). */
export interface Card {
  kind: "partner-kicks" | "well-travelled" | "night-owl";
  seed: number;
  emoji: string;
  caption: string;
  sub?: string;
  /** Whose profile picture goes in it (the partner). */
  person?: { name: string; userId: string | null };
}

/** Rare scans: how each one is written up. */
export const RARE_SCANS: Record<ScanVariant, { emoji: string; title: string; text: string }> = {
  wave: { emoji: "👋", title: "Waved at the camera", text: "A tiny hand, right up at the screen. Hi to you too!" },
  yawn: { emoji: "🥱", title: "Caught a yawn", text: "Big sleepy yawn, right in the middle of the scan." },
  shy: { emoji: "😳", title: "Camera shy", text: "Turned away the whole time. Maybe next scan." },
  hiccups: { emoji: "🫧", title: "Hiccups!", text: "Little hiccups the whole scan. Hic!" },
  heart: { emoji: "💗", title: "A heart on the screen", text: "Something in there looked just like a heart." },
  highfive: { emoji: "🙌", title: "High five!", text: "Caught them touching hands in there." },
};

/** Rare week rolls. */
export const RARE_CHANCE = 1 / 6;
export const CRAVING_CHANCE = 1 / 10;

const FOODS = [
  ["🥒", "pickles"], ["🍦", "ice cream"], ["🍟", "fries"], ["🍫", "chocolate"], ["🍓", "strawberries"], ["🧀", "cheese"],
  ["🌶️", "hot sauce"], ["🍕", "pizza"], ["🥜", "peanut butter"], ["🍉", "watermelon"], ["🍋", "lemons"], ["🥞", "pancakes"],
  ["🍜", "ramen"], ["🍌", "bananas"], ["🍩", "donuts"], ["🌮", "tacos"],
] as const;

/** What happened in a rare week roll (null: nothing this week). */
export function rareRoll(p: Pregnancy, week: number): { scan?: ScanVariant; craving?: [number, number] } | null {
  const r = rng(seedOf(p.id) ^ Math.imul(week + 1, 2654435761));
  const roll = r();
  if (week >= BABY_REVEAL_WEEK) {
    if (roll < RARE_CHANCE) {
      const kinds: ScanVariant[] = ["wave", "yawn", "shy", "hiccups", "heart", ...(p.babyCount > 1 ? (["highfive"] as const) : [])];
      return { scan: kinds[Math.floor(r() * kinds.length)] };
    }
  }
  if (roll >= RARE_CHANCE && roll < RARE_CHANCE + CRAVING_CHANCE && week >= 6) {
    const a = Math.floor(r() * FOODS.length);
    const b = (a + 1 + Math.floor(r() * (FOODS.length - 1))) % FOODS.length;
    return { craving: [a, b] };
  }
  return null;
}

/** A saved journal moment (from how you played) as an item. */
function momentItem(d: AppData, id: string, title: string, timeMs: number, text: string, p: Pregnancy): JournalItem | null {
  const seed = seedOf(id);
  const base = { key: id, timeMs, scan: null, note: false, rare: true };
  if (title === "partner-kicks") {
    const who = shownName(d, p.partner.name, p.partner.userId) ?? "your partner";
    return { ...base, emoji: "💞", title: "Knows their other parent", text: `Kicked and kicked while ${who} was around.`, card: { kind: "partner-kicks", seed, emoji: "💞🦶", caption: `Kicking for ${who}!`, sub: "they know you're here", person: { name: who, userId: p.partner.userId } } };
  }
  if (title === "well-travelled") {
    const n = Number(text) || 4;
    return { ...base, emoji: "🌍", title: "Well-travelled", text: `Kicked in ${n} different worlds in one week.`, card: { kind: "well-travelled", seed, emoji: "🌍✈️", caption: `${n} worlds in a week!`, sub: "a little traveller" } };
  }
  if (title === "night-owl") {
    return { ...base, emoji: "🌙", title: "Night owl", text: "Wide awake after midnight, kicking away.", card: { kind: "night-owl", seed, emoji: "🌙🦉", caption: "3 AM kick party", sub: "somebody's a night owl" } };
  }
  return null;
}

/** Scan weeks: an early one (just a little bean), the count, the big one, and a squeeze near the end. */
export const SCAN_WEEKS = [8, BABY_REVEAL_WEEK, GENDER_REVEAL_WEEK, 36] as const;

/** What a scan's screen says, worked out fresh for every picture: your name, the dates, and genders once they're shown. */
export function scanMeta(d: AppData, p: Pregnancy, timeMs: number, scanWeek: number, showName = true) {
  const genders = d.settings.genderMode === "random" && p.genders.length && scanWeek >= GENDER_REVEAL_WEEK ? p.genders : null;
  return {
    // Privacy mode: no name on the scan screen.
    patient: showName && !hidesMe(d) ? (d.instance.local?.name ?? null) : null,
    timeMs,
    dueMs: timeAtProgress(p.timeline, p.endedMs ?? timeMs, 1),
    genders,
  };
}

export function scanTag(s: Scan): string {
  return `scan:${s.babies}:${Math.floor(s.week)}:${s.seed}${s.variant ? `:${s.variant}` : ""}`;
}

export function parseScan(image: string | null | undefined): Scan | null {
  const m = image?.match(/^scan:(\d+):(\d+):(\d+)(?::(\w+))?$/);
  if (!m) return null;
  const variant = m[4] && m[4] in RARE_SCANS ? (m[4] as ScanVariant) : undefined;
  return { babies: +m[1], week: +m[2], seed: +m[3], ...(variant ? { variant } : {}) };
}

/** A seed that stays the same for one pregnancy (so its scans match). */
export function seedOf(id: string): number {
  let h = 2166136261;
  for (const c of id) h = Math.imul(h ^ c.charCodeAt(0), 16777619);
  return h >>> 0;
}

/** How many babies a scan at this week shows (before the count week: unknown). */
export function shownBabies(p: Pregnancy, week: number): number {
  return week < BABY_REVEAL_WEEK ? 0 : p.babyCount;
}

/** Everything in a pregnancy's journal, newest first. */
export function journalFor(d: AppData, p: Pregnancy, now: number): JournalItem[] {
  const until = p.endedMs ?? now;
  const reached = progress(p.timeline, until) * TERM_WEEKS;
  // Nothing is dated before the pregnancy started: jumping ahead with "Adjust
  // progress" puts the weeks it skipped right at the start, in week order.
  const startedMs = p.source === "conception" ? Math.min(p.timeline.startMs, p.createdMs) : p.createdMs;
  // Scans and cravings wait until you found out (a short pregnancy can pass
  // week 8 before the reveal): the ones before it are dated at the reveal.
  const knownMs = Math.max(startedMs, p.createdMs);
  const at = (week: number) => Math.max(knownMs, Math.min(until, timeAtProgress(p.timeline, until, week / TERM_WEEKS)));
  const weekAt = (t: number) => progress(p.timeline, Math.min(t, until)) * TERM_WEEKS;
  const genders = d.settings.genderMode === "random" && p.genders.length > 0;
  const seed = seedOf(p.id);
  const out: JournalItem[] = [];
  const add = (key: string, timeMs: number, emoji: string, title: string, text = "", scan: Scan | null = null, week = weekAt(timeMs), order?: number) =>
    out.push({ key, timeMs, emoji, title, text, scan, card: null, note: false, week, ...(order != null ? { order } : {}) });

  const partnerName = shownName(d, p.partner.name, p.partner.userId);
  const withPartner = partnerName ? `With ${partnerName}` : "";
  if (p.source === "conception") {
    add("conceived", startedMs, "💕", "A little one on the way", [withPartner, p.worldName ? `in ${p.worldName}` : ""].filter(Boolean).join(" "), null, 0, -2);
    // Its real week (it used to say Week 0), but still first among things at the same time.
    if (p.createdMs > startedMs + 60_000) add("found-out", p.createdMs, "💫", "Found out you're pregnant", "", null, weekAt(p.createdMs), -1);
  } else {
    add("started", p.createdMs, "🤰", "Pregnancy started", withPartner, null, 0, -2);
  }

  for (const w of SCAN_WEEKS) {
    if (reached < w) continue;
    const babies = shownBabies(p, w);
    let text = w === 8 ? "Too early to count, but there's a little bean in there." : babyWord(p.babyCount);
    if (w === GENDER_REVEAL_WEEK && genders) text = `${babyWord(p.babyCount)} ${genderText(p.genders)}`.trim();
    if (w === 36) text = p.babyCount > 1 ? "Getting really snug in there!" : "Running out of room in there!";
    add(`scan-${w}`, at(w), "🩻", `Week ${w} scan`, text, { babies, week: w, seed: seed + w }, w);
  }

  // Rare weeks: a cute scan, a craving. Each week rolls once (the same for this
  // pregnancy every time), half way through the week.
  const skip = new Set<number>(SCAN_WEEKS);
  for (let w = 6; w + 0.5 <= Math.min(reached, TERM_WEEKS - 1); w++) {
    if (skip.has(w)) continue;
    const roll = rareRoll(p, w);
    if (!roll) continue;
    if (roll.scan) {
      const info = RARE_SCANS[roll.scan];
      out.push({ key: `rare-${w}`, week: w + 0.5, timeMs: at(w + 0.5), emoji: info.emoji, title: info.title, text: info.text, rare: true, note: false, card: null, scan: { babies: p.babyCount, week: w, seed: seed + w * 7, variant: roll.scan } });
    } else if (roll.craving) {
      const [a, b] = roll.craving.map((i) => FOODS[i]);
      // No picture: the foods' own emoji and what they are.
      out.push({ key: `craving-${w}`, week: w + 0.5, timeMs: at(w + 0.5), emoji: a[0], title: "Craving of the week", text: [a, b].map(([e, name]) => `${e} ${name[0].toUpperCase()}${name.slice(1)}`).join("\n"), note: false, scan: null, rare: true, card: null });
    }
  }

  const kick = p.kicks[0];
  if (kick) add("first-kick", kick.timeMs, "🦶", "First kick");
  // Only when it came before the end (a birth right on time says enough).
  const dueAt = timeAtProgress(p.timeline, until, 1);
  if (reached >= TERM_WEEKS && dueAt < until - 60_000) add("due", Math.max(startedMs, dueAt), "📅", "Due date", "", null, TERM_WEEKS);
  if (p.labor?.waterBrokeMs) add("water", p.labor.waterBrokeMs, "💧", "Water broke");
  if (p.labor?.startedMs) add("labor", p.labor.startedMs, "⏳", "Labor started");
  if (p.endedMs) {
    if (p.born) add("born", p.endedMs, "🍼", p.babyCount > 1 ? `${babyWord(p.babyCount).replace("!", "")} born!` : "Baby born!", p.labor?.partnerAtBirth && partnerName ? `${partnerName} was there` : "");
    else add("ended", p.endedMs, "🤍", "Ended early");
  }

  // Your notes. The app's own old "event" entries are covered above.
  for (const j of d.journal) {
    if (j.pregnancyId !== p.id) continue;
    if (j.kind === "moment") {
      const m = momentItem(d, j.id, j.title, j.timeMs, j.text, p);
      if (m) out.push(m);
    }
    if (j.kind !== "note") continue;
    out.push({ key: j.id, timeMs: j.timeMs, emoji: "📝", title: j.title && j.title !== "Note" ? j.title : "", text: j.text, scan: parseScan(j.image), card: null, note: true });
  }
  const rank = (x: JournalItem) => x.order ?? x.week ?? weekAt(x.timeMs);
  return out.sort((a, b) => b.timeMs - a.timeMs || rank(b) - rank(a));
}
