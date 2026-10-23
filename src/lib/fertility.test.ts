import { describe, expect, it } from "vitest";
import { chance, cycleDay, cycleMs, graceMs, level, untilFertile } from "./fertility";
import { linkedGroup, owner, partnerStats, pregnancyFor } from "./scope";
import { defaultSettings, emptyData, pregnancy } from "./testdata";
import type { AvatarProfile } from "./types";

const DAY = 86_400_000;

describe("fertility (matches the Rust engine)", () => {
  it("scales with the pregnancy length", () => {
    const s = defaultSettings(); // 1 month
    expect(cycleMs(s)).toBe(3 * DAY);
    expect(graceMs(s)).toBe(1.5 * DAY);
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
});
