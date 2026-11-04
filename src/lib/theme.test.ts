import { describe, expect, it } from "vitest";
import { CUSTOM_VARS, contrast, customColors, hexToHsv, hslToHex, hsvToHex, ownIsDark } from "./theme";

describe("custom colors", () => {
  it("sets the whole theme, not just the accent", () => {
    const c = customColors("#7fb8f5", null, "light");
    expect(Object.keys(c).sort()).toEqual([...CUSTOM_VARS].sort());
    expect(c["--accent"]).toBe("#7fb8f5");
    // Bluish, nothing left of a pink preset.
    const n = parseInt(c["--bg-3"].slice(1), 16);
    expect(n & 255).toBeGreaterThan((n >> 16) & 255);
  });

  it("black mode only changes the accent", () => {
    expect(Object.keys(customColors("#7fb8f5", "#9ccf6f", "black")).sort()).toEqual(["--accent", "--accent-2", "--accent-text"]);
  });

  it("your own background is used exactly, and sets light or dark", () => {
    expect(customColors("#7fb8f5", "#123456", "light")["--bg"]).toBe("#123456");
    expect(customColors("#7fb8f5", "#ffffff", "dark")["--bg"]).toBe("#ffffff");
    expect(ownIsDark("#000000")).toBe(true);
    expect(ownIsDark("#123456")).toBe(true);
    expect(ownIsDark("#ffffff")).toBe(false);
    expect(ownIsDark("#ffee88")).toBe(false);
    expect(ownIsDark(null)).toBeNull();
  });

  it("text stays readable on any background at all", () => {
    for (let h = 0; h < 360; h += 15) {
      for (const s of [0, 25, 50, 75, 100]) {
        for (let l = 0; l <= 100; l += 2) {
          const c = customColors("#7fb8f5", hslToHex(h, s, l), "light");
          const at = `hsl ${h} ${s} ${l}`;
          for (const bg of ["--bg", "--bg-2", "--bg-3"]) {
            expect(contrast(c["--text"], c[bg]), `text on ${bg}, ${at}`).toBeGreaterThanOrEqual(4.5);
          }
          for (const bg of ["--bg", "--bg-2"]) {
            expect(contrast(c["--muted"], c[bg]), `muted on ${bg}, ${at}`).toBeGreaterThanOrEqual(3);
          }
        }
      }
    }
  });

  it("following the accent stays readable in light and dark, with no tint from a gray", () => {
    for (let h = 0; h < 360; h += 15) {
      for (const mode of ["light", "dark"] as const) {
        const c = customColors(hslToHex(h, 80, 60), null, mode);
        for (const bg of ["--bg", "--bg-2", "--bg-3"]) {
          expect(contrast(c["--text"], c[bg]), `text on ${bg}, hue ${h}, ${mode}`).toBeGreaterThanOrEqual(4.5);
        }
      }
    }
    const channels = (hex: string) => [hex.slice(1, 3), hex.slice(3, 5), hex.slice(5, 7)];
    const [r, g, b] = channels(customColors("#808080", null, "light")["--bg-3"]);
    expect(r === g && g === b).toBe(true);
  });

  it("button text reads on any accent", () => {
    for (let h = 0; h < 360; h += 15) {
      for (const s of [0, 50, 100]) {
        for (let l = 0; l <= 100; l += 2) {
          const c = customColors(hslToHex(h, s, l), null, "light");
          expect(contrast(c["--accent"], c["--accent-text"]), `hsl ${h} ${s} ${l}`).toBeGreaterThanOrEqual(3);
        }
      }
    }
  });

  it("the wheel's colors go there and back", () => {
    expect(hsvToHex(0, 1, 1)).toBe("#ff0000");
    expect(hsvToHex(120, 1, 1)).toBe("#00ff00");
    expect(hsvToHex(240, 1, 0.5)).toBe("#000080");
    expect(hsvToHex(200, 0, 1)).toBe("#ffffff");
    expect(hsvToHex(200, 1, 0)).toBe("#000000");
    for (const hex of ["#f59ac0", "#123456", "#9ccf6f", "#808080", "#ffffff", "#000000"]) {
      const { h, s, v } = hexToHsv(hex);
      expect(hsvToHex(h, s, v)).toBe(hex);
    }
  });
});
