// The Share picture on the Me tab: drawn here, in the app's own theme, and only
// leaves the PC if you copy or save it. Nothing about other people goes on it.

import { ACHIEVEMENTS, TIERS, type AchievementInfo, type Tier } from "./achievements";

export interface ShareInput {
  name: string;
  /** A data URL, or null for the 🤰 placeholder. */
  picture: string | null;
  /** Unlocked achievement ids. */
  unlocked: string[];
  stats: { label: string; value: number }[];
}

const WIDTH = 440;
const SCALE = 2;
const PAD = 22;
const ROW_H = 72;
const ROW_GAP = 7;
const CHIP = 34;
const CHIP_GAP = 6;
/** Your rarest get a big row each; the rest are small icons. */
const TOP_ROWS = 3;
/** Where to get the app (it's only text: a picture can't hold a link you can click). */
const DOWNLOAD = "github.com/CukroTheOtter/pregnancy-osc/releases";

/** Your rarest achievements as rows, and everything else you've unlocked as icons, rarest first. */
export function splitAchievements(unlocked: string[], all: AchievementInfo[] = ACHIEVEMENTS): { rows: AchievementInfo[]; chips: AchievementInfo[] } {
  const mine = all.filter((a) => unlocked.includes(a.id)).sort((a, b) => b.tier - a.tier);
  return { rows: mine.slice(0, TOP_ROWS), chips: mine.slice(TOP_ROWS) };
}

/** How many rows of small icons it takes. */
export function chipRows(chips: number): number {
  const perRow = Math.floor((WIDTH - PAD * 2 + CHIP_GAP) / (CHIP + CHIP_GAP));
  return Math.ceil(chips / perRow);
}

function shape(c: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  c.beginPath();
  c.moveTo(x + r, y);
  c.arcTo(x + w, y, x + w, y + h, r);
  c.arcTo(x + w, y + h, x, y + h, r);
  c.arcTo(x, y + h, x, y, r);
  c.arcTo(x, y, x + w, y, r);
  c.closePath();
}

/** Cuts text to fit, with an ellipsis. */
function fit(c: CanvasRenderingContext2D, text: string, width: number): string {
  if (c.measureText(text).width <= width) return text;
  let out = text;
  while (out.length > 1 && c.measureText(`${out}…`).width > width) out = out.slice(0, -1);
  return `${out}…`;
}

function loadImage(src: string): Promise<HTMLImageElement | null> {
  return new Promise((resolve) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => resolve(null);
    img.src = src;
  });
}

