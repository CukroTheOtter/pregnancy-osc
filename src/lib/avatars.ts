// Sorting the Avatars tab: which avatars sit up top and which fold away.

import { pregnancyIdOf } from "./scope";
import { humanDuration } from "./timeline";
import type { AppData, AvatarProfile } from "./types";

/** Avatars not worn for this long fold into "Older" (unless they're set up). */
export const RECENT_MS = 14 * 24 * 60 * 60_000;

export type Shelf = "main" | "older" | "other";

/** Whether VRChat showed Pregnancy System parameters on it (null: not checked yet). */
export function hasSystem(p: AvatarProfile): boolean | null {
  if (!p.params.length) return null;
  const own = new Set(Object.values(p.paramNames ?? {}));
  return p.params.some((n) => n.startsWith("Pregnancy/") || own.has(n));
}

/** Something about it was chosen by you or is going on (never folded away). */
export function isSetUp(d: AppData, p: AvatarProfile): boolean {
  const preg = pregnancyIdOf(d, p.id);
  return (
    !!p.groupId ||
    Object.keys(p.paramNames ?? {}).length > 0 ||
    !!p.sps?.custom ||
    p.species != null ||
    p.kind != null ||
    p.hideBelly != null ||
    p.bellyScale != null ||
    p.fetalSounds != null ||
    d.pregnancies.some((x) => x.id === preg && !x.endedMs) ||
    d.waiting.some((w) => w.avatarId === p.id)
  );
}

export function shelfOf(d: AppData, p: AvatarProfile, now: number): Shelf {
  if (p.id === d.currentAvatarId || isSetUp(d, p)) return "main";
  if (hasSystem(p) === false) return "other";
  if (p.lastSeenMs && now - p.lastSeenMs > RECENT_MS) return "older";
  return "main";
}

/** "seen 3 days ago", "seen just now", "not worn yet". */
export function seenText(lastSeenMs: number, now: number): string {
  if (!lastSeenMs) return "not worn yet";
  return now - lastSeenMs < 60_000 ? "seen just now" : `seen ${humanDuration(now - lastSeenMs)} ago`;
}

/** Search by avatar name, group name or ID. */
export function matches(d: AppData, p: AvatarProfile, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  const group = p.groupId ? d.groups?.[p.groupId]?.name : null;
  return [p.name, group, p.id].some((s) => s?.toLowerCase().includes(q));
}
