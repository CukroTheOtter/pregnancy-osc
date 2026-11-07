import { describe, expect, it } from "vitest";
import { funRows, partnerFunRows, personKey } from "./scope";
import { emptyData } from "./testdata";
import type { FunEvent, PendingConception } from "./types";

const ember = { partnerName: "Ember Fox", partnerUserId: "usr_1" };
const rex = { partnerName: "Rex", partnerUserId: "usr_2" };
const fun = (timeMs: number, who: { partnerName: string | null; partnerUserId: string | null }, extra: Partial<FunEvent> = {}): FunEvent => ({
  timeMs, avatarId: "avtr_a", worldName: "Cabin", ...who, ...extra,
});

describe("partnerFunRows", () => {
  const key = personKey("Ember Fox", "usr_1");

  it("groups a partner's WooHoos by wait and only says pregnant when it was them", () => {
    const d = emptyData();
    d.funLog = [
      // Wait 1: Ember twice, Rex once; Rex's took.
      fun(100, ember, { waitMs: 100, took: false }),
      fun(200, rex, { waitMs: 100, took: true }),
      fun(300, ember, { waitMs: 100, took: false }),
      // Wait 2: Ember's took.
      fun(1000, ember, { waitMs: 1000, took: true }),
      // Wait 3: nothing took.
      fun(2000, ember, { waitMs: 2000, took: false }),
      // From before results were saved.
      fun(50, ember),
    ];
    const rows = partnerFunRows(d, key);
    expect(rows.map((r) => [r.timeMs, r.count, r.result])).toEqual([
      [2000, 1, "notPregnant"],
      [1000, 1, "pregnant"],
      [100, 2, "notPregnant"],
      [50, 1, "unknown"],
    ]);
    expect(partnerFunRows(d, personKey("Rex", "usr_2")).map((r) => r.result)).toEqual(["pregnant"]);
  });

  it("says waiting without hinting who", () => {
    const d = emptyData();
    d.funLog = [fun(100, ember), fun(200, rex)];
    d.waiting = [{ timeMs: 100, funTimes: [100, 200] } as PendingConception];
    expect(partnerFunRows(d, key)).toEqual([{ timeMs: 100, worldName: "Cabin", avatarId: "avtr_a", count: 1, result: "waiting" }]);
  });

  it("lists WooHoos while already pregnant on their own", () => {
    const d = emptyData();
    d.funLog = [fun(100, ember, { waitMs: 100, took: true }), fun(500, ember, { alreadyPregnant: true }), fun(600, ember, { alreadyPregnant: true })];
    expect(partnerFunRows(d, key).map((r) => [r.timeMs, r.result])).toEqual([
      [600, "alreadyPregnant"],
      [500, "alreadyPregnant"],
      [100, "pregnant"],
    ]);
    // Nobody named: it only counts, it isn't listed.
    d.funLog.push(fun(700, { partnerName: null, partnerUserId: null }, { alreadyPregnant: true }));
    expect(funRows(d, null).map((r) => [r.timeMs, r.count, r.result])).toEqual([
      [600, 1, "alreadyPregnant"],
      [500, 1, "alreadyPregnant"],
      [100, 1, "pregnant"],
    ]);
  });

  it("skips WooHoos with nobody named", () => {
    const d = emptyData();
    d.funLog = [fun(100, { partnerName: null, partnerUserId: null })];
    expect(partnerFunRows(d, personKey(null, null))).toEqual([]);
  });
});
