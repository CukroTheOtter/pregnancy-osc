// Made-up readouts for the made-up scans, like a real ultrasound screen shows:
// CRL (crown to rump) early on, then BPD (head width), FL (thigh bone), EFW
// (estimated weight), the heart rate and, late on, which way baby is lying.
// Roughly real growth charts, with a little wobble from the scan's seed.

/** [week, value] points; values in between are drawn as straight lines. */
type Curve = [number, number][];

const CRL_MM: Curve = [[6, 5], [7, 10], [8, 16], [9, 23], [10, 31], [11, 41], [12, 54], [13, 67], [14, 80]];
const BPD_MM: Curve = [[14, 26], [16, 35], [20, 47], [24, 60], [28, 71], [32, 81], [36, 88], [40, 94]];
const FL_MM: Curve = [[14, 14], [16, 21], [20, 33], [24, 44], [28, 54], [32, 62], [36, 69], [40, 75]];
const EFW_G: Curve = [[12, 14], [14, 43], [16, 100], [18, 190], [20, 300], [22, 430], [24, 600], [26, 760], [28, 1000], [30, 1300], [32, 1700], [34, 2100], [36, 2600], [38, 3100], [40, 3400]];
const FHR_BPM: Curve = [[6, 110], [7, 130], [8, 150], [9, 170], [10, 165], [14, 158], [20, 150], [30, 145], [40, 140]];

/** CRL is measured up to here; after, head, thigh and weight. */
export const CRL_UNTIL = 14;

export function curve(c: Curve, week: number): number {
  if (week <= c[0][0]) return c[0][1];
  for (let i = 1; i < c.length; i++) {
    const [w1, v1] = c[i];
    if (week <= w1) {
      const [w0, v0] = c[i - 1];
      return v0 + ((v1 - v0) * (week - w0)) / (w1 - w0);
    }
  }
  return c[c.length - 1][1];
}

/** Seeded 0..1 numbers, so a scan always says the same thing. */
export function rng(seed: number) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export interface BabyStats {
  crlMm: number | null;
  bpdMm: number | null;
  flMm: number | null;
  efwG: number | null;
  fhrBpm: number;
  /** Head down or bottom first (from week 28). */
  lie: "Cephalic" | "Breech" | null;
}

/** Readouts for each baby on a scan (babies = 0: one, before the count is known). */
export function scanStats(week: number, babies: number, seed: number): BabyStats[] {
  const r = rng(seed ^ 0x5eed);
  const n = Math.max(1, babies);
  // Twins and more run a little smaller late on.
  const small = week >= 24 ? Math.pow(0.95, n - 1) : 1;
  const wobble = (spread: number) => 1 + (r() - 0.5) * 2 * spread;
  return Array.from({ length: n }, () => {
    const late = week >= CRL_UNTIL;
    return {
      crlMm: late ? null : Math.round(curve(CRL_MM, week) * wobble(0.06)),
      bpdMm: late ? Math.round(curve(BPD_MM, week) * wobble(0.04)) : null,
      flMm: late ? Math.round(curve(FL_MM, week) * wobble(0.04)) : null,
      efwG: week >= 12 ? Math.round(curve(EFW_G, week) * small * wobble(0.08)) : null,
      fhrBpm: Math.round(curve(FHR_BPM, week) + (r() - 0.5) * 16),
      lie: week >= 28 ? (r() < 0.8 ? "Cephalic" : "Breech") : null,
    };
  });
}

/** "350 g", "1.2 kg". */
export function weightText(g: number): string {
  return g < 1000 ? `${g} g` : `${(g / 1000).toFixed(g < 10_000 ? 2 : 1)} kg`;
}

const SEX: Record<string, string> = { girl: "♀ GIRL", boy: "♂ BOY" };

/**
 * The lines in the measurement box: per baby (A, B, …) when there's more than one.
 * `genders` (one per baby) only when they're known to be shown.
 */
export function statLines(stats: BabyStats[], counted: boolean, genders: string[] | null = null): string[] {
  const sex = (i: number) => (genders && counted ? SEX[genders[i]] ?? null : null);
  if (stats.length === 1 || !counted) {
    const s = stats[0];
    return [
      s.crlMm != null ? `CRL  ${s.crlMm} mm` : null,
      s.bpdMm != null ? `BPD  ${s.bpdMm} mm` : null,
      s.flMm != null ? `FL   ${s.flMm} mm` : null,
      s.efwG != null ? `EFW  ${weightText(s.efwG)}` : null,
      `FHR  ${s.fhrBpm} bpm`,
      s.lie,
      sex(0) ? `SEX  ${sex(0)}` : null,
    ].filter((x): x is string => !!x);
  }
  const lines = stats.slice(0, 4).map((s, i) => {
    const size = s.efwG != null ? `EFW ${weightText(s.efwG)}` : `CRL ${s.crlMm} mm`;
    const tail = [s.lie && (s.lie === "Cephalic" ? "Ceph" : "Breech"), sex(i)?.slice(0, 1)].filter(Boolean).join(" ");
    return `${"ABCD"[i]}  ${size} · ${s.fhrBpm} bpm${tail ? ` · ${tail}` : ""}`;
  });
  if (stats.length > 4) lines.push(`+${stats.length - 4} more`);
  return lines;
}
