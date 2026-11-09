// Which pregnancy belongs to which avatar. Mirrors AppData::owner and
// friends in crates/pregsys-core/src/model.rs.

import type { AppData, FunEvent, PendingConception, Pregnancy, Person } from "./types";

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

/** Names of the avatars a pregnancy is on (linked to it, or "every avatar" in global mode). */
export function pregnancyAvatarNames(d: AppData, p: Pregnancy): string[] {
  if (d.settings.syncScope === "global") return p.id === d.activePregnancyId ? ["every avatar"] : [];
  return Object.values(d.avatars)
    .filter((a) => pregnancyIdOf(d, owner(d, a.id)) === p.id)
    .map((a) => a.name ?? "Unnamed avatar");
}

/** The group a pregnancy belongs to (per-avatar mode only). */
export function pregnancyGroup(d: AppData, p: Pregnancy) {
  if (d.settings.syncScope === "global") return undefined;
  return Object.values(d.groups ?? {}).find((g) => g.pregnancyId === p.id);
}

/** An avatar showing this pregnancy, to open it on Home (null in global mode). */
export function pregnancyAvatar(d: AppData, p: Pregnancy): string | null {
  if (d.settings.syncScope === "global") return null;
  return Object.values(d.avatars).find((x) => pregnancyIdOf(d, owner(d, x.id)) === p.id)?.id ?? null;
}

