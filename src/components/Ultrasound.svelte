<script lang="ts" module>
  let next = 0;

  /** What the scan screen says about you (worked out fresh for every picture). */
  export interface ScanMeta {
    /** Your name, or null to leave it out. */
    patient: string | null;
    /** One per baby, from the gender week on (null: not shown). */
    genders?: string[] | null;
    timeMs: number;
    dueMs: number | null;
  }
</script>

<script lang="ts">
  import { babyWord } from "../lib/fertility";
  import type { Scan } from "../lib/journal";
  import { rng, scanStats, statLines } from "../lib/scanstats";

  // A made-up ultrasound: toony, grainy and blurry enough that nobody can tell
  // what species is in there. Before the count week (babies = 0) it's one
  // little bean and no markings; after, each baby gets a circle and a letter,
  // like a doctor points them out. Late on they're packed in tight.
  // Everything is drawn with plain SVG attributes (no CSS), so the same picture
  // can be saved or copied with the text stamped in (lib/scanimage.ts).
  let { scan, meta = null, svg = $bindable(null) }: { scan: Scan; meta?: ScanMeta | null; svg?: SVGSVGElement | null } = $props();

  const uid = `us${next++}`;
  const v = $derived(scan.variant);
  const MONO = "Consolas, 'Cascadia Mono', ui-monospace, monospace";
  const ROUND = "'Segoe UI', system-ui, sans-serif";
  // The doctor's scribbles.
  const HAND = "'Comic Sans MS', 'Segoe Print', cursive";
  const scribble = (x: number, y: number, size = 17) => ({ x, y, "font-family": HAND, "font-size": size, "font-weight": 700, fill: "#ffe14d", stroke: "#050505", "stroke-width": 4, "paint-order": "stroke", "text-anchor": "middle" });
  const W = 400;
  const H = 340;

  const LAYOUTS: [number, number][][] = [
    [[200, 160]],
    [[148, 162], [252, 156]],
    [[140, 180], [260, 176], [200, 114]],
    [[148, 124], [252, 124], [148, 200], [252, 200]],
  ];

  const view = $derived.by(() => {
    const r = rng(scan.seed);
    const n = Math.max(0, scan.babies);
    const shown = Math.max(1, n);
    // Late on: the babies fill the space and get pushed together.
    const packed = Math.min(1, Math.max(0, (scan.week - 26) / 12));
    const squeeze = 1 - packed * 0.22;
    const spots: [number, number][] = (
      LAYOUTS[shown - 1] ??
      Array.from({ length: shown }, (_, i) => {
        const a = (i / shown) * Math.PI * 2 - Math.PI / 2;
        return [200 + Math.cos(a) * 82, 162 + Math.sin(a) * 66] as [number, number];
      })
    ).map(([x, y]) => [200 + (x - 200) * squeeze, 160 + (y - 160) * squeeze]);
    const sac = (shown === 1 ? 72 : shown === 2 ? 54 : shown <= 4 ? 42 : 32) * (1 - packed * 0.12);
    const grow = Math.min(1, Math.max(0, (scan.week - 6) / 18));
    const u = sac * (0.24 + 0.5 * grow + 0.42 * packed);
    const babies = spots.map(([x, y]) => ({
      x: x + (r() - 0.5) * 14 * (1 - packed),
      y: y + (r() - 0.5) * 12 * (1 - packed),
      rot: (r() - 0.5) * 120,
      flip: r() < 0.5 ? -1 : 1,
      sx: 0.9 + r() * 0.25,
    }));
    // Soft tissue around the sacs.
    const tissue = Array.from({ length: 7 }, () => ({ x: 60 + r() * 280, y: 60 + r() * 200, rx: 30 + r() * 60, ry: 10 + r() * 22, rot: (r() - 0.5) * 70, o: 0.18 + r() * 0.25 }));
    const days = scan.seed % 7;
    const lines = statLines(scanStats(scan.week, n, scan.seed), n > 0, meta?.genders ?? null);
    // Where the heart-shaped shadow sits (the "heart" scan).
    const heart = { x: 120 + r() * 160, y: 215 + r() * 25 };
    return { n, sac, u, babies, tissue, packed, early: scan.week < 10, turb: Math.floor(r() * 1000), days, lines, heart };
  });

  const LETTERS = "ABCDEFGH";
  // The fan: apex at the top, opening 36 degrees each way.
  const FAN = "M200,14 L41,233 A270,270 0 0 0 359,233 Z";
  const dateFmt = new Intl.DateTimeFormat(undefined, { year: "numeric", month: "2-digit", day: "2-digit" });
  const timeFmt = new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit" });
  const text = (x: number, y: number) => ({ x, y, "font-family": MONO, "font-size": 10.5, "font-weight": 700, fill: "#e6e6e6", "letter-spacing": 0.6 });
</script>

