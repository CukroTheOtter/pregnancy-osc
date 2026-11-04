// Custom colors in Settings > Look: a whole theme worked out from your accent
// (and your background, if you pick one). Text and borders always follow, so
// they stay readable.

export const HEX = /^#[0-9a-f]{6}$/i;

const clamp = (x: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, x));

function rgb(hex: string): [number, number, number] {
  const n = parseInt(hex.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

const toHex = (channels: number[]) => "#" + channels.map((x) => Math.round(x * 255).toString(16).padStart(2, "0")).join("");

export function hslToHex(h: number, s: number, l: number): string {
  s /= 100;
  l /= 100;
  const k = (n: number) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
  return toHex([f(0), f(8), f(4)]);
}

/** Hue (0-360), saturation and lightness (0-100). */
function hexToHsl(hex: string): { h: number; s: number; l: number } {
  const [r, g, b] = rgb(hex).map((x) => x / 255);
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const d = max - min;
  let h = 0;
  if (d) h = max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  const l = (max + min) / 2;
  return { h: (h * 60 + 360) % 360, s: d ? (d / (1 - Math.abs(2 * l - 1))) * 100 : 0, l: l * 100 };
}

/** The color wheel's color: hue (0-360), how vivid and how bright (0-1). */
export function hsvToHex(h: number, s: number, v: number): string {
  const f = (n: number) => {
    const k = (n + h / 60) % 6;
    return v - v * s * Math.max(0, Math.min(k, 4 - k, 1));
  };
  return toHex([f(5), f(3), f(1)]);
}

export function hexToHsv(hex: string): { h: number; s: number; v: number } {
  const [r, g, b] = rgb(hex).map((x) => x / 255);
  const max = Math.max(r, g, b);
  const d = max - Math.min(r, g, b);
  return { h: hexToHsl(hex).h, s: max ? d / max : 0, v: max };
}

export function contrast(a: string, b: string): number {
  const light = (hex: string) => {
    const [r, g, b] = rgb(hex).map((x) => {
      const c = x / 255;
      return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  };
  const [hi, lo] = [light(a), light(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

const BLACK = "#000000";
const WHITE = "#ffffff";

/** A background of your own picks the app's light or dark look (null: none picked). */
export function ownIsDark(background: string | null | undefined): boolean | null {
  if (!background || !HEX.test(background)) return null;
  return contrast(WHITE, background) >= contrast(BLACK, background);
}

/** Everything from your exact background: cards step away from the text, so it stays readable. */
function ownColors(background: string): Record<string, string> {
  const { h, s, l } = hexToHsl(background);
  const shade = (dl: number, sat = s) => hslToHex(h, sat, clamp(l + dl, 0, 100));
  const dark = ownIsDark(background)!;
  const text = dark ? WHITE : BLACK;
  // Lighter cards, unless that gets too close to the text.
  const up = dark ? contrast(text, shade(8)) >= 4.5 : l < 90;
  const step = up ? 1 : -1;
  const [card, soft] = [shade(4 * step), shade(8 * step)];
  // Muted text: the nearest step towards the text that still reads on the page and cards.
  let muted = text;
  for (let d = 20; d <= 100; d += 5) {
    const c = shade(dark ? d : -d, Math.min(s, 30));
    if (contrast(c, background) >= 3.2 && contrast(c, card) >= 3.2) {
      muted = c;
      break;
    }
  }
  return { "--bg": background, "--bg-2": card, "--bg-3": soft, "--line": shade(dark ? 16 : -14), "--text": text, "--muted": muted };
}

/** How strong a background following the accent is (0 faint, 1 strong). */
const FOLLOW_STRENGTH = 0.35;

/** Every color a custom theme sets. Black mode keeps its true black. */
export const CUSTOM_VARS = ["--accent", "--accent-2", "--accent-text", "--bg", "--bg-2", "--bg-3", "--text", "--muted", "--line"];

export function customColors(accent: string, background: string | null | undefined, mode: "light" | "dark" | "black"): Record<string, string> {
  const a = hexToHsl(accent);
  const plain = a.s < 2 || a.l < 1 || a.l > 99;
  const out: Record<string, string> = {
    "--accent": accent,
    "--accent-2": plain ? hslToHex(0, 0, a.l > 50 ? a.l - 25 : a.l + 25) : hslToHex((a.h + 50) % 360, 85, 80),
    // White on the color when it reads, dark on the pale ones.
    "--accent-text": contrast(WHITE, accent) >= 3 ? WHITE : hslToHex(0, 0, 10),
  };
  if (mode === "black") return out;
  if (background && HEX.test(background)) return { ...out, ...ownColors(background) };
  // Following the accent: a tint of it that matches Light or Dark (no tint from a gray).
  const h = a.h;
  const k = FOLLOW_STRENGTH;
  const tint = plain ? 0 : 1;
  if (mode === "dark") {
    const s = (12 + 28 * k) * tint;
    return {
      ...out,
      "--bg": hslToHex(h, s, 10 + 6 * k),
      "--bg-2": hslToHex(h, s, 14 + 8 * k),
      "--bg-3": hslToHex(h, s, 18 + 10 * k),
      "--line": hslToHex(h, s, 25 + 10 * k),
      "--text": hslToHex(h, 40 * tint, 94),
      "--muted": hslToHex(h, 16 * tint, 68 + 4 * k),
    };
  }
  const s = (30 + 60 * k) * tint;
  return {
    ...out,
    "--bg": hslToHex(h, s, 98 - 10 * k),
    "--bg-2": hslToHex(h, s, 100 - 5 * k),
    "--bg-3": hslToHex(h, s, 95 - 14 * k),
    "--line": hslToHex(h, s * 0.8, 89 - 16 * k),
    "--text": hslToHex(h, 25 * tint, 22),
    "--muted": hslToHex(h, 14 * tint, 46 - 12 * k),
  };
}
