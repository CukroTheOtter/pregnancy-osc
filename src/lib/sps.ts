// SPS socket helpers, mirroring crates/pregsys-core/src/sps.rs.

import type { AvatarProfile, HoleType, Settings } from "./types";

const PREFIX = "OGB/Orf/";
const OTHERS = ["PenOthersNewRoot", "PenOthers", "PenOthersNewTip"];
/** SPS3 (VRCFury 1.1442+): `OGB/Socket/<socket>/DepthOthers`. */
const V3_PREFIX = "OGB/Socket/";
const V3_CHANNEL = "DepthOthers";
/** Newest `OGB/Version/<n>` this app understands (mirrors KNOWN_VERSION in sps.rs). */
export const KNOWN_SPS_VERSION = 11;
const VERSION_PREFIX = "OGB/Version/";

/** The avatar's SPS haptics version, if it says (older avatars don't). */
export function spsVersion(params: string[]): number | null {
  const found = params.filter((p) => p.startsWith(VERSION_PREFIX)).map((p) => Number(p.slice(VERSION_PREFIX.length))).filter(Number.isInteger);
  return found.length ? Math.max(...found) : null;
}

/** True when the avatar's SPS is newer than this app knows. */
export function spsTooNew(params: string[]): boolean {
  return (spsVersion(params) ?? 0) > KNOWN_SPS_VERSION;
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
