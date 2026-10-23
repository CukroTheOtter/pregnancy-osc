// Baby sprite parts as SVG path data. Swap these for better art any time:
// each part is drawn in a 120x120 box centered on the baby (0,0).
//
// Coordinates: head center (0,-14) r=26, body center (0,18).
// Ears are drawn for the RIGHT side and mirrored for the left.
// "main" paths use the fur color, "inner" paths a light accent, "dark" a shade.

export interface Part {
  main?: string[];
  inner?: string[];
  dark?: string[];
  /** Stroke-only paths in the fur color (thin tails). */
  stroke?: string[];
  /** Where the right-side part attaches (left side is mirrored). */
  at: { x: number; y: number; r: number };
  /** Draw behind the head (true) or in front (false). */
  behind?: boolean;
}

export const EARS: Record<string, Part> = {
  human: { at: { x: 25, y: -14, r: 0 }, main: ["M-2 -7 C6 -9 9 -2 8 3 C7 8 1 9 -2 7 Z"], inner: ["M0 -3 C4 -4 5 0 4 2 C3 4 1 4 0 3 Z"] },
  fox: { at: { x: 15, y: -33, r: 18 }, behind: true, main: ["M-10 6 Q-7 -20 3 -32 Q12 -14 10 6 Z"], inner: ["M-5 3 Q-3 -13 3 -22 Q7 -10 6 3 Z"] },
  wolf: { at: { x: 16, y: -32, r: 22 }, behind: true, main: ["M-11 6 Q-6 -16 4 -26 Q13 -10 11 6 Z"], inner: ["M-6 3 Q-3 -10 3 -18 Q8 -7 7 3 Z"] },
  cat: { at: { x: 16, y: -32, r: 20 }, behind: true, main: ["M-11 6 Q-4 -10 2 -20 Q11 -8 11 6 Z"], inner: ["M-6 3 Q-2 -6 2 -13 Q7 -5 7 3 Z"] },
  bunny: { at: { x: 11, y: -36, r: 10 }, behind: true, main: ["M-7 6 C-12 -18 -9 -42 0 -45 C9 -42 12 -18 7 6 Z"], inner: ["M-3 2 C-6 -16 -4 -34 0 -37 C4 -34 6 -16 3 2 Z"] },
  dragon: {
    at: { x: 14, y: -34, r: 25 },
    behind: true,
    dark: ["M-4 6 Q-3 -12 10 -24 Q5 -8 6 6 Z"],
    main: ["M10 8 Q20 2 24 -6 Q14 -4 6 2 Z"],
  },
  deer: {
    at: { x: 22, y: -26, r: 55 },
    behind: true,
    main: ["M-6 4 Q-8 -10 2 -20 Q10 -8 6 4 Z"],
    inner: ["M-3 2 Q-4 -7 2 -14 Q6 -5 3 2 Z"],
    dark: ["M-14 -8 Q-16 -18 -12 -26 Q-10 -18 -10 -8 Z"],
  },
  bird: { at: { x: 12, y: -36, r: 12 }, behind: true, main: ["M-4 6 Q-8 -8 -4 -20 Q0 -10 2 -18 Q6 -8 5 6 Z"] },
  shark: { at: { x: 0, y: -38, r: 0 }, behind: true, main: ["M-10 8 Q-2 -6 10 -16 Q8 -2 10 8 Z"] },
  lizard: { at: { x: 22, y: -26, r: 50 }, behind: true, main: ["M-6 6 L-4 -8 L0 2 L3 -10 L5 2 L9 -6 L8 6 Z"], inner: ["M-4 5 L-3 -3 L0 4 L3 -5 L5 4 L7 -2 L6 5 Z"] },
  bear: { at: { x: 18, y: -32, r: 0 }, behind: true, main: ["M0 -10 A10 10 0 1 1 -0.1 -10 Z"], inner: ["M0 -5 A5 5 0 1 1 -0.1 -5 Z"] },
  mouse: { at: { x: 21, y: -30, r: 0 }, behind: true, main: ["M0 -14 A14 14 0 1 1 -0.1 -14 Z"], inner: ["M0 -8 A8 8 0 1 1 -0.1 -8 Z"] },
  horse: { at: { x: 15, y: -34, r: 15 }, behind: true, main: ["M-6 6 Q-9 -10 1 -22 Q9 -8 6 6 Z"], inner: ["M-3 3 Q-5 -7 1 -15 Q5 -6 3 3 Z"] },
  bat: { at: { x: 18, y: -32, r: 28 }, behind: true, main: ["M-13 6 Q-10 -22 6 -36 Q16 -14 12 6 Z"], inner: ["M-7 3 Q-5 -14 5 -26 Q10 -10 7 3 Z"] },
};

