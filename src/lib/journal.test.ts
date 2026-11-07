import { describe, expect, it } from "vitest";
import { journalFor, parseScan, rareRoll, scanTag, MAX_CRAVINGS, MAX_RARE_SCANS } from "./journal";
import { emptyData, pregnancy } from "./testdata";
import { DAY_MS } from "./timeline";

// pregnancy() runs at 1 real day = 1 week.
const START = 1_700_000_000_000;
const week = (w: number) => START + w * DAY_MS;

describe("journal", () => {
  it("adds scans as the weeks pass, and only counts babies from week 12", () => {
    const d = emptyData();
    const p = pregnancy({ babyCount: 2 });
    const plain = (now: number) => journalFor(d, p, now).filter((x) => !x.rare);
    expect(plain(week(5)).map((x) => x.key)).toEqual(["started"]);
    const at13 = plain(week(13));
    expect(at13.map((x) => x.key)).toEqual(["scan-12", "scan-8", "started"]);
    expect(at13.find((x) => x.key === "scan-8")!.scan!.babies).toBe(0);
    expect(at13.find((x) => x.key === "scan-12")!.scan!.babies).toBe(2);
    expect(at13.find((x) => x.key === "scan-12")!.timeMs).toBe(week(12));
  });

  it("stops at the end and shows the birth", () => {
    const d = emptyData();
    const p = pregnancy({ endedMs: week(15), born: true, babyCount: 3 });
    const items = journalFor(d, p, week(30));
    expect(items[0]).toMatchObject({ key: "born", title: "Triplets born!" });
    expect(items.some((x) => x.key === "scan-20")).toBe(false);
    expect(journalFor(d, pregnancy(), week(37)).some((x) => x.key === "scan-36")).toBe(true);
  });

  it("keeps your notes with their scan, but not the app's old event entries", () => {
    const d = emptyData();
    const p = pregnancy();
    d.journal = [
      { id: "j1", timeMs: week(2), pregnancyId: "p1", kind: "note", title: "", text: "Hi bean", image: scanTag({ babies: 0, week: 2, seed: 7 }) },
      { id: "j2", timeMs: week(0), pregnancyId: "p1", kind: "event", title: "Pregnancy started", text: "", image: null },
      { id: "j3", timeMs: week(1), pregnancyId: "other", kind: "note", title: "", text: "Not this one", image: null },
    ];
    const items = journalFor(d, p, week(3)).filter((x) => !x.rare);
    expect(items.map((x) => x.key)).toEqual(["j1", "started"]);
    expect(items[0]).toMatchObject({ note: true, text: "Hi bean", scan: { babies: 0, week: 2, seed: 7 } });
  });

  it("never dates anything before the start, after jumping ahead", () => {
    const d = emptyData();
    // Started "now" at week 0, then jumped to week 21: the timeline start moves back.
    const created = week(30);
    const p = pregnancy({ createdMs: created, timeline: { ...pregnancy().timeline, startMs: created - 21 * DAY_MS } });
    const items = journalFor(d, p, created + 60_000).filter((x) => !x.rare);
    expect(items.map((x) => x.key)).toEqual(["scan-20", "scan-12", "scan-8", "started"]);
    expect(items.every((x) => x.timeMs >= created)).toBe(true);
  });

  it("found out shows its real week, and nothing comes before it", () => {
    const d = emptyData();
    // Conceived at week 0, found out at week 8.4 (a short pregnancy, a long wait).
    const found = week(8.4);
    const p = pregnancy({ source: "conception", createdMs: found, timeline: pregnancy().timeline });
    const items = journalFor(d, p, week(9)).filter((x) => !x.rare);
    expect(items.map((x) => x.key)).toEqual(["scan-8", "found-out", "conceived"]);
    expect(Math.floor(items.find((x) => x.key === "found-out")!.week!)).toBe(8);
    expect(items.find((x) => x.key === "conceived")!.week).toBe(0);
    // The week 8 scan waited for the reveal.
    expect(items.find((x) => x.key === "scan-8")!.timeMs).toBe(found);
    expect(journalFor(d, p, week(9)).every((x) => x.key === "conceived" || x.timeMs >= found)).toBe(true);
  });

  it("reads only real scan tags", () => {
    expect(parseScan("scan:2:12:99")).toEqual({ babies: 2, week: 12, seed: 99 });
    expect(parseScan("scan:2:20:99:wave")).toEqual({ babies: 2, week: 20, seed: 99, variant: "wave" });
    expect(parseScan("scan:2:20:99:nope")).toEqual({ babies: 2, week: 20, seed: 99 });
    expect(parseScan("data:image/png;base64,xx")).toBeNull();
    expect(parseScan(null)).toBeNull();
  });

  it("rolls rare weeks the same way every time, and never spoils the count", () => {
    const rolls = Array.from({ length: 400 }, (_, i) => pregnancy({ id: `p${i}`, babyCount: 1 }));
    for (const p of rolls) {
      for (let w = 6; w < 40; w++) {
        const r = rareRoll(p, w);
        expect(rareRoll(p, w)).toEqual(r);
        if (r?.scan) expect(w).toBeGreaterThanOrEqual(12);
        expect(r?.scan).not.toBe("highfive");
      }
    }
    const scans = rolls.flatMap((p) => Array.from({ length: 28 }, (_, i) => rareRoll(p, i + 12)?.scan)).filter(Boolean);
    const share = scans.length / (rolls.length * 28);
    expect(share).toBeGreaterThan(0.1);
    expect(share).toBeLessThan(0.25);
  });

  it("keeps cute scans and cravings special: each scan once, a few of each", () => {
    const d = emptyData();
    let anyCraving = false;
    for (let i = 0; i < 200; i++) {
      const items = journalFor(d, pregnancy({ id: `p${i}`, babyCount: 2 }), week(41));
      const scans = items.filter((x) => x.scan?.variant).map((x) => x.scan!.variant);
      const cravings = items.filter((x) => x.key.startsWith("craving-")).length;
      expect(new Set(scans).size).toBe(scans.length);
      expect(scans.length).toBeLessThanOrEqual(MAX_RARE_SCANS);
      expect(cravings).toBeLessThanOrEqual(MAX_CRAVINGS);
      anyCraving ||= cravings > 0;
    }
    expect(anyCraving).toBe(true);
  });

  it("turns saved moments into polaroids", () => {
    const d = emptyData();
    const p = pregnancy({ partner: { ...pregnancy().partner, name: "Fox" } });
    d.journal = [{ id: "m1", timeMs: week(20), pregnancyId: "p1", kind: "moment", title: "partner-kicks", text: "", image: null }];
    const m = journalFor(d, p, week(21)).find((x) => x.key === "m1")!;
    expect(m.card).toMatchObject({ kind: "partner-kicks", caption: "Kicking for Fox!" });
  });
});
