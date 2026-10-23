import { describe, expect, it } from "vitest";
import { RECENT_MS, hasSystem, matches, shelfOf } from "./avatars";
import { emptyData } from "./testdata";
import type { AvatarProfile } from "./types";

const NOW = 1_700_000_000_000;
const prof = (id: string, extra: Partial<AvatarProfile> = {}): AvatarProfile => ({
  id, name: id, autoName: true, species: null, kind: null, hideBelly: null, bellyScale: null, fetalSounds: null,
  pregnancyId: null, followGlobal: false, lastSeenMs: NOW, params: ["Pregnancy/Stage"], groupId: null, paramNames: {}, sps: { custom: false, sockets: [], types: {} }, ...extra,
});

describe("avatar shelves", () => {
  it("folds old and non-Pregnancy System avatars", () => {
    const d = emptyData();
    const old = prof("old", { lastSeenMs: NOW - RECENT_MS - 1 });
    const plain = prof("plain", { params: ["VelocityX"] });
    const unchecked = prof("unchecked", { params: [], lastSeenMs: 0 });
    expect(shelfOf(d, prof("new"), NOW)).toBe("main");
    expect(shelfOf(d, old, NOW)).toBe("older");
    expect(shelfOf(d, plain, NOW)).toBe("other");
    expect(hasSystem(unchecked)).toBeNull();
    expect(shelfOf(d, unchecked, NOW)).toBe("main");
  });

  it("keeps worn and set-up avatars up top", () => {
    const d = emptyData();
    const old = NOW - RECENT_MS * 3;
    d.currentAvatarId = "wearing";
    expect(shelfOf(d, prof("wearing", { params: ["VelocityX"], lastSeenMs: old }), NOW)).toBe("main");
    expect(shelfOf(d, prof("species", { species: "fox", lastSeenMs: old }), NOW)).toBe("main");
    // A name alone doesn't keep it up top.
    expect(shelfOf(d, prof("named", { autoName: false, lastSeenMs: old }), NOW)).toBe("older");
    expect(shelfOf(d, prof("grouped", { groupId: "g", lastSeenMs: old }), NOW)).toBe("main");
    // Custom parameter names count as the Pregnancy System.
    expect(hasSystem(prof("renamed", { params: ["Belly"], paramNames: { stage: "Belly" } }))).toBe(true);
  });

  it("searches names, groups and IDs", () => {
    const d = emptyData();
    d.groups = { g: { id: "g", name: "Fluffs", pregnancyId: null } };
    const p = prof("avtr_123", { name: "Bunny Base", groupId: "g" });
    expect(matches(d, p, "bun")).toBe(true);
    expect(matches(d, p, "FLUFF")).toBe(true);
    expect(matches(d, p, "avtr_12")).toBe(true);
    expect(matches(d, p, "fox")).toBe(false);
  });
});
