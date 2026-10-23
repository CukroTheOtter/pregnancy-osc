// Which pregnancy belongs to which avatar. Mirrors AppData::owner and
// friends in crates/pregsys-core/src/model.rs.

import type { AppData, PendingConception, Pregnancy, Person } from "./types";

/** Who holds the pregnancy this avatar shows: its group's id, or the avatar itself. */
export function owner(d: AppData, avatarId: string | null): string | null {
  if (d.settings.syncScope === "global" || !avatarId) return null;
  if (d.groups?.[avatarId]) return avatarId;
  const g = d.avatars[avatarId]?.groupId;
  return g && d.groups?.[g] ? g : avatarId;
}

/** The pregnancy id an owner (group or avatar) holds. */
export function pregnancyIdOf(d: AppData, ownerId: string | null): string | null {
  if (!ownerId) return null;
  const g = d.groups?.[ownerId];
  if (g) return g.pregnancyId;
  const prof = d.avatars[ownerId];
  return prof ? (prof.followGlobal ? d.activePregnancyId : prof.pregnancyId) : null;
}

export function pregnancyFor(d: AppData, avatarId: string | null): Pregnancy | undefined {
  let id: string | null;
  if (d.settings.syncScope === "global") id = d.activePregnancyId;
  else if (!avatarId) id = d.activePregnancyId;
  else id = pregnancyIdOf(d, owner(d, avatarId));
  return d.pregnancies.find((p) => p.id === id && !p.endedMs);
}

/** Avatars in a group, most recently worn first. */
export function groupMembers(d: AppData, groupId: string): string[] {
  return Object.values(d.avatars)
    .filter((a) => a.groupId === groupId)
    .sort((a, b) => b.lastSeenMs - a.lastSeenMs)
    .map((a) => a.id);
}

/** A name for an owner: the group's name, or the avatar's. */
export function ownerName(d: AppData, ownerId: string | null): string {
  const g = ownerId ? d.groups?.[ownerId] : undefined;
  return g ? g.name : avatarName(d, ownerId);
}

/** An avatar to show (and open) for an owner: itself, or the group's last worn. */
export function ownerAvatar(d: AppData, ownerId: string | null): string | null {
  if (!ownerId) return null;
  return d.groups?.[ownerId] ? (groupMembers(d, ownerId)[0] ?? null) : ownerId;
}

export function pendingFor(d: AppData, avatarId: string | null): PendingConception | undefined {
  const global = d.settings.syncScope === "global";
  const o = owner(d, avatarId);
  return d.waiting.find((w) => global || w.avatarId === o);
}

/** Avatars sharing this avatar's pregnancy (including itself). */
export function linkedGroup(d: AppData, avatarId: string): string[] {
  const o = owner(d, avatarId);
  return Object.keys(d.avatars).filter((k) => owner(d, k) === o);
}

export function avatarName(d: AppData, id: string | null): string {
  if (!id) return "Your avatar";
  return d.avatars[id]?.name ?? "Unnamed avatar";
}

// ---------- partner stats ----------

export interface PartnerStats {
  key: string;
  name: string;
  userId: string | null;
  fun: number;
  pregnancies: number;
  kids: number;
  firstMs: number;
  lastMs: number;
}

export const personKey = (name: string | null, userId: string | null) => userId ?? (name ?? "").trim().toLowerCase();

export function partnerStats(d: AppData): PartnerStats[] {
  const map = new Map<string, PartnerStats>();
  const get = (name: string, userId: string | null, t: number) => {
    const key = personKey(name, userId);
    let s = map.get(key);
    if (!s) {
      s = { key, name, userId, fun: 0, pregnancies: 0, kids: 0, firstMs: t, lastMs: t };
      map.set(key, s);
    }
    s.firstMs = Math.min(s.firstMs, t);
    s.lastMs = Math.max(s.lastMs, t);
    if (userId) s.userId = userId;
    return s;
  };
  for (const f of d.funLog) {
    if (f.partnerName) get(f.partnerName, f.partnerUserId, f.timeMs).fun++;
  }
  for (const p of d.pregnancies) {
    if (!p.partner.name) continue;
    // When it really happened: conception for "first", the latest event for "last".
    const s = get(p.partner.name, p.partner.userId, p.timeline.startMs);
    s.lastMs = Math.max(s.lastMs, p.endedMs ?? p.createdMs);
    s.pregnancies++;
    if (p.born) s.kids += p.babyCount;
  }
  return [...map.values()].sort((a, b) => b.lastMs - a.lastMs);
}

export function statsFor(d: AppData, name: string | null, userId: string | null): PartnerStats | undefined {
  if (!name) return undefined;
  return partnerStats(d).find((s) => s.key === personKey(name, userId));
}

export function personFromStats(s: PartnerStats): Person {
  return { name: s.name, userId: s.userId, avatarName: null, lastSeenMs: s.lastMs };
}

export function ordinal(n: number): string {
  const v = n % 100;
  const suffix = v >= 11 && v <= 13 ? "th" : ["th", "st", "nd", "rd"][n % 10] ?? "th";
  return `${n}${suffix}`;
}
