<script lang="ts">
  import Baby from "./Baby.svelte";
  import type { BabyLook } from "../lib/look";
  import { BODY, EGG_PATH } from "../assets/art/parts";
  import { mixColors } from "../lib/species";

  let {
    progress = 0,
    babies = [] as BabyLook[],
    egg = false,
    kick = 0,
    sonar = false,
    hideBaby = false,
    label = "",
  }: {
    progress?: number;
    babies?: BabyLook[];
    egg?: boolean;
    kick?: number;
    sonar?: boolean;
    hideBaby?: boolean;
    label?: string;
  } = $props();

  const uid = Math.random().toString(36).slice(2, 8);
  const p = $derived(Math.max(0, Math.min(1.08, progress)));
  const ease = (t: number) => 1 - Math.pow(1 - Math.min(1, t), 1.6);
  const R = $derived(BODY.minR + (BODY.maxR - BODY.minR) * ease(p) + (p > 1 ? (p - 1) * 120 : 0));
  const cx = BODY.belly.x;
  const cy = $derived(BODY.belly.y + (R - BODY.minR) * 0.12);
  const n = $derived(Math.max(1, babies.length));

  // Babies grow from a speck to filling the womb.
  const growth = $derived(0.3 + 0.7 * Math.min(1, p));
  const perBaby = $derived(n === 1 ? 1 : n === 2 ? 0.66 : n === 3 ? 0.56 : n <= 4 ? 0.48 : 0.4);
  const babyScale = $derived(((R * 0.82) / 62) * growth * perBaby);

  function slots(count: number): [number, number, number][] {
    if (count === 1) return [[0, 0.04, -8]];
    if (count === 2) return [[-0.36, 0.02, -14], [0.36, -0.02, 12]];
    if (count === 3) return [[-0.38, 0.2, -12], [0.38, 0.2, 12], [0, -0.32, 0]];
    if (count === 4) return [[-0.34, -0.3, -10], [0.34, -0.3, 10], [-0.34, 0.32, -6], [0.34, 0.32, 8]];
    return Array.from({ length: count }, (_, i) => {
      const a = (i / count) * Math.PI * 2 - Math.PI / 2;
      return [Math.cos(a) * 0.44, Math.sin(a) * 0.44, (i % 2 ? 1 : -1) * 10] as [number, number, number];
    });
  }

  // Kick: a little bump at a random spot.
  let kicking = $state(false);
  let kickAt = $state({ x: 0, y: 0 });
  let lastKick = 0;
  $effect(() => {
    if (kick !== lastKick && kick > 0) {
      lastKick = kick;
      const a = Math.random() * Math.PI * 2;
      kickAt = { x: Math.cos(a) * 0.75, y: Math.sin(a) * 0.6 };
      kicking = true;
      setTimeout(() => (kicking = false), 700);
    }
  });

  const skin = $derived(sonar ? "#8a8a8a" : "var(--skin)");
  const eggColor = (look: BabyLook) => (sonar ? "#dcdcdc" : mixColors(look.furColor ?? "#f3e6d6", "#ffffff", 0.6));
</script>

