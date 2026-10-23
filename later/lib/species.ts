// Species definitions. The `index` is what the optional OSC `species`
// parameter sends: never change an existing number, only add new ones.

export type TraitKey = "sex" | "ears" | "tail" | "muzzle" | "covering" | "wings" | "pattern" | "furColor" | "eyeColor";

export interface TraitInfo {
  key: TraitKey;
  label: string;
  /** "color" traits can blend between parents. */
  kind: "shape" | "color" | "choice";
  options?: string[];
}

export const TRAITS: TraitInfo[] = [
  { key: "sex", label: "Sex", kind: "choice", options: ["girl", "boy"] },
  { key: "ears", label: "Ears", kind: "shape" },
  { key: "tail", label: "Tail", kind: "shape" },
  { key: "muzzle", label: "Face", kind: "choice", options: ["flat", "short", "snout", "beak"] },
  { key: "covering", label: "Coat", kind: "choice", options: ["skin", "fur", "scales", "feathers"] },
  { key: "wings", label: "Wings", kind: "choice", options: ["none", "feathered", "leathery"] },
  { key: "pattern", label: "Pattern", kind: "choice", options: ["plain", "spots", "stripes", "socks", "mask", "belly"] },
  { key: "furColor", label: "Color", kind: "color" },
  { key: "eyeColor", label: "Eye color", kind: "color" },
];

export const TRAIT_KEYS = TRAITS.map((t) => t.key);

export interface SpeciesDef {
  id: string;
  name: string;
  index: number;
  ears: string;
  tail: string;
  muzzle: string;
  covering: string;
  wings: string;
  pattern: string;
  furColors: string[];
  eyeColors: string[];
  /** 0..1 per trait; higher wins more often against the other parent. */
  dominance: Partial<Record<TraitKey, number>>;
  /** Avatar-name words that suggest this species. */
  keywords: string[];
}

const skin = ["#f6d7c3", "#eac0a2", "#d9a27e", "#b67b56", "#8d5a3b", "#5e3b28"];
const eyesWarm = ["#7a4a2a", "#3d6b4f", "#4a7bb7", "#8a6b2a", "#5b3f8c"];

