import { describe, expect, it } from "vitest";
import { computeBaby, guessSpecies, hash, parents, rollSpecies, type GeneticsContext, CONFIDENT } from "./genetics";
import { baby, pregnancy } from "./testdata";
import { dueReveals, isTraitVisible, newMilestones, sizeOf } from "./reveals";
import { DAY_MS } from "./timeline";

const ctx: GeneticsContext = { userId: "usr_me", userName: "Me", hybridChance: 0.05 };

const foxPartner = {
  status: "confirmed" as const,
  name: "Ember",
  userId: "usr_ember",
  avatarName: "Kitsune",
  species: "fox",
  speciesSource: "confirmed" as const,
  candidates: [],
};

describe("hybrid roll", () => {
  it("is deterministic", () => {
    expect(hash("abc")).toBe(hash("abc"));
    expect(rollSpecies("seed1", "human", "fox", 0.05)).toEqual(rollSpecies("seed1", "human", "fox", 0.05));
  });

  it("same species never hybridizes", () => {
    expect(rollSpecies("x", "fox", "fox", 1)).toEqual({ parts: { fox: 1 } });
  });

  it("is rare at the default chance and common when raised", () => {
    const count = (chance: number) =>
      Array.from({ length: 4000 }, (_, i) => rollSpecies(`s${i}`, "cat", "wolf", chance)).filter((b) => Object.keys(b.parts).length > 1).length;
    const rare = count(0.05) / 4000;
    expect(rare).toBeGreaterThan(0.03);
    expect(rare).toBeLessThan(0.07);
    expect(count(1)).toBe(4000);
    expect(count(0)).toBe(0);
  });

  it("hybrid weights sum to 1", () => {
    const b = rollSpecies("s1", "cat", "wolf", 1);
    const total = Object.values(b.parts).reduce((x, y) => x + y, 0);
    expect(total).toBeCloseTo(1);
  });
});

describe("babies", () => {
  it("results survive a reload (same inputs, same baby)", () => {
    const p = pregnancy({ partner: foxPartner, babyCount: 2, babies: [baby(0), baby(1)] });
    const again = JSON.parse(JSON.stringify(p));
    expect(computeBaby(again, again.babies[0], ctx)).toEqual(computeBaby(p, p.babies[0], ctx));
  });

  it("twins can differ", () => {
    let differ = 0;
    for (let i = 0; i < 50; i++) {
      const p = pregnancy({ createdMs: 1_700_000_000_000 + i, partner: foxPartner, babyCount: 2, babies: [baby(0), baby(1)] });
      const a = computeBaby(p, p.babies[0], ctx);
      const b = computeBaby(p, p.babies[1], ctx);
      if (JSON.stringify(a.traits) !== JSON.stringify(b.traits)) differ++;
    }
    expect(differ).toBeGreaterThan(40);
  });

  it("unknown partner falls back to the user's species", () => {
    const p = pregnancy({ motherSpecies: "cat" });
    const [, b] = parents(p, ctx);
    expect(b.species).toBe("cat");
    expect(b.source).toBe("fallback");
    expect(computeBaby(p, p.babies[0], ctx).mainSpecies).toBe("cat");
  });

  it("manual species override wins and is marked", () => {
    const p = pregnancy({ partner: foxPartner, babies: [baby(0, { manualSpecies: { parts: { dragon: 60, fox: 40 } }, mode: "manual" })] });
    const r = computeBaby(p, p.babies[0], { ...ctx, hybridChance: 0 });
    expect(r.manualSpecies).toBe(true);
    expect(r.hybrid).toBe(true);
    expect(r.blend.parts.dragon).toBeCloseTo(0.6);
  });

  it("locks win and survive rerolls; reroll changes the rest", () => {
    const base = pregnancy({ partner: foxPartner });
    const locked = baby(0, { locks: { ears: "bunny", sex: "girl" } });
    const r0 = computeBaby(base, locked, ctx);
    expect(r0.traits.ears.value).toBe("bunny");
    expect(r0.traits.ears.from).toBe("locked");
    let changed = false;
    for (let n = 1; n < 20; n++) {
      const r = computeBaby(base, { ...locked, reroll: n }, ctx);
      expect(r.traits.ears.value).toBe("bunny");
      expect(r.traits.sex.value).toBe("girl");
      expect(r.blend).toEqual(r0.blend);
      if (JSON.stringify(r.traits) !== JSON.stringify(r0.traits)) changed = true;
    }
    expect(changed).toBe(true);
  });

  it("changing the partner recalculates, but manual choices stay", () => {
    const manual = baby(0, { manualSpecies: { parts: { bunny: 1 } }, locks: { eyeColor: "#123456" } });
    const p1 = pregnancy({ partner: foxPartner, babies: [manual] });
    const p2 = pregnancy({ partner: { ...foxPartner, name: "Rex", userId: "usr_rex", species: "wolf" }, babies: [manual] });
    const a = computeBaby(p1, manual, ctx), b = computeBaby(p2, manual, ctx);
    expect(a.mainSpecies).toBe("bunny");
    expect(b.mainSpecies).toBe("bunny");
    expect(b.traits.eyeColor.value).toBe("#123456");
  });
});

describe("species guessing", () => {
  it("keywords", () => {
    const g = guessSpecies("Kitsune Deluxe (Quest)")!;
    expect(g.species).toBe("fox");
    expect(g.confidence).toBeGreaterThanOrEqual(CONFIDENT);
  });

  it("substring match is low confidence", () => {
    const g = guessSpecies("Foxgirl3000")!;
    expect(g.species).toBe("fox");
    expect(g.confidence).toBeLessThan(CONFIDENT);
  });

  it("remembered answers beat keywords", () => {
    expect(guessSpecies("Kitsune", { kitsune: "wolf" })!.species).toBe("wolf");
    expect(guessSpecies("Whatever", { usr_x: "bat" }, {}, "usr_x")!.species).toBe("bat");
  });

  it("community table", () => {
    expect(guessSpecies("Odd Base v2", {}, { "odd base v2": "deer" })).toEqual({ species: "deer", confidence: 0.85, source: "community" });
  });

  it("unknown names give nothing", () => {
    expect(guessSpecies("Protogen MK3")).toBeNull();
    expect(guessSpecies(null)).toBeNull();
  });
});

describe("reveals and milestones", () => {
  it("missed scans are all due on next open", () => {
    const p = pregnancy();
    // Accelerated: 25 days = week 25.
    const due = dueReveals(p, p.timeline.startMs + 25 * DAY_MS).map((r) => r.key);
    expect(due).toEqual(["sex", "ears", "coat", "pattern"]);
  });

  it("revealed traits are visible; spoiler 'all' shows everything", () => {
    const p = pregnancy({ reveals: { ears: 1 } });
    expect(isTraitVisible(p, "ears", "surprise", false)).toBe(true);
    expect(isTraitVisible(p, "eyeColor", "surprise", false)).toBe(false);
    expect(isTraitVisible(p, "eyeColor", "surprise", true)).toBe(true);
    expect(isTraitVisible(p, "eyeColor", "all", false)).toBe(true);
  });

  it("milestones", () => {
    const p = pregnancy({ milestones: ["w4"] });
    expect(newMilestones(p, p.timeline.startMs + 14 * DAY_MS).map((m) => m.key)).toEqual(["w8", "tri2"]);
  });

  it("sizes", () => {
    expect(sizeOf(20)).toBe("a banana");
    expect(sizeOf(99)).toBe("a pumpkin");
    expect(sizeOf(40, true)).toBe("an ostrich egg");
  });
});