<svg viewBox="0 0 400 480" class="xray" class:sonar role="img" aria-label={label || "X-ray belly"}>
  <defs>
    <radialGradient id="womb-{uid}" cx="0.45" cy="0.4" r="0.7">
      <stop offset="0" stop-color={sonar ? "#3a3a3a" : "var(--womb-1)"} />
      <stop offset="1" stop-color={sonar ? "#151515" : "var(--womb-2)"} />
    </radialGradient>
    <radialGradient id="glow-{uid}" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0" stop-color={sonar ? "#fff" : "var(--accent)"} stop-opacity="0.25" />
      <stop offset="1" stop-color={sonar ? "#fff" : "var(--accent)"} stop-opacity="0" />
    </radialGradient>
    <clipPath id="clip-{uid}">
      <ellipse {cx} {cy} rx={R * 0.86} ry={R * 0.8} />
    </clipPath>
  </defs>

  <circle cx="200" cy="300" r="190" fill="url(#glow-{uid})" />

  <!-- Body silhouette -->
  <path d={BODY.torso} fill={skin} opacity={sonar ? 0.25 : 0.55} stroke={sonar ? "#aaa" : "var(--skin-line)"} stroke-width="3" />

  <g class="bump" class:kicking style="transform-origin: {cx}px {cy}px">
    <!-- Translucent skin -->
    <ellipse {cx} {cy} rx={R} ry={R * 0.93} fill={skin} opacity={sonar ? 0.25 : 0.45} stroke={sonar ? "#bbb" : "var(--skin-line)"} stroke-width="3" />
    {#if kicking}
      <circle cx={cx + kickAt.x * R} cy={cy + kickAt.y * R} r={R * 0.16} fill={skin} opacity="0.6" class="kickbump" />
    {/if}
    <!-- Interior -->
    <ellipse {cx} {cy} rx={R * 0.86} ry={R * 0.8} fill="url(#womb-{uid})" opacity="0.92" />
    {#if !hideBaby}
      <g clip-path="url(#clip-{uid})">
        {#if egg}
          {#each slots(n) as [sx, sy, rot], i}
            {@const look = babies[i] ?? {}}
            <g transform="translate({cx + sx * R * 0.8} {cy + sy * R * 0.8}) rotate({rot}) scale({babyScale * 2.3})">
              <path d={EGG_PATH} fill={eggColor(look)} stroke={sonar ? "#999" : "var(--egg-line)"} stroke-width="1.5" opacity="0.92" />
              {#each [[-7, -8, 2], [6, -2, 1.6], [-3, 9, 1.8], [8, 10, 1.3]] as [ex, ey, er]}
                <circle cx={ex} cy={ey} r={er} fill={sonar ? "#aaa" : mixColors(look.furColor ?? "#d9b99b", "#000", 0.15)} opacity="0.5" />
              {/each}
              {#if p > 0.25}
                <g opacity={sonar ? 0.5 : 0.35}>
                  <Baby {look} stage={p} scale={0.24} y={2} {sonar} />
                </g>
              {/if}
            </g>
          {/each}
        {:else}
          {#each slots(n) as [sx, sy, rot], i}
            <Baby look={babies[i] ?? {}} stage={p} x={cx + sx * R * 0.8} y={cy + sy * R * 0.8} rotate={rot} scale={babyScale} {sonar} />
          {/each}
        {/if}
      </g>
    {/if}
    <!-- Shine -->
    <path d="M{cx - R * 0.62} {cy - R * 0.38} q {R * 0.2} {-R * 0.36} {R * 0.58} {-R * 0.44}" fill="none" stroke="#fff" stroke-width={Math.max(4, R * 0.06)} stroke-linecap="round" opacity={sonar ? 0.15 : 0.45} />
  </g>
</svg>

<style>
  .xray {
    width: 100%;
    height: 100%;
    max-height: 520px;
    display: block;
    overflow: visible;
  }
  .bump {
    animation: breathe 4.2s ease-in-out infinite;
    transform-box: view-box;
  }
  .bump.kicking {
    animation: breathe 4.2s ease-in-out infinite, wiggle 0.6s ease-in-out;
  }
  .kickbump {
    animation: pop 0.7s ease-out;
    transform-box: fill-box;
    transform-origin: center;
  }
  @keyframes breathe {
    0%, 100% { transform: scale(1); }
    50% { transform: scale(1.018); }
  }
  @keyframes wiggle {
    0%, 100% { translate: 0 0; }
    25% { translate: 2px -1px; }
    50% { translate: -2px 1px; }
    75% { translate: 1px 0; }
  }
  @keyframes pop {
    0% { transform: scale(0.2); opacity: 0; }
    40% { transform: scale(1); opacity: 0.7; }
    100% { transform: scale(0.4); opacity: 0; }
  }
  @media (prefers-reduced-motion: reduce) {
    .bump, .bump.kicking, .kickbump { animation: none; }
  }
</style>