/** Can Home show this pregnancy? (On an avatar, or the one global pregnancy.) */
export function canOpenPregnancy(d: AppData, p: Pregnancy): boolean {
  return !p.endedMs && (d.settings.syncScope === "global" ? p.id === d.activePregnancyId : !!pregnancyAvatar(d, p));
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

/** Already pregnant, "Only my partners" and not with one, or protected: no wait, no result. */
const notTry = (f: FunEvent) => !!(f.alreadyPregnant || f.notPartner || f.protected);
const notTryResult = (f: FunEvent) => (f.alreadyPregnant ? ("alreadyPregnant" as const) : f.notPartner ? ("notPartner" as const) : f.protected ? ("protected" as const) : null);

export interface PartnerFunRow {
  /** Their first WooHoo in that wait. */
  timeMs: number;
  worldName: string | null;
  /** The avatar it was on (its group's name in per-avatar mode). */
  avatarId: string | null;
  /** WooHoos with them in that wait. */
  count: number;
  /** "pregnant" only if it was them; "unknown" for WooHoos from before results were saved. */
  result: "waiting" | "pregnant" | "notPregnant" | "alreadyPregnant" | "notPartner" | "protected" | "unknown";
}

/**
 * Every WooHoo with one partner, newest first, one row per wait. A wait can
 * hold WooHoos with others too: it only says "pregnant" when it took with them,
 * and nothing about who while it's still waiting.
 */
export function partnerFunRows(d: AppData, partnerKey: string): PartnerFunRow[] {
  const theirs = (f: FunEvent) => !!(f.partnerName || f.partnerUserId) && personKey(f.partnerName, f.partnerUserId) === partnerKey;
  const waitOf = (f: FunEvent) => d.waiting.find((w) => w.timeMs === f.timeMs || w.funTimes?.includes(f.timeMs));
  const groups = new Map<string, FunEvent[]>();
  for (const f of d.funLog) {
    const w = waitOf(f);
    const wait = w ? Math.min(w.timeMs, ...(w.funTimes ?? [])) : (f.waitMs ?? (f.took != null ? f.timeMs : null));
    // Old WooHoos with no wait saved, and ones that weren't tries: one row each.
    const key = notTry(f) ? `p${f.timeMs}` : wait != null ? `w${wait}` : `f${f.timeMs}`;
    groups.set(key, [...(groups.get(key) ?? []), f]);
  }
  const out: PartnerFunRow[] = [];
  for (const list of groups.values()) {
    const mine = list.filter(theirs).sort((a, b) => a.timeMs - b.timeMs);
    if (!mine.length) continue;
    const took = list.find((f) => f.took);
    const known = list.some((f) => f.took != null || f.waitMs != null);
    out.push({
      timeMs: mine[0].timeMs,
      worldName: mine[0].worldName,
      avatarId: mine[0].avatarId,
      count: mine.length,
      result: notTryResult(list[0]) ?? (list.some(waitOf) ? "waiting" : took ? (theirs(took) ? "pregnant" : "notPregnant") : known ? "notPregnant" : "unknown"),
    });
  }
  return out.sort((a, b) => b.timeMs - a.timeMs);
}

export interface FunRow {
  /** Its first WooHoo: stays put, so it never hints at which one took. */
  timeMs: number;
  worldName: string | null;
  partnerName: string | null;
  partnerUserId: string | null;
  count: number;
  result: "waiting" | "pregnant" | "notPregnant" | "alreadyPregnant" | "notPartner" | "protected";
}

/**
 * This avatar's history (its group's, sharing one pregnancy), newest first:
 * one row per wait, since a wait has one result, and one per WooHoo while
 * already pregnant with someone named. WooHoos from before results were saved
 * have none, so they aren't listed.
 */
export function funRows(d: AppData, avatarId: string | null): FunRow[] {
  const mine = (f: FunEvent) => {
    if (!avatarId) return true;
    return d.settings.syncScope === "global" ? f.avatarId === avatarId : !!f.avatarId && owner(d, f.avatarId) === owner(d, avatarId);
  };
  const waits = new Map<string, FunEvent[]>();
  const waitOf = (f: FunEvent) => d.waiting.find((w) => w.timeMs === f.timeMs || w.funTimes?.includes(f.timeMs));
  for (const f of d.funLog) {
    // Not a try with nobody named: there's no picking who, so it only counts (Me), never listed.
    // Protected ones are, since you called them off yourself.
    if (notTry(f) && !f.protected && !f.partnerName) continue;
    const w = waitOf(f);
    const wait = w ? Math.min(w.timeMs, ...(w.funTimes ?? [])) : (f.waitMs ?? (f.took != null ? f.timeMs : null));
    const key = notTry(f) ? `p${f.timeMs}` : wait != null ? `w${wait}` : null;
    if (key != null) waits.set(key, [...(waits.get(key) ?? []), f]);
  }
  return [...waits.values()]
    .filter((list) => list.some(mine))
    .map((list) => {
      const fs = [...list].sort((a, b) => a.timeMs - b.timeMs);
      const took = fs.find((f) => f.took);
      return {
        timeMs: fs[0].timeMs,
        worldName: fs[0].worldName,
        partnerName: (took ?? fs.find((f) => f.partnerName))?.partnerName ?? null,
        partnerUserId: (took ?? fs.find((f) => f.partnerName))?.partnerUserId ?? null,
        count: fs.length,
        result: notTryResult(fs[0]) ?? (fs.some(waitOf) ? ("waiting" as const) : took ? ("pregnant" as const) : ("notPregnant" as const)),
      };
    })
    .sort((a, b) => b.timeMs - a.timeMs);
}

export interface PartnerStats {
  key: string;
  name: string;
  userId: string | null;
  fun: number;
  pregnancies: number;
  kids: number;
  firstMs: number;
  lastMs: number;
  /** One of your 💞 partners (added by hand). */
  mine: boolean;
}

/** Ended without a birth: left out of counts and achievements (as in model.rs). */
export const isStopped = (p: Pregnancy) => p.endedMs != null && !p.born;

export const personKey = (name: string | null, userId: string | null) => userId ?? (name ?? "").trim().toLowerCase();

/** The same person as a partner you added: by user ID when both have one, else by name (as in actions.rs). */
export const samePartner = (m: { name: string; userId: string | null }, name: string, userId: string | null) =>
  m.userId && userId ? m.userId === userId : m.name.trim().toLowerCase() === name.trim().toLowerCase();

/** One of your 💞 partners. */
export const isMyPartner = (d: AppData, name: string | null, userId: string | null) => !!name && (d.myPartners ?? []).some((m) => samePartner(m, name, userId));

export function partnerStats(d: AppData): PartnerStats[] {
  const map = new Map<string, PartnerStats>();
  const get = (name: string, userId: string | null, t: number) => {
    const key = personKey(name, userId);
    let s = map.get(key);
    if (!s) {
      s = { key, name, userId, fun: 0, pregnancies: 0, kids: 0, firstMs: t, lastMs: t, mine: false };
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
    if (!isStopped(p)) s.pregnancies++;
    if (p.born) s.kids += p.babyCount;
  }
  // Your partners: listed even before anything happened (from when they were added).
  for (const m of d.myPartners ?? []) {
    const found = [...map.values()].find((s) => samePartner(m, s.name, s.userId));
    if (found) found.mine = true;
    else map.set(personKey(m.name, m.userId), { key: personKey(m.name, m.userId), name: m.name, userId: m.userId, fun: 0, pregnancies: 0, kids: 0, firstMs: m.addedMs, lastMs: m.addedMs, mine: true });
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