<svg bind:this={svg} xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width={W} height={H} class="scan" role="img"
  aria-label={view.n ? `Ultrasound scan, week ${scan.week}: ${babyWord(view.n)}` : `Ultrasound scan, week ${scan.week}`}>
  <defs>
    <clipPath id="{uid}-fan"><path d={FAN} /></clipPath>
    <radialGradient id="{uid}-bg" cx="200" cy="14" r="290" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#5a5a5a" />
      <stop offset="0.25" stop-color="#2c2c2c" />
      <stop offset="0.75" stop-color="#232323" />
      <stop offset="1" stop-color="#3b3b3b" />
    </radialGradient>
    <radialGradient id="{uid}-bean" cx="0.4" cy="0.35" r="0.75">
      <stop offset="0" stop-color="#f4f4f4" />
      <stop offset="0.6" stop-color="#bdbdbd" />
      <stop offset="1" stop-color="#8a8a8a" />
    </radialGradient>
    <filter id="{uid}-soft" x="-20%" y="-20%" width="140%" height="140%">
      <feTurbulence type="fractalNoise" baseFrequency="0.035" numOctaves="2" seed={view.turb} result="warp" />
      <feDisplacementMap in="SourceGraphic" in2="warp" scale="14" xChannelSelector="R" yChannelSelector="G" result="wobbly" />
      <feGaussianBlur in="wobbly" stdDeviation="2.6" />
    </filter>
    <filter id="{uid}-haze"><feGaussianBlur stdDeviation="9" /></filter>
    <filter id="{uid}-grain" x="0" y="0" width="100%" height="100%">
      <feTurbulence type="fractalNoise" baseFrequency="0.85" numOctaves="2" seed={view.turb + 1} />
      <feColorMatrix values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  0 0 0 0.9 -0.32" />
    </filter>
  </defs>

  <rect width={W} height={H} fill="#050505" />

  <g clip-path="url(#{uid}-fan)">
    <rect width={W} height={H} fill="url(#{uid}-bg)" />
    <g filter="url(#{uid}-haze)">
      {#each view.tissue as t}
        <ellipse cx={t.x} cy={t.y} rx={t.rx} ry={t.ry} fill="#9a9a9a" opacity={t.o} transform="rotate({t.rot} {t.x} {t.y})" />
      {/each}
    </g>

    <g filter="url(#{uid}-soft)">
      {#each view.babies as b}
        <!-- The sac: dark water with a bright wall (thicker when it's a squeeze). -->
        <ellipse cx={b.x} cy={b.y} rx={view.sac * 1.08} ry={view.sac * 0.9} fill="#070707" stroke="#b9b9b9" stroke-width={7 + view.packed * 6} stroke-opacity="0.75" />
      {/each}
      {#if v === "heart"}
        <path d="M{view.heart.x},{view.heart.y + 14} c-26,-18 -22,-38 -6,-38 c4,0 6,3 6,6 c0,-3 2,-6 6,-6 c16,0 20,20 -6,38 z" fill="#d6d6d6" opacity="0.85" />
      {/if}
      {#if v === "highfive" && view.babies.length > 1}
        {@const mx = (view.babies[0].x + view.babies[1].x) / 2}
        {@const my = (view.babies[0].y + view.babies[1].y) / 2 - 4}
        <ellipse cx={mx - 7} cy={my} rx={view.u * 0.22} ry={view.u * 0.13} fill="#e0e0e0" transform="rotate(-20 {mx - 7} {my})" />
        <ellipse cx={mx + 7} cy={my} rx={view.u * 0.22} ry={view.u * 0.13} fill="#e0e0e0" transform="rotate(20 {mx + 7} {my})" />
      {/if}
      {#each view.babies as b, i}
        <g transform="translate({b.x} {b.y}) rotate({b.rot}) scale({b.flip * b.sx} 1)">
          {#if view.early}
            <!-- Just a little bean this early. -->
            <ellipse cx="0" cy="0" rx={view.u * 0.75} ry={view.u * 0.5} fill="url(#{uid}-bean)" />
          {:else}
            <!-- Curled up: a big round head and a soft body, nothing else to tell by. -->
            <ellipse cx={view.u * 0.15} cy={view.u * 0.32} rx={view.u * 0.5} ry={view.u * 0.62} fill="url(#{uid}-bean)" transform="rotate(18 {view.u * 0.15} {view.u * 0.32})" />
            <circle cx={-view.u * 0.12} cy={-view.u * 0.5} r={view.u * 0.46} fill="url(#{uid}-bean)" />
            <ellipse cx={-view.u * 0.38} cy={view.u * 0.15} rx={view.u * 0.2} ry={view.u * 0.11} fill="#cfcfcf" transform="rotate(-30 {-view.u * 0.38} {view.u * 0.15})" />
            {#if i === 0 && v === "wave"}
              <!-- A little hand up. -->
              <ellipse cx={-view.u * 0.72} cy={-view.u * 0.72} rx={view.u * 0.13} ry={view.u * 0.3} fill="#e4e4e4" transform="rotate(-35 {-view.u * 0.72} {-view.u * 0.72})" />
            {:else if i === 0 && v === "yawn"}
              <ellipse cx={-view.u * 0.34} cy={-view.u * 0.36} rx={view.u * 0.13} ry={view.u * 0.16} fill="#111" />
            {/if}
          {/if}
        </g>
      {/each}
    </g>

    <rect width={W} height={H} filter="url(#{uid}-grain)" opacity="0.55" />
  </g>

  <!-- Depth ticks down the side. -->
  <g stroke="#d8d8d8" stroke-width="1.5" stroke-linecap="round">
    {#each Array.from({ length: 9 }, (_, i) => i) as i}
      <line x1="384" x2={i % 2 ? 380 : 376} y1={52 + i * 24} y2={52 + i * 24} />
    {/each}
  </g>

  <!-- The screen's text, like a real one: clinic, patient and dates on top. -->
  <text {...text(10, 16)}>PREG·OSC CLINIC</text>
  {#if meta}
    <text {...text(10, 30)} fill="#bdbdbd">{meta.patient ? meta.patient.toUpperCase().slice(0, 22) : "PATIENT"}</text>
    <text {...text(10, 44)} fill="#9a9a9a">{dateFmt.format(meta.timeMs)} {timeFmt.format(meta.timeMs)}</text>
  {/if}
  <text {...text(390, 16)} text-anchor="end">OB · C5-1</text>
  <text {...text(390, 30)} text-anchor="end" fill="#bdbdbd">GA {scan.week}w{view.days}d</text>
  {#if meta?.dueMs}<text {...text(390, 44)} text-anchor="end" fill="#9a9a9a">EDD {dateFmt.format(meta.dueMs)}</text>{/if}

  <!-- The measurement box, bottom left. -->
  <g>
    {#each view.lines as l, i}
      <text {...text(10, H - 10 - (view.lines.length - 1 - i) * 13)} fill="#f3f3f3">{l}</text>
    {/each}
  </g>
  <text {...text(390, H - 23)} text-anchor="end" fill="#9a9a9a">MI 0.8  TIs 0.2</text>
  <text {...text(390, H - 10)} text-anchor="end">{view.n ? babyWord(view.n).replace("!", "").toUpperCase() : "1/1"}</text>

  <!-- What the doctor scribbled on the rare ones. -->
  {#if v}
    {@const f = view.babies[0]}
    {@const rad = Math.max(view.u * 1.05, 18)}
    {#if v === "wave"}<text {...scribble(f.x - rad - 8, f.y - rad - 6)}>HI! 👋</text>
    {:else if v === "yawn"}<text {...scribble(f.x - rad - 4, f.y - rad - 4)}>yawn!</text>
    {:else if v === "shy"}<text {...scribble(f.x, f.y - rad - 10)}>camera shy, not today!</text>
    {:else if v === "hiccups"}
      <text {...scribble(f.x - rad - 6, f.y - rad * 0.6, 15)}>hic!</text>
      <text {...scribble(f.x + rad + 8, f.y - rad * 0.2, 13)}>hic!</text>
    {:else if v === "heart"}
      <circle cx={view.heart.x} cy={view.heart.y} r="30" fill="none" stroke="#ff7eb6" stroke-width="2.2" stroke-dasharray="6 5" />
      <text {...scribble(view.heart.x + 44, view.heart.y - 22)} fill="#ff9ccb">♥?</text>
    {:else if v === "highfive"}<text {...scribble(200, 76)}>high five!</text>
    {/if}
  {/if}

  {#if view.n}
    {#each view.babies as b, i}
      {@const rad = Math.max(view.u * 1.05, 18)}
      <circle cx={b.x} cy={b.y} r={rad} fill="none" stroke="#ffe14d" stroke-width="2.2" stroke-dasharray="6 5" stroke-linecap="round" />
      {#if view.n > 1}
        <text x={b.x - rad * 0.72} y={b.y - rad * 0.72} text-anchor="middle" font-family={ROUND} font-size="15" font-weight="900" fill="#ffe14d" stroke="#050505" stroke-width="4" paint-order="stroke">{LETTERS[i] ?? i + 1}</text>
      {/if}
    {/each}
    {@const f = view.babies[0]}
    {@const len = Math.max(view.u * 0.9, 12)}
    <!-- A measuring line across the first one, with calipers. -->
    <g stroke="#7fe3ff" stroke-width="1.6" fill="none" stroke-linecap="round">
      <line x1={f.x - len} y1={f.y + len * 0.25} x2={f.x + len} y2={f.y - len * 0.25} stroke-dasharray="2 4" />
      <path d="M{f.x - len - 4},{f.y + len * 0.25} h8 m-4,-4 v8 M{f.x + len - 4},{f.y - len * 0.25} h8 m-4,-4 v8" />
    </g>
  {/if}
</svg>

<style>
  .scan {
    display: block;
    width: 100%;
    max-width: 420px;
    height: auto;
    border-radius: 14px;
    background: #050505;
  }
</style>
