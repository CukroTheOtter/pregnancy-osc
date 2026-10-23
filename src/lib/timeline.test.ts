import { describe, expect, it } from "vitest";
import { DAY_MS, dueMs, newTimeline, pause, progress, resume, retime, setProgress, speedLabel, trimester, week } from "./timeline";

const T0 = 1_700_000_000_000;

describe("timeline (matches the Rust engine)", () => {
  it("real time", () => {
    const t = newTimeline(T0, "realtime", 99);
    expect(t.speed).toBe(1);
    expect(progress(t, T0 + 140 * DAY_MS)).toBeCloseTo(0.5);
    expect(dueMs(t, T0)).toBe(T0 + 280 * DAY_MS);
  });

  it("accelerated: one day is one week", () => {
    const t = newTimeline(T0, "accelerated", 7);
    expect(week(t, T0 + DAY_MS)).toBeCloseTo(1);
    expect(speedLabel(t)).toBe("1 real day = 1 week");
  });

  it("correct after weeks away", () => {
    const t = newTimeline(T0, "accelerated", 7);
    expect(progress(t, T0 + 50 * DAY_MS)).toBeGreaterThan(1.2);
  });

  it("pause and resume", () => {
    let t = newTimeline(T0, "realtime", 1);
    t = pause(t, T0 + 10 * DAY_MS);
    const p = progress(t, T0 + 10 * DAY_MS);
    expect(progress(t, T0 + 30 * DAY_MS)).toBe(p);
    t = resume(t, T0 + 30 * DAY_MS);
    expect(progress(t, T0 + 40 * DAY_MS)).toBeCloseTo(20 / 280);
    expect(dueMs(t, T0 + 40 * DAY_MS)).toBe(T0 + 300 * DAY_MS);
  });

  it("retime keeps progress", () => {
    const t = newTimeline(T0, "realtime", 1);
    const now = T0 + 70 * DAY_MS;
    const r = retime(t, now, "accelerated", 7);
    expect(progress(r, now)).toBeCloseTo(progress(t, now), 6);
    expect(week(r, now + DAY_MS) - week(r, now)).toBeCloseTo(1, 5);
  });

  it("set progress / back-date", () => {
    const t = setProgress(newTimeline(T0, "realtime", 1), T0, 0.25);
    expect(week(t, T0)).toBeCloseTo(10);
  });

  it("trimesters", () => {
    expect(trimester(0.1)).toBe(1);
    expect(trimester(0.5)).toBe(2);
    expect(trimester(0.9)).toBe(3);
  });
});