export const SPECIES: SpeciesDef[] = [
  {
    id: "human", name: "Human", index: 1, ears: "human", tail: "none", muzzle: "flat", covering: "skin", wings: "none", pattern: "plain",
    furColors: skin, eyeColors: ["#6b4426", "#3f6e8c", "#4f7a4a", "#8a8f99", "#2f2a26"],
    dominance: { ears: 0.35, tail: 0.3, muzzle: 0.45, covering: 0.35, wings: 0.5 },
    keywords: ["human", "manuka", "karin", "rusk", "kikyo", "selestia", "anime", "girl", "boy", "maid"],
  },
  {
    id: "fox", name: "Fox", index: 2, ears: "fox", tail: "fox", muzzle: "snout", covering: "fur", wings: "none", pattern: "socks",
    furColors: ["#e8833a", "#f2a65a", "#c9622a", "#f4e6d0", "#8a8f99"], eyeColors: ["#d49a2a", "#7a4a2a", "#3d6b4f"],
    dominance: { ears: 0.6, tail: 0.65, furColor: 0.6 },
    keywords: ["fox", "kitsune", "vulpine", "fennec", "vixen", "foxxo", "todd"],
  },
  {
    id: "wolf", name: "Wolf", index: 3, ears: "wolf", tail: "wolf", muzzle: "snout", covering: "fur", wings: "none", pattern: "mask",
    furColors: ["#8a8f99", "#5b5f66", "#d8d4cc", "#3a3a40", "#a67c52"], eyeColors: ["#d4b13a", "#4a7bb7", "#7a4a2a"],
    dominance: { ears: 0.6, tail: 0.6, muzzle: 0.6 },
    keywords: ["wolf", "lupine", "husky", "dog", "canine", "pup", "doggo", "rexouium", "shiba", "corgi", "malamute"],
  },
  {
    id: "cat", name: "Cat", index: 4, ears: "cat", tail: "cat", muzzle: "short", covering: "fur", wings: "none", pattern: "stripes",
    furColors: ["#f2a65a", "#3a3a40", "#f4f1ea", "#8a8f99", "#c9a37a"], eyeColors: ["#7fbf5a", "#d4b13a", "#4a7bb7"],
    dominance: { ears: 0.55, pattern: 0.6, eyeColor: 0.6 },
    keywords: ["cat", "kitty", "kitten", "feline", "neko", "lion", "tiger", "leopard", "lynx", "panther", "caracal", "snep"],
  },
  {
    id: "bunny", name: "Bunny", index: 5, ears: "bunny", tail: "bunny", muzzle: "short", covering: "fur", wings: "none", pattern: "plain",
    furColors: ["#f4f1ea", "#c9a37a", "#8a6b52", "#d8d4cc", "#3a3a40"], eyeColors: ["#7a4a2a", "#c2405f", "#3a3a40"],
    dominance: { ears: 0.65, tail: 0.4 },
    keywords: ["bunny", "rabbit", "hare", "lop", "bun", "usagi"],
  },
  {
    id: "dragon", name: "Dragon", index: 6, ears: "dragon", tail: "dragon", muzzle: "snout", covering: "scales", wings: "leathery", pattern: "belly",
    furColors: ["#6fb07f", "#5a8fd1", "#c2405f", "#9b6fd1", "#3a3a40", "#e3b54a"], eyeColors: ["#e3b54a", "#c2405f", "#5ad1c4"],
    dominance: { wings: 0.4, covering: 0.55, tail: 0.6, eyeColor: 0.65 },
    keywords: ["dragon", "drake", "wyvern", "derg", "dergen", "nardoragon", "wyrm", "drakon"],
  },
  {
    id: "deer", name: "Deer", index: 7, ears: "deer", tail: "deer", muzzle: "snout", covering: "fur", wings: "none", pattern: "spots",
    furColors: ["#b07a4a", "#c9a37a", "#8a5a3b", "#e8d4b8"], eyeColors: ["#3a2a20", "#7a4a2a"],
    dominance: { pattern: 0.65, ears: 0.5 },
    keywords: ["deer", "fawn", "stag", "doe", "reindeer", "elk", "moose"],
  },
  {
    id: "bird", name: "Bird", index: 8, ears: "bird", tail: "bird", muzzle: "beak", covering: "feathers", wings: "feathered", pattern: "mask",
    furColors: ["#5a8fd1", "#e3b54a", "#c2405f", "#6fb07f", "#3a3a40", "#f4f1ea"], eyeColors: ["#3a2a20", "#e3b54a"],
    dominance: { wings: 0.55, muzzle: 0.5, covering: 0.45 },
    keywords: ["bird", "avian", "avali", "owl", "raven", "crow", "parrot", "gryphon", "griffin", "birb", "phoenix"],
  },
  {
    id: "shark", name: "Shark", index: 9, ears: "shark", tail: "shark", muzzle: "snout", covering: "skin", wings: "none", pattern: "belly",
    furColors: ["#7d93a8", "#5b6f85", "#a9bccc", "#3f4e5e"], eyeColors: ["#2a2a30", "#5ad1c4"],
    dominance: { tail: 0.6, pattern: 0.55 },
    keywords: ["shark", "orca", "fish", "aquatic", "sergal"],
  },
  {
    id: "lizard", name: "Lizard", index: 10, ears: "lizard", tail: "lizard", muzzle: "snout", covering: "scales", wings: "none", pattern: "spots",
    furColors: ["#7fbf5a", "#c9a37a", "#e3b54a", "#5ad1c4", "#9b6fd1"], eyeColors: ["#e3b54a", "#c2405f"],
    dominance: { covering: 0.55, tail: 0.55 },
    keywords: ["lizard", "kobold", "gecko", "reptile", "snake", "naga", "iguana", "chameleon"],
  },
  {
    id: "bear", name: "Bear", index: 11, ears: "bear", tail: "bear", muzzle: "snout", covering: "fur", wings: "none", pattern: "plain",
    furColors: ["#7a5a3b", "#3a3a40", "#f4f1ea", "#a67c52"], eyeColors: ["#3a2a20", "#7a4a2a"],
    dominance: { ears: 0.45, muzzle: 0.55 },
    keywords: ["bear", "panda", "teddy", "grizzly", "polar"],
  },
  {
    id: "mouse", name: "Mouse", index: 12, ears: "mouse", tail: "mouse", muzzle: "short", covering: "fur", wings: "none", pattern: "plain",
    furColors: ["#a39a92", "#f4f1ea", "#c9a37a", "#5b5f66"], eyeColors: ["#2a2a30", "#c2405f"],
    dominance: { ears: 0.5 },
    keywords: ["mouse", "rat", "rodent", "mousey", "hamster", "squeak"],
  },
  {
    id: "horse", name: "Horse", index: 13, ears: "horse", tail: "horse", muzzle: "snout", covering: "fur", wings: "none", pattern: "socks",
    furColors: ["#8a5a3b", "#3a3a40", "#f4f1ea", "#c9a37a", "#b07a4a"], eyeColors: ["#3a2a20", "#4a7bb7"],
    dominance: { tail: 0.55, pattern: 0.5 },
    keywords: ["horse", "pony", "equine", "unicorn", "pegasus", "zebra"],
  },
  {
    id: "bat", name: "Bat", index: 14, ears: "bat", tail: "none", muzzle: "short", covering: "fur", wings: "leathery", pattern: "plain",
    furColors: ["#3a3a40", "#5b4a5e", "#8a6b52", "#2f2a26"], eyeColors: ["#c2405f", "#e3b54a"],
    dominance: { ears: 0.6, wings: 0.45 },
    keywords: ["bat", "batty", "fruitbat", "chiroptera"],
  },
];

