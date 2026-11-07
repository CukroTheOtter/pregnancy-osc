// Privacy mode: for streaming and screenshots. People's names become a stable
// "Hidden #123" (so two partners still look different), their pictures are
// blurred (PersonPic), and your own name becomes "You". Avatar and world names stay.

import type { AppData } from "./types";

type Data = Pick<AppData, "settings"> | null | undefined;

export const isPrivate = (d: Data): boolean => !!d?.settings.privacyMode;

/** A short number from someone's ID or name: the same person, the same number. */
function tag(key: string): number {
  let h = 2166136261;
  for (let i = 0; i < key.length; i++) h = Math.imul(h ^ key.charCodeAt(i), 16777619);
  return 100 + ((h >>> 0) % 900);
}

/** Someone's name as shown: theirs, or "Hidden #123" in privacy mode. */
export function shownName(d: Data, name: string, userId?: string | null): string;
export function shownName(d: Data, name: string | null, userId?: string | null): string | null;
export function shownName(d: Data, name: string | null, userId: string | null = null): string | null {
  if (!name || !isPrivate(d)) return name;
  return `Hidden #${tag(userId ?? name.trim().toLowerCase())}`;
}

/** Your own name as shown: yours, or "You" in privacy mode. */
export function myShownName(d: Data, name: string | null): string | null {
  return name && isPrivate(d) ? "You" : name;
}
