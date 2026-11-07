// SPS socket helpers, mirroring crates/pregsys-core/src/sps.rs.

import type { AvatarProfile, HoleType, Settings } from "./types";

const PREFIX = "OGB/Orf/";
const OTHERS = ["PenOthersNewRoot", "PenOthers", "PenOthersNewTip"];
/** SPS3 (VRCFury 1.1442+): `OGB/Socket/<socket>/DepthOthers`. */
const V3_PREFIX = "OGB/Socket/";
const V3_CHANNEL = "DepthOthers";
/** Newest `OGB/Version/<n>` this app understands (mirrors KNOWN_VERSION in sps.rs). */
export const KNOWN_SPS_VERSION = 11;
/** Where the version has lived (`VFH/Version/9`, `/10` before SPS3); it can move again. */
const VERSION_PREFIXES = ["OGB/Version/", "VFH/Version/"];
/** Everything this app knows under `OGB/` (the device types OGB reads, and flags). */
const KNOWN_OGB = ["Socket", "Orf", "Plug", "Pen", "Touch", "Version", "Enabled", "HapticsDisabled"];

/** The avatar's SPS haptics version, if it says (older avatars don't). */
export function spsVersion(params: string[]): number | null {
  const found: number[] = [];
  for (const p of params) {
    const pre = VERSION_PREFIXES.find((x) => p.startsWith(x));
    const n = pre ? Number(p.slice(pre.length)) : NaN;
    if (Number.isInteger(n) && n > 0) found.push(n);
  }
  return found.length ? Math.max(...found) : null;
}

/** An `OGB/...` parameter this app doesn't know: a new SPS format or a renamed one. */
export function unknownOgb(name: string): boolean {
  return name.startsWith("OGB/") && !KNOWN_OGB.includes(name.slice(4).split("/")[0]);
}

/** True when the avatar's SPS looks newer than this app knows: a higher version,
 *  or (in case the version moves again) unknown `OGB/` parameters and no sockets it can read. */
export function spsTooNew(params: string[]): boolean {
  return (spsVersion(params) ?? 0) > KNOWN_SPS_VERSION || (!socketsFrom(params).length && params.some(unknownOgb));
}

/** "a newer SPS (version 12) than this app knows (11)", or without numbers when the version can't be read. */
export function spsNewerText(params: string[]): string {
  const v = spsVersion(params);
  return v && v > KNOWN_SPS_VERSION ? `a newer SPS (version ${v}) than this app knows (${KNOWN_SPS_VERSION})` : "SPS parameters this app doesn't know yet (probably a newer SPS)";
}

/** On avatars uploaded with VRCFury's haptics off. */
export const HAPTICS_DISABLED = "OGB/HapticsDisabled";
const ORAL = ["mouth", "oral", "throat", "blowjob", "lips", "face"];
const ANAL = ["anal", "anus", "ass", "butt", "rear", "booty"];
const VAGINAL = ["vagina", "vag", "pussy", "womb", "cervix", "vulva", "cunt", "front"];

export const HOLE_TYPES: { id: HoleType; label: string; emoji: string; hint: string }[] = [
  { id: "vaginal", label: "Vaginal", emoji: "💗", hint: "vagina, pussy, womb…" },
  { id: "anal", label: "Anal", emoji: "🍑", hint: "anal, ass, butt…" },
  { id: "oral", label: "Oral", emoji: "👄", hint: "mouth, throat…" },
  { id: "other", label: "Other", emoji: "✨", hint: "anything else" },
];

export function classify(socket: string): HoleType {
  const n = socket.toLowerCase();
  const has = (w: string[]) => w.some((x) => n.includes(x));
  return has(ORAL) ? "oral" : has(ANAL) ? "anal" : has(VAGINAL) ? "vaginal" : "other";
}

/** Socket names found among an avatar's parameters. */
export function socketsFrom(params: string[]): string[] {
  const out = new Set<string>();
  for (const p of params) {
    const v3 = p.startsWith(V3_PREFIX);
    if (!v3 && !p.startsWith(PREFIX)) continue;
    const [socket, channel] = p.slice(v3 ? V3_PREFIX.length : PREFIX.length).split("/");
    if (socket && (v3 ? channel === V3_CHANNEL : OTHERS.includes(channel))) out.add(socket);
  }
  return [...out].sort();
}

export function typeOf(a: AvatarProfile | undefined, socket: string): HoleType {
  return a?.sps?.types?.[socket] ?? classify(socket);
}

export function socketCounts(s: Settings, a: AvatarProfile | undefined, socket: string): boolean {
  if (a?.sps?.custom) return a.sps.sockets.includes(socket);
  return s.sps.types[typeOf(a, socket)];
}