const BY_ID = new Map(SPECIES.map((s) => [s.id, s]));

export function species(id: string | null | undefined): SpeciesDef {
  return (id && BY_ID.get(id)) || BY_ID.get("human")!;
}

export function speciesName(id: string | null | undefined): string {
  return id ? (BY_ID.get(id)?.name ?? id) : "Unknown";
}

export function isSpecies(id: string | null | undefined): boolean {
  return !!id && BY_ID.has(id);
}

/** Display text for a trait value, e.g. "Fox ears", "Feathered wings". */
export function traitLabel(key: TraitKey, value: string): string {
  switch (key) {
    case "ears":
      return value === "human" ? "Round ears" : `${speciesName(value)} ears`;
    case "tail":
      return value === "none" ? "No tail" : `${speciesName(value)} tail`;
    case "wings":
      return value === "none" ? "No wings" : `${cap(value)} wings`;
    case "muzzle":
      return { flat: "Flat face", short: "Button nose", snout: "Little snout", beak: "Tiny beak" }[value] ?? value;
    case "covering":
      return { skin: "Smooth skin", fur: "Soft fur", scales: "Shiny scales", feathers: "Fluffy feathers" }[value] ?? value;
    case "pattern":
      return { plain: "Plain", spots: "Spots", stripes: "Stripes", socks: "Socks", mask: "Face mask", belly: "Light belly" }[value] ?? value;
    case "sex":
      return value === "girl" ? "Girl" : value === "boy" ? "Boy" : value;
    case "furColor":
    case "eyeColor":
      return colorName(value);
  }
}

/** All values a trait can take (for pickers). */
export function traitOptions(key: TraitKey): string[] {
  const info = TRAITS.find((t) => t.key === key)!;
  if (info.options) return info.options;
  if (key === "ears") return [...new Set(SPECIES.map((s) => s.ears))];
  if (key === "tail") return [...new Set(SPECIES.map((s) => s.tail))];
  if (key === "furColor") return [...new Set(SPECIES.flatMap((s) => s.furColors))];
  return [...new Set(SPECIES.flatMap((s) => s.eyeColors))];
}

function cap(s: string) {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

/** Rough friendly name for a hex color. */
export function colorName(hex: string): string {
  const [r, g, b] = hexToRgb(hex);
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  const l = (max + min) / 2 / 255;
  const s = max === min ? 0 : (max - min) / 255 / (1 - Math.abs(2 * l - 1) || 1);
  if (s < 0.15) return l > 0.85 ? "Snow white" : l > 0.6 ? "Silver" : l > 0.3 ? "Grey" : "Charcoal";
  let h = 0;
  if (max === r) h = ((g - b) / (max - min)) % 6;
  else if (max === g) h = (b - r) / (max - min) + 2;
  else h = (r - g) / (max - min) + 4;
  h = (h * 60 + 360) % 360;
  const tone = l > 0.7 ? "Light " : l < 0.3 ? "Deep " : "";
  const name = h < 15 ? "red" : h < 40 ? "orange" : h < 55 ? "amber" : h < 70 ? "gold" : h < 160 ? "green" : h < 200 ? "teal" : h < 250 ? "blue" : h < 290 ? "purple" : h < 340 ? "pink" : "red";
  if ((name === "orange" || name === "amber") && s < 0.5) return `${tone}brown`.trim().replace(/^./, (c) => c.toUpperCase());
  return (tone + name).replace(/^./, (c) => c.toUpperCase());
}

export function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace("#", "");
  const n = parseInt(h.length === 3 ? h.split("").map((c) => c + c).join("") : h, 16) || 0;
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

export function mixColors(a: string, b: string, t: number): string {
  const [r1, g1, b1] = hexToRgb(a);
  const [r2, g2, b2] = hexToRgb(b);
  const m = (x: number, y: number) => Math.round(x + (y - x) * t).toString(16).padStart(2, "0");
  return `#${m(r1, r2)}${m(g1, g2)}${m(b1, b2)}`;
}