export const TAILS: Record<string, Part> = {
  none: { at: { x: 0, y: 0, r: 0 } },
  fox: { at: { x: 22, y: 30, r: 0 }, behind: true, main: ["M0 0 C14 -2 30 -14 34 -30 C40 -18 36 4 18 12 C10 15 2 10 0 0 Z"], inner: ["M34 -30 C40 -18 38 -8 32 -2 C30 -10 31 -22 34 -30 Z"] },
  wolf: { at: { x: 22, y: 30, r: 0 }, behind: true, main: ["M0 0 C12 0 26 -8 32 -22 C36 -10 32 6 16 12 C8 14 2 8 0 0 Z"], dark: ["M32 -22 C36 -12 34 -4 29 0 C28 -8 29 -16 32 -22 Z"] },
  cat: { at: { x: 22, y: 32, r: 0 }, behind: true, stroke: ["M0 0 C14 4 26 0 28 -12 C30 -22 22 -28 18 -22"] },
  bunny: { at: { x: 24, y: 32, r: 0 }, behind: true, main: ["M8 0 A8 8 0 1 1 7.9 0 Z"], inner: ["M8 3 A4 4 0 1 1 7.9 3 Z"] },
  dragon: { at: { x: 22, y: 30, r: 0 }, behind: true, main: ["M0 -4 C16 -2 28 -10 34 -24 C36 -10 26 6 2 8 Z"], dark: ["M34 -24 L42 -30 L38 -18 Z"] },
  deer: { at: { x: 24, y: 28, r: 0 }, behind: true, main: ["M0 0 Q8 -12 14 -10 Q12 0 2 6 Z"], inner: ["M3 1 Q8 -7 11 -7 Q9 -1 4 3 Z"] },
  bird: { at: { x: 22, y: 30, r: 0 }, behind: true, main: ["M0 0 L22 -12 L18 -2 L28 -2 L18 6 L24 14 L4 8 Z"] },
  shark: { at: { x: 22, y: 30, r: 0 }, behind: true, main: ["M0 -2 C10 -2 18 -4 22 -8 L32 -20 L28 -4 L34 10 L22 2 C16 6 6 8 0 6 Z"] },
  lizard: { at: { x: 22, y: 30, r: 0 }, behind: true, main: ["M0 -4 C14 -4 30 -6 40 -18 C38 -6 24 8 0 6 Z"], inner: ["M4 -1 C14 -1 26 -3 34 -11 C30 -3 20 3 4 3 Z"] },
  bear: { at: { x: 26, y: 30, r: 0 }, behind: true, main: ["M5 0 A5 5 0 1 1 4.9 0 Z"] },
  mouse: { at: { x: 22, y: 32, r: 0 }, behind: true, stroke: ["M0 0 C18 6 30 -6 36 4 C40 12 34 18 28 14"] },
  horse: { at: { x: 22, y: 26, r: 0 }, behind: true, dark: ["M0 0 C10 -4 20 2 24 14 C18 10 12 12 8 18 C8 10 4 6 0 6 Z"] },
};

export const WINGS: Record<string, Part> = {
  none: { at: { x: 0, y: 0, r: 0 } },
  feathered: { at: { x: 24, y: 4, r: 0 }, behind: true, main: ["M0 0 C10 -16 26 -20 34 -14 C30 -10 32 -6 28 -4 C32 0 30 4 24 4 C26 8 22 12 14 10 C8 10 2 6 0 0 Z"] },
  leathery: { at: { x: 24, y: 4, r: 0 }, behind: true, dark: ["M0 0 L28 -20 L30 -8 L36 -6 L30 2 L32 10 L22 6 L14 12 Z"] },
};

/** Little egg shape for egg mode (centered at 0,0, about 40 tall). */
export const EGG_PATH = "M0 -22 C12 -22 18 -6 18 6 C18 16 10 22 0 22 C-10 22 -18 16 -18 6 C-18 -6 -12 -22 0 -22 Z";

/** Front-view torso for the x-ray belly (400x480 box, centered on x=200). */
export const BODY = {
  // Outline without the bump; the bump is drawn separately so it can grow.
  torso:
    "M176 6 L224 6 L227 46 C262 54 292 68 302 98 C308 128 292 168 264 204 C256 240 264 300 282 360 C294 400 292 440 286 480 L114 480 C108 440 106 400 118 360 C136 300 144 240 136 204 C108 168 92 128 98 98 C108 68 138 54 173 46 Z",
  /** Center of the bump. */
  belly: { x: 200, y: 318 },
  /** Bump radius from flat to full term. */
  minR: 62,
  maxR: 150,
};
