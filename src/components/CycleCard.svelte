<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { chance, CYCLE_DAYS, cycleDay, cycleMs, LEVEL_LABEL, level, untilFertile, WINDOW } from "../lib/fertility";
  import { humanDuration } from "../lib/timeline";

  const s = $derived(app.data!.settings);
  const cycle = $derived(cycleMs(s));
  const anchor = $derived(s.cycleAnchorMs ?? app.now);
  const day = $derived(cycleDay(app.now, anchor, cycle));
  const c = $derived(chance(day, s.fertilityPeak));
  const lv = $derived(level(c, s.fertilityPeak));
  const wait = $derived(untilFertile(app.now, anchor, cycle, s.fertilityPeak));

  // Ring geometry: day 1 at the top, clockwise.
  const R = 70;
  const C = 2 * Math.PI * R;
  const frac = (d: number) => (d - 1) / CYCLE_DAYS;
  const windowLen = $derived((frac(WINDOW.end + 1) - frac(WINDOW.start)) * C);
  const dot = $derived.by(() => {
    const a = frac(day) * 2 * Math.PI - Math.PI / 2;
    return { x: 90 + R * Math.cos(a), y: 90 + R * Math.sin(a) };
  });
</script>

<section class="card cycle">
  <h2>🌸 Your cycle</h2>
  <div class="body">
    <svg viewBox="0 0 180 180" width="170" height="170" aria-label="Day {Math.floor(day)} of {CYCLE_DAYS}">
      <circle cx="90" cy="90" r={R} fill="none" stroke="var(--bg-3)" stroke-width="14" />
      <!-- Fertile window -->
      <circle
        cx="90"
        cy="90"
        r={R}
        fill="none"
        stroke="var(--accent)"
        stroke-opacity="0.45"
        stroke-width="14"
        stroke-dasharray="{windowLen} {C}"
        stroke-dashoffset={-frac(WINDOW.start) * C}
        transform="rotate(-90 90 90)"
        stroke-linecap="round"
      />
      <circle cx={dot.x} cy={dot.y} r="10" fill="var(--accent)" stroke="var(--bg-2)" stroke-width="4" />
      <text x="90" y="86" text-anchor="middle" class="big">Day {Math.floor(day)}</text>
      <text x="90" y="108" text-anchor="middle" class="sub">of {CYCLE_DAYS}</text>
    </svg>
    <div class="info">
      <span class="pill level {lv}">{LEVEL_LABEL[lv]}</span>
      <strong class="pct">{Math.round(c * 100)}% chance right now</strong>
      <span class="muted small">
        {#if wait <= 0}
          You're in your fertile window.
        {:else}
          Next fertile window in {humanDuration(wait)}.
        {/if}
      </span>
      <span class="muted small">A cycle lasts {humanDuration(cycle)}, scaled to your pregnancy length.</span>
    </div>
  </div>
</section>

<style>
  .body {
    display: flex;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
    justify-content: center;
  }
  .info {
    flex: 1;
    min-width: 180px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .big {
    font-size: 26px;
    font-weight: 900;
    fill: var(--text);
  }
  .sub {
    font-size: 13px;
    fill: var(--muted);
  }
  .pct {
    font-size: 1.05rem;
  }
  .level {
    align-self: flex-start;
  }
  .level.high,
  .level.peak {
    background: color-mix(in srgb, var(--accent) 30%, var(--bg-2));
    color: var(--text);
  }
  .level.peak {
    animation: glow 1.8s ease-in-out infinite alternate;
  }
  @keyframes glow {
    to {
      box-shadow: 0 0 14px color-mix(in srgb, var(--accent) 60%, transparent);
    }
  }
</style>
