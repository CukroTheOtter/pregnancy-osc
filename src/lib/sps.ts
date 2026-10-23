// SPS socket helpers, mirroring crates/pregsys-core/src/sps.rs.

import type { AvatarProfile, HoleType, Settings } from "./types";

const PREFIX = "OGB/Orf/";
const OTHERS = ["PenOthersNewRoot", "PenOthers", "PenOthersNewTip"];
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
    if (!p.startsWith(PREFIX)) continue;
    const [socket, channel] = p.slice(PREFIX.length).split("/");
    if (socket && OTHERS.includes(channel)) out.add(socket);
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
