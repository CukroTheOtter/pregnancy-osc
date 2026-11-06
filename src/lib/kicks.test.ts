import { describe, expect, it } from "vitest";
import { HOUR_MS, busiest, kickHours } from "./kicks";

// 14:30 local time, so the current hour started at 14:00.
const NOW = new Date(2026, 10, 5, 14, 30).getTime();
const TOP = new Date(2026, 10, 5, 14, 0).getTime();

describe("kick chart", () => {
  it("puts kicks in clock-hour bars, the last bar being this hour", () => {
    const hours = kickHours(
      [
        { timeMs: NOW - 60_000, strength: "weak" },
        { timeMs: TOP - 1, strength: "extreme" },
        { timeMs: TOP - 2 * HOUR_MS + 5, strength: "strong" },
        { timeMs: TOP - 2 * HOUR_MS + 9, strength: "weak" },
        { timeMs: TOP - 24 * HOUR_MS, strength: "weak" }, // too old
      ],
      NOW,
    );
    expect(hours).toHaveLength(24);
    expect(hours[23]).toMatchObject({ startMs: TOP, weak: 1, total: 1 });
    expect(hours[22]).toMatchObject({ extreme: 1, total: 1 });
    expect(hours[21]).toMatchObject({ weak: 1, strong: 1, total: 2 });
    expect(hours.reduce((s, h) => s + h.total, 0)).toBe(4);
  });

  it("finds the busiest hour, the latest on a tie", () => {
    const hours = kickHours([{ timeMs: TOP - 5 * HOUR_MS, strength: "weak" }, { timeMs: TOP + 60_000, strength: "weak" }], NOW);
    expect(busiest(hours)?.startMs).toBe(TOP);
    expect(busiest(kickHours([], NOW))).toBeNull();
  });
});
