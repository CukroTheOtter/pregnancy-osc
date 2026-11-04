import { describe, expect, it } from "vitest";
import { chipRows, splitAchievements } from "./sharecard";
import { ACHIEVEMENTS } from "./achievements";

describe("the Share picture", () => {
  it("shows everything you've unlocked, rarest first", () => {
    const { rows, chips } = splitAchievements(["first_woohoo", "four_leaf_clover", "double_trouble", "one_of_each", "busy_bee"]);
    expect(rows.map((a) => a.id)).toEqual(["four_leaf_clover", "one_of_each", "double_trouble"]);
    expect(chips.map((a) => a.id)).toEqual(["busy_bee", "first_woohoo"]);
  });

  it("only lists what's unlocked", () => {
    expect(splitAchievements([])).toEqual({ rows: [], chips: [] });
    const all = splitAchievements(ACHIEVEMENTS.map((a) => a.id));
    expect(all.rows.length + all.chips.length).toBe(ACHIEVEMENTS.length);
  });

  it("wraps the icons", () => {
    expect(chipRows(0)).toBe(0);
    expect(chipRows(1)).toBe(1);
    expect(chipRows(28)).toBe(3);
  });
});
