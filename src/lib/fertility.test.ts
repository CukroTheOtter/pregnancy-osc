import { describe, expect, it } from "vitest";
import { birthTitle, chance, cycleDay, cycleMs, cycleShades, genderEmoji, graceMs, hourShades, hoursDay, kidWord, level, litterOdds, rollLitter, untilFertile } from "./fertility";
import { funRows, linkedGroup, owner, partnerStats, pregnancyFor } from "./scope";
import { defaultSettings, emptyData, pregnancy } from "./testdata";
import type { AvatarProfile } from "./types";

const DAY = 86_400_000;

describe("fertility (matches the Rust engine)", () => {
  it("scales with the pregnancy length", () => {
    const s = defaultSettings(); // 1 month
    expect(cycleMs(s)).toBe(3 * DAY);
    expect(graceMs(s)).toBe(1.5 * DAY);
  });

  it("a fixed wait ignores the length but is capped", () => {
    const s = { ...defaultSettings(), graceMinutes: 60 };
    expect(graceMs(s)).toBe(3_600_000);
    expect(cycleMs(s)).toBe(3 * DAY);
    expect(graceMs({ ...s, graceMinutes: 24 * 60, defaultCustomMs: DAY })).toBe(DAY / 4);
  });

  it("cycle days and chances", () => {
    expect(cycleDay(0, 0, 28 * DAY)).toBe(1);
    expect(cycleDay(28 * DAY, 0, 28 * DAY)).toBe(1);
    expect(chance(13.5, 0.6)).toBeCloseTo(0.6);
    expect(level(chance(13, 0.6), 0.6)).toBe("peak");
    expect(level(chance(3, 0.6), 0.6)).toBe("low");
  });

  it("time until the fertile window", () => {
    expect(untilFertile(0, 0, 28 * DAY, 0.6)).toBeCloseTo(10 * DAY, -3); // day 1 -> day 11
    expect(untilFertile(12 * DAY, 0, 28 * DAY, 0.6)).toBe(0); // day 13 is now
  });
});

const prof = (id: string, extra: Partial<AvatarProfile> = {}): AvatarProfile => ({
  id, name: id, species: null, kind: null, hideBelly: null, bellyScale: null, fetalSounds: null,
  pregnancyId: null, followGlobal: false, lastSeenMs: 0, params: [], groupId: null, paramNames: {}, sps: { custom: false, sockets: [], types: {} }, ...extra,
});

describe("avatars and linking", () => {
  it("avatars in a group share a pregnancy", () => {
    const d = emptyData();
    d.settings.syncScope = "perAvatar";
    d.groups = { g: { id: "g", name: "Fluffs", pregnancyId: "p1" } };
    d.avatars = { a: prof("a", { groupId: "g" }), b: prof("b", { groupId: "g" }), c: prof("c") };
    d.pregnancies = [pregnancy({ id: "p1" })];
    expect(owner(d, "b")).toBe("g");
    expect(pregnancyFor(d, "b")?.id).toBe("p1");
    expect(pregnancyFor(d, "c")).toBeUndefined();
    expect(linkedGroup(d, "a").sort()).toEqual(["a", "b"]);
  });

  it("global mode ignores links", () => {
    const d = emptyData();
    d.activePregnancyId = "p1";
    d.pregnancies = [pregnancy({ id: "p1" })];
    expect(pregnancyFor(d, "anything")?.id).toBe("p1");
  });
});