export async function drawShareCard(input: ShareInput): Promise<Blob> {
  const css = getComputedStyle(document.documentElement);
  const v = (name: string) => css.getPropertyValue(name).trim();
  const t = { bg: v("--bg"), card: v("--bg-2"), soft: v("--bg-3"), text: v("--text"), muted: v("--muted"), line: v("--line"), accent: v("--accent") };
  const tierColor: Record<Tier, string> = { 1: t.muted, 2: v("--tier-uncommon"), 3: v("--tier-rare"), 4: v("--tier-epic"), 5: v("--tier-legendary") };
  const font = getComputedStyle(document.body).fontFamily;
  await document.fonts?.ready;

  const { rows, chips } = splitAchievements(input.unlocked);
  const chipLines = chipRows(chips.length);
  const rowsH = rows.length ? rows.length * ROW_H + (rows.length - 1) * ROW_GAP : 0;
  const chipsH = chipLines ? chipLines * CHIP + (chipLines - 1) * CHIP_GAP + (rows.length ? 12 : 0) : 0;
  const height = PAD + 68 + 14 + 8 + 16 + 62 + 16 + (rowsH + chipsH || 20) + 14 + 14 + PAD;

  const canvas = document.createElement("canvas");
  canvas.width = WIDTH * SCALE;
  canvas.height = height * SCALE;
  const c = canvas.getContext("2d")!;
  c.scale(SCALE, SCALE);
  c.textBaseline = "alphabetic";

  // The card.
  shape(c, 0, 0, WIDTH, height, 22);
  c.fillStyle = t.card;
  c.fill();
  c.save();
  c.clip();
  c.strokeStyle = t.line;
  c.lineWidth = 2;
  shape(c, 0, 0, WIDTH, height, 22);
  c.stroke();
  c.restore();

  // Picture, name and count.
  let y = PAD;
  const cx = PAD + 34;
  const cy = y + 34;
  c.fillStyle = t.soft;
  c.beginPath();
  c.arc(cx, cy, 34, 0, Math.PI * 2);
  c.fill();
  const img = input.picture ? await loadImage(input.picture) : null;
  if (img) {
    c.save();
    c.clip();
    c.drawImage(img, PAD, y, 68, 68);
    c.restore();
  } else {
    c.font = `32px ${font}`;
    c.textAlign = "center";
    c.fillStyle = t.text;
    c.fillText("🤰", cx, cy + 12);
  }
  c.strokeStyle = t.accent;
  c.lineWidth = 3;
  c.beginPath();
  c.arc(cx, cy, 34, 0, Math.PI * 2);
  c.stroke();

  const tx = PAD + 68 + 14;
  c.textAlign = "left";
  c.fillStyle = t.text;
  c.font = `800 22px ${font}`;
  c.fillText(fit(c, input.name, WIDTH - tx - PAD), tx, cy - 2);
  c.fillStyle = t.muted;
  c.font = `600 13px ${font}`;
  c.fillText(`🏆 ${input.unlocked.length} of ${ACHIEVEMENTS.length} achievements`, tx, cy + 19);
  y += 68 + 14;

  // Progress.
  const barW = WIDTH - PAD * 2;
  shape(c, PAD, y, barW, 8, 4);
  c.fillStyle = t.soft;
  c.fill();
  if (input.unlocked.length) {
    shape(c, PAD, y, Math.max(8, (barW * input.unlocked.length) / ACHIEVEMENTS.length), 8, 4);
    c.fillStyle = t.accent;
    c.fill();
  }
  y += 8 + 16;

  // Stats.
  const gap = 8;
  const statW = (barW - gap * (input.stats.length - 1)) / input.stats.length;
  input.stats.forEach((s, i) => {
    const x = PAD + i * (statW + gap);
    shape(c, x, y, statW, 62, 14);
    c.fillStyle = t.bg;
    c.fill();
    c.textAlign = "center";
    c.fillStyle = t.text;
    c.font = `800 20px ${font}`;
    c.fillText(String(s.value), x + statW / 2, y + 30);
    c.fillStyle = t.muted;
    c.font = `600 11px ${font}`;
    c.fillText(s.label, x + statW / 2, y + 49);
  });
  y += 62 + 16;

  // Your rarest, as rows in their rarity's color.
  for (const a of rows) {
    const color = tierColor[a.tier];
    shape(c, PAD, y, barW, ROW_H, 14);
    c.fillStyle = t.bg;
    c.fill();
    c.save();
    c.globalAlpha = 0.14;
    c.fillStyle = color;
    c.fill();
    c.restore();
    c.strokeStyle = color;
    c.lineWidth = 1;
    c.stroke();
    c.textAlign = "center";
    c.fillStyle = t.text;
    c.font = `24px ${font}`;
    c.fillText(a.emoji, PAD + 26, y + 45);
    const stars = "⭐".repeat(a.tier);
    c.font = `12px ${font}`;
    const starsW = c.measureText(stars).width;
    c.textAlign = "right";
    c.fillText(stars, PAD + barW - 12, y + 25);
    c.textAlign = "left";
    c.font = `800 14px ${font}`;
    c.fillText(fit(c, a.title, barW - 66 - starsW - 12), PAD + 50, y + 23);
    c.fillStyle = color;
    c.font = `700 11px ${font}`;
    c.fillText(TIERS[a.tier], PAD + 50, y + 39);
    // What you did to get it.
    c.fillStyle = t.muted;
    c.font = `600 11px ${font}`;
    c.fillText(fit(c, a.hint, barW - 62), PAD + 50, y + 57);
    y += ROW_H + ROW_GAP;
  }
  if (rows.length) y -= ROW_GAP;

  // Everything else, as small icons.
  if (chips.length) {
    if (rows.length) y += 12;
    const perRow = Math.floor((barW + CHIP_GAP) / (CHIP + CHIP_GAP));
    chips.forEach((a, i) => {
      const x = PAD + (i % perRow) * (CHIP + CHIP_GAP);
      const yy = y + Math.floor(i / perRow) * (CHIP + CHIP_GAP);
      shape(c, x, yy, CHIP, CHIP, 10);
      c.fillStyle = t.bg;
      c.fill();
      c.strokeStyle = tierColor[a.tier];
      c.lineWidth = 1.5;
      c.stroke();
      c.textAlign = "center";
      c.fillStyle = t.text;
      c.font = `18px ${font}`;
      c.fillText(a.emoji, x + CHIP / 2, yy + 24);
    });
    y += chipLines * CHIP + (chipLines - 1) * CHIP_GAP;
  }
  if (!rows.length && !chips.length) {
    c.textAlign = "center";
    c.fillStyle = t.muted;
    c.font = `600 13px ${font}`;
    c.fillText("No achievements yet", WIDTH / 2, y + 14);
    y += 20;
  }

  // Footer.
  c.fillStyle = t.muted;
  c.font = `700 11px ${font}`;
  c.textAlign = "left";
  c.fillText(DOWNLOAD, PAD, y + 14 + 11);
  c.textAlign = "right";
  c.fillText("💕 Pregnancy OSC", WIDTH - PAD, y + 14 + 11);

  return new Promise((resolve, reject) => canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("Couldn't make the picture."))), "image/png"));
}
