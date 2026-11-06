import { describe, expect, it } from "vitest";
import { curve, scanStats, statLines, weightText } from "./scanstats";

describe("scan readouts", () => {
  it("follows the growth curve between points", () => {
    const c: [number, number][] = [[10, 100], [20, 300]];
    expect(curve(c, 5)).toBe(100);
    expect(curve(c, 15)).toBe(200);
    expect(curve(c, 30)).toBe(300);
  });

  it("measures CRL early, then head, thigh and weight", () => {
    const [early] = scanStats(8, 0, 1);
    expect(early.crlMm).toBeGreaterThan(10);
    expect(early.efwG).toBeNull();
    const [late] = scanStats(30, 1, 1);
    expect(late.crlMm).toBeNull();
    expect(late.efwG).toBeGreaterThan(1000);
    expect(late.lie).not.toBeNull();
  });

  it("says the same thing for the same scan", () => {
    expect(scanStats(20, 2, 42)).toEqual(scanStats(20, 2, 42));
    expect(scanStats(20, 2, 42)).not.toEqual(scanStats(20, 2, 43));
  });

  it("shows one baby per line for multiples, and never counts before it's known", () => {
    expect(statLines(scanStats(20, 3, 5), true)).toHaveLength(3);
    expect(statLines(scanStats(20, 3, 5), true)[1]).toMatch(/^B {2}EFW/);
    expect(statLines(scanStats(20, 6, 5), true).at(-1)).toBe("+2 more");
    expect(statLines(scanStats(8, 0, 5), false).some((l) => l.startsWith("CRL"))).toBe(true);
  });

  it("names girls and boys only when told to", () => {
    expect(statLines(scanStats(20, 1, 5), true, ["girl"]).at(-1)).toBe("SEX  ♀ GIRL");
    expect(statLines(scanStats(20, 1, 5), true).some((l) => l.startsWith("SEX"))).toBe(false);
    const twins = statLines(scanStats(20, 2, 5), true, ["girl", "boy"]);
    expect(twins[0].endsWith("♀")).toBe(true);
    expect(twins[1].endsWith("♂")).toBe(true);
  });

  it("writes weights in g or kg", () => {
    expect(weightText(350)).toBe("350 g");
    expect(weightText(1250)).toBe("1.25 kg");
  });
});
