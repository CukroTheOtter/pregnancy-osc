import { describe, expect, it } from "vitest";
import { HOUR_MS, busiest, kickBins, kickHours, pickSession, smoothPath } from "./kicks";

describe("smooth line", () => {
  it("curves through every point", () => {
    expect(smoothPath([])).toBe("");
    expect(smoothPath([[10, 50]])).toBe("M6,50 L14,50");
    const d = smoothPath([[0, 90], [10, 20], [20, 90]]);
    expect(d.startsWith("M0,90 C")).toBe(true);
    expect(d).toContain(" 10,20 C");
    expect(d.endsWith(" 20,90")).toBe(true);
  });
});

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

  it("fits a session in 24 bars or fewer, lined up with the clock", () => {
    // 13:50 to 14:30: 40 minutes in 5-minute bars, from 13:50.
    const from = NOW - 40 * 60_000;
    const { bins, binMs } = kickBins(
      [
        { timeMs: from - 1, strength: "weak" }, // before the session
        { timeMs: from + 1, strength: "strong" },
        { timeMs: NOW - 1, strength: "extreme" },
      ],
      from,
      NOW,
    );
    expect(binMs).toBe(5 * 60_000);
    expect(bins).toHaveLength(8);
    expect(bins[0]).toMatchObject({ startMs: from, strong: 1, total: 1 });
    expect(bins[7]).toMatchObject({ extreme: 1, total: 1 });

    // 9 hours: 30-minute bars, the first one starting on the half hour.
    const long = kickBins([], NOW - 9 * HOUR_MS - 10 * 60_000, NOW);
    expect(long.binMs).toBe(HOUR_MS / 2);
    expect(long.bins.length).toBeLessThanOrEqual(24);
    expect(new Date(long.bins[0].startMs).getMinutes()).toBe(0);
  });
});

describe("which session the chart shows", () => {
  const H = HOUR_MS;
  const k = (timeMs: number) => ({ timeMs, strength: "weak" as const });
  const runs = [
    { sinceMs: 0, untilMs: 3 * H }, // wore this avatar
    { sinceMs: 10 * H, untilMs: 12 * H }, // another avatar
  ];
  const current = { sinceMs: 20 * H, untilMs: null };

  it("is this session while you wear the avatar", () => {
    expect(pickSession([k(H)], runs, current, true, true, 21 * H)).toEqual({ fromMs: 20 * H, toMs: 21 * H, live: true });
  });

  it("is the avatar's own last session when you view another one", () => {
    expect(pickSession([k(H), k(2 * H)], runs, current, true, false, 21 * H)).toEqual({ fromMs: 0, toMs: 3 * H, live: false });
    // Worn earlier this session: this one, still going.
    expect(pickSession([k(H), k(20.5 * H)], runs, current, true, false, 21 * H)).toEqual({ fromMs: 20 * H, toMs: 21 * H, live: true });
  });

  it("is the avatar's last session after VRChat closes", () => {
    const closed = { sinceMs: 20 * H, untilMs: 22 * H };
    expect(pickSession([k(H), k(21 * H)], runs, closed, false, true, 30 * H)).toEqual({ fromMs: 20 * H, toMs: 22 * H, live: false });
    expect(pickSession([k(H)], runs, closed, false, true, 30 * H)).toEqual({ fromMs: 0, toMs: 3 * H, live: false });
  });

  it("ends a session with no known end at its last kick", () => {
    const open = [{ sinceMs: 0, untilMs: null }];
    expect(pickSession([k(H), k(2 * H)], open, current, true, false, 21 * H)).toEqual({ fromMs: 0, toMs: 2 * H, live: false });
  });

  it("is none with no kicks in any session", () => {
    expect(pickSession([k(5 * H)], runs, current, true, false, 21 * H)).toBeNull();
    expect(pickSession([], [], null, false, true, 21 * H)).toBeNull();
  });
});