describe("partner stats", () => {
  it("counts fun, pregnancies and kids", () => {
    const d = emptyData();
    const rex = { status: "confirmed" as const, name: "Rex", userId: "usr_rex", avatarName: null, species: null, speciesSource: "confirmed" as const, candidates: [] };
    d.funLog = [
      { timeMs: 1, partnerName: "Rex", partnerUserId: "usr_rex", avatarId: null, worldName: null },
      { timeMs: 5, partnerName: "Rex", partnerUserId: "usr_rex", avatarId: null, worldName: null },
      { timeMs: 3, partnerName: "Fox", partnerUserId: null, avatarId: null, worldName: null },
    ];
    d.pregnancies = [pregnancy({ id: "1", partner: rex, babyCount: 2, born: true, endedMs: 9 }), pregnancy({ id: "2", partner: rex })];
    const [first] = partnerStats(d);
    expect(first.name).toBe("Rex");
    expect(first.fun).toBe(2);
    expect(first.pregnancies).toBe(2);
    expect(first.kids).toBe(2);
  });

  it("the history has one row per wait, newest first", () => {
    const d = emptyData();
    const fun = (timeMs: number, avatarId: string, partnerName: string | null, took: boolean | null, waitMs: number | null) => ({ timeMs, partnerName, partnerUserId: null, avatarId, worldName: "W" + timeMs, took, waitMs });
    d.funLog = [
      fun(1, "avtr_a", "Old", null, null), // from before results were saved
      fun(2, "avtr_a", null, false, 2),
      fun(3, "avtr_a", "Fox", true, 2),
      fun(4, "avtr_b", null, false, 4),
      fun(5, "avtr_a", null, false, 5),
      fun(6, "avtr_a", "Rex", null, null),
      fun(7, "avtr_a", null, null, null),
    ];
    d.waiting = [{ timeMs: 7, revealMs: 99, chance: 0, cycleDay: 1, positive: true, babyCount: 1, partner: { status: "unknown", name: null, userId: null, avatarName: null, species: null, speciesSource: "fallback", candidates: [] }, avatarId: null, worldName: null, funTimes: [6, 7] }];
    const pick = (a: string | null) => funRows(d, a).map((r) => [r.timeMs, r.worldName, r.partnerName, r.count, r.result]);
    expect(pick("avtr_a")).toEqual([
      [6, "W6", "Rex", 2, "waiting"],
      [5, "W5", null, 1, "notPregnant"],
      [2, "W2", "Fox", 2, "pregnant"],
    ]);
    expect(pick(null)).toHaveLength(4);
  });
});

describe("litters", () => {
  it("bunch around the usual size, like fertility.rs litter_odds", () => {
    const odds = litterOdds({ usual: 4, spread: "some" });
    expect(odds).toHaveLength(8);
    expect(odds.reduce((a, b) => a + b, 0)).toBeCloseTo(1, 9);
    expect(odds[3]).toBeCloseTo(0.44, 1);
    expect(odds[2]).toBeCloseTo(odds[4], 9);
    expect(rollLitter({ usual: 8, spread: "little" }, 0.999)).toBe(8);
    expect(rollLitter({ usual: 1, spread: "little" }, 0)).toBe(1);
    expect(rollLitter({ usual: 4, spread: "some" }, 0.5)).toBe(4);
  });

  it("say litter, but eggs win", () => {
    expect(kidWord({ litter: 4 }, 6)).toBe("Litter of 6!");
    expect(kidWord({ litter: 4 }, 1)).toBe("One baby");
    expect(kidWord({ litter: null }, 2)).toBe("Twins!");
    expect(kidWord({ kind: "egg", litter: 4 }, 6)).toBe("6 eggs!");
    expect(birthTitle({ babyCount: 6, litter: 4 })).toBe("Litter of 6 born");
  });

  it("count genders past 4", () => {
    expect(genderEmoji(["girl", "boy"])).toBe("👧👦");
    expect(genderEmoji(["girl", "boy", "girl", "girl", "boy", "girl"])).toBe("👧×4 👦×2");
    expect(genderEmoji(["boy", "boy", "boy", "boy", "boy"])).toBe("👦×5");
  });
});

describe("set fertile hours", () => {
  const h = { startMin: 20 * 60, endMin: 23 * 60 };
  const lv = (m: number) => level(chance(hoursDay(m, h), 0.6), 0.6);
  it("peaks inside the window, like fertility.rs", () => {
    expect(lv(12 * 60)).toBe("low");
    expect(lv(20 * 60 + 1)).toBe("medium");
    expect(lv(21 * 60 + 40)).toBe("peak");
    expect(lv(23 * 60)).toBe("low");
  });
  it("crosses midnight", () => {
    const late = { startMin: 22 * 60, endMin: 2 * 60 };
    expect(hoursDay(23 * 60, late)).toBe(11);
    expect(hoursDay(60, late)).toBe(15);
    expect(hoursDay(3 * 60, late)).toBe(4);
    expect(hoursDay(720, { startMin: 0, endMin: 0 })).toBe(13);
  });
});

describe("ring shades", () => {
  it("are deepest on the peak", () => {
    const top = (xs: { strength: number }[]) => xs.indexOf(xs.reduce((a, b) => (b.strength > a.strength ? b : a)));
    const cyc = cycleShades();
    expect(cyc[top(cyc)].strength).toBe(1);
    expect(cyc[top(cyc)].from).toBeCloseTo(12 / 28);
    const hrs = hourShades({ startMin: 20 * 60, endMin: 23 * 60 });
    expect(hrs[0].from).toBeCloseTo((20 * 60) / 1440);
    expect(hrs[hrs.length - 1].to).toBeCloseTo((23 * 60) / 1440);
    expect(hrs[top(hrs)].strength).toBe(1);
  });
});
