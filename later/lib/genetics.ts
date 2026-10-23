// Seeded, deterministic genetics. Nothing random is stored: results are
// recomputed from (conception time, both user IDs, baby index), so they
// survive reloads and twins can differ. Manual choices and locks always win.

import type { AppData, Baby, Pregnancy, SpeciesBlend } from "./types";
import { mixColors, species, SPECIES, type SpeciesDef, type TraitKey, TRAIT_KEYS } from "./species";

// ---------- seeded random ----------

/** FNV-1a 32-bit hash. */
export function hash(s: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

/** Small fast PRNG (mulberry32). */
export function rng(seed: string): () => number {
  let a = hash(seed);
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function pick<T>(r: () => number, list: T[]): T {
  return list[Math.floor(r() * list.length) % list.length];
}

// ---------- parents ----------

export interface ParentInfo {
  role: "A" | "B";
  label: string;
  species: string;
  /** Where the species came from (B only). */
  source: "self" | "partner" | "fallback" | "manual" | "guess";
  furColor: string;
  eyeColor: string;
  seedId: string;
}

export interface GeneticsContext {
  userId: string;
  userName: string;
  hybridChance: number;
}

export function contextFrom(data: AppData): GeneticsContext {
  const local = data.instance?.local;
  return {
    userId: local?.userId ?? local?.name ?? "me",
    userName: local?.name ?? "You",
    hybridChance: data.settings.hybridChance,
  };
}

function parentColors(sp: SpeciesDef, seedId: string) {
  const r = rng(`colors|${sp.id}|${seedId}`);
  return { furColor: pick(r, sp.furColors), eyeColor: pick(r, sp.eyeColors) };
}

export function parents(p: Pregnancy, ctx: GeneticsContext): [ParentInfo, ParentInfo] {
  const aSp = species(p.motherSpecies);
  const a: ParentInfo = {
    role: "A",
    label: ctx.userName,
    species: aSp.id,
    source: "self",
    seedId: ctx.userId,
    ...parentColors(aSp, ctx.userId),
  };
  const known = p.partner.species && p.partner.speciesSource !== "fallback";
  const bId = known ? p.partner.species! : aSp.id;
  const bSeed = p.partner.userId ?? p.partner.name ?? "unknown";
  const bSp = species(bId);
  const b: ParentInfo = {
    role: "B",
    label: p.partner.name ?? "Unknown parent",
    species: bSp.id,
    source: !known ? "fallback" : p.partner.speciesSource === "manual" ? "manual" : p.partner.speciesSource === "guess" ? "guess" : "partner",
    seedId: bSeed,
    ...parentColors(bSp, bSeed),
  };
  return [a, b];
}

// ---------- babies ----------

export type TraitFrom = "A" | "B" | "blend" | "locked" | "chance";

export interface TraitResult {
  key: TraitKey;
  value: string;
  from: TraitFrom;
  hint: "dominant" | "recessive" | "blend" | "even" | null;
  /** Species whose version of the trait this is. */
  speciesId: string | null;
}

export interface BabyResult {
  index: number;
  blend: SpeciesBlend;
  hybrid: boolean;
  manualSpecies: boolean;
  /** Main species (largest share). */
  mainSpecies: string;
  traits: Record<TraitKey, TraitResult>;
}

export function babySeed(p: Pregnancy, a: ParentInfo, b: ParentInfo, index: number): string {
  return `${p.createdMs}|${a.seedId}|${b.seedId}|${index}`;
}

/** The species roll: pure A, pure B, or a rare hybrid. */
export function rollSpecies(seed: string, aSp: string, bSp: string, hybridChance: number): SpeciesBlend {
  if (aSp === bSp) return { parts: { [aSp]: 1 } };
  const r = rng(`${seed}|species`);
  if (r() < hybridChance) {
    const w = Math.round((0.3 + 0.4 * r()) * 100) / 100;
    return { parts: { [aSp]: w, [bSp]: Math.round((1 - w) * 100) / 100 } };
  }
  return { parts: { [r() < 0.5 ? aSp : bSp]: 1 } };
}

export function mainSpeciesOf(blend: SpeciesBlend): string {
  return Object.entries(blend.parts).sort((x, y) => y[1] - x[1])[0]?.[0] ?? "human";
}

function dom(sp: SpeciesDef, key: TraitKey): number {
  return sp.dominance[key] ?? 0.5;
}

function shapeValue(sp: SpeciesDef, key: TraitKey): string {
  return (sp as unknown as Record<string, string>)[key];
}

export function computeBaby(p: Pregnancy, baby: Baby, ctx: GeneticsContext): BabyResult {
  const [a, b] = parents(p, ctx);
  const seed = babySeed(p, a, b, baby.index);
  const manual = !!baby.manualSpecies && Object.keys(baby.manualSpecies.parts).length > 0;
  const blend = manual ? normalize(baby.manualSpecies!) : rollSpecies(seed, a.species, b.species, ctx.hybridChance);
  const entries = Object.entries(blend.parts);
  const hybrid = entries.length > 1;
  const aSp = species(a.species);
  const bSp = species(b.species);

  const traits = {} as Record<TraitKey, TraitResult>;
  for (const key of TRAIT_KEYS) {
    const locked = baby.locks[key];
    if (locked != null) {
      traits[key] = { key, value: locked, from: "locked", hint: null, speciesId: null };
      continue;
    }
    const r = rng(`${seed}|${key}|${baby.reroll}`);

    if (key === "sex") {
      traits[key] = { key, value: r() < 0.5 ? "girl" : "boy", from: "chance", hint: null, speciesId: null };
      continue;
    }

    if (key === "furColor" || key === "eyeColor") {
      const ca = a[key], cb = b[key];
      if (ca === cb) {
        traits[key] = { key, value: ca, from: "A", hint: "even", speciesId: a.species };
      } else if (r() < 0.35) {
        traits[key] = { key, value: mixColors(ca, cb, 0.3 + 0.4 * r()), from: "blend", hint: "blend", speciesId: null };
      } else {
        const da = dom(aSp, key), db = dom(bSp, key);
        const fromA = r() < da / (da + db);
        traits[key] = {
          key,
          value: fromA ? ca : cb,
          from: fromA ? "A" : "B",
          hint: da === db ? "even" : (fromA ? da > db : db > da) ? "dominant" : "recessive",
          speciesId: fromA ? a.species : b.species,
        };
      }
      continue;
    }

    // Shape and choice traits come from one species in the blend,
    // weighted by its share and how dominant its version is.
    const weights = entries.map(([id, share]) => share * dom(species(id), key));
    const total = weights.reduce((x, y) => x + y, 0) || 1;
    let roll = r() * total;
    let chosen = entries[0][0];
    for (let i = 0; i < entries.length; i++) {
      roll -= weights[i];
      if (roll <= 0) {
        chosen = entries[i][0];
        break;
      }
    }
    const sp = species(chosen);
    const others = entries.filter(([id]) => id !== chosen).map(([id]) => dom(species(id), key));
    const myDom = dom(sp, key);
    const hint = !hybrid || others.length === 0 ? null : others.every((d) => myDom > d) ? "dominant" : others.every((d) => myDom < d) ? "recessive" : "even";
    const from: TraitFrom = chosen === a.species ? "A" : chosen === b.species ? "B" : "chance";
    traits[key] = { key, value: shapeValue(sp, key), from, hint, speciesId: chosen };
  }

  return { index: baby.index, blend, hybrid, manualSpecies: manual, mainSpecies: mainSpeciesOf(blend), traits };
}

export function normalize(blend: SpeciesBlend): SpeciesBlend {
  const entries = Object.entries(blend.parts).filter(([, w]) => w > 0);
  const total = entries.reduce((s, [, w]) => s + w, 0) || 1;
  return { parts: Object.fromEntries(entries.map(([k, w]) => [k, Math.round((w / total) * 100) / 100])) };
}

export function blendLabel(blend: SpeciesBlend): string {
  const e = Object.entries(blend.parts).sort((x, y) => y[1] - x[1]);
  if (e.length === 1) return species(e[0][0]).name;
  return e.map(([id, w]) => `${Math.round(w * 100)}% ${species(id).name}`).join(" / ");
}

// ---------- species guessing from avatar names ----------

export interface SpeciesGuess {
  species: string;
  confidence: number;
  source: "answer" | "community" | "keyword";
}

/** Below this, the UI asks "Looks like a fox, correct?". */
export const CONFIDENT = 0.8;

export function guessSpecies(
  avatarName: string | null | undefined,
  answers: Record<string, string> = {},
  community: Record<string, string> = {},
  userId?: string | null,
): SpeciesGuess | null {
  if (userId && answers[userId]) return { species: answers[userId], confidence: 1, source: "answer" };
  if (!avatarName) return null;
  const key = avatarName.trim().toLowerCase();
  if (answers[key]) return { species: answers[key], confidence: 1, source: "answer" };
  if (community[key]) return { species: community[key], confidence: 0.85, source: "community" };

  const words = key.split(/[^a-z0-9]+/).filter(Boolean);
  const hits: { id: string; score: number }[] = [];
  for (const sp of SPECIES) {
    let best = 0;
    for (const kw of sp.keywords) {
      if (words.includes(kw)) best = Math.max(best, 0.9);
      else if (kw.length >= 3 && key.includes(kw)) best = Math.max(best, 0.65);
    }
    if (best > 0) hits.push({ id: sp.id, score: best });
  }
  // "Foxgirl", "Anime Wolf": an animal word beats generic human words.
  if (hits.some((h) => h.id !== "human")) hits.splice(0, hits.length, ...hits.filter((h) => h.id !== "human"));
  if (hits.length === 0) return null;
  hits.sort((x, y) => y.score - x.score);
  // Two species tied = not sure.
  const tied = hits.length > 1 && hits[1].score === hits[0].score;
  return { species: hits[0].id, confidence: tied ? 0.45 : hits[0].score, source: "keyword" };
}
