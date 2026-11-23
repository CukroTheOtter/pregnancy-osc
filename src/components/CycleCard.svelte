<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { chance, CYCLE_DAYS, cycleDay, cycleMs, cycleShades, dayNow, hhmm, hourShades, LEVEL_LABEL, level, untilFertileNow, untilWindowEnds } from "../lib/fertility";
  import { humanDuration } from "../lib/timeline";

  const s = $derived(app.data!.settings);
  const hours = $derived(s.fertileHours ?? null);
  const cycle = $derived(cycleMs(s));
  const anchor = $derived(s.cycleAnchorMs ?? app.now);
  const day = $derived(cycleDay(app.now, anchor, cycle));
  const c = $derived(chance(dayNow(s, app.now), s.fertilityPeak));
  const lv = $derived(level(c, s.fertilityPeak));
  const wait = $derived(untilFertileNow(s, app.now));
  const clock = $derived(new Date(app.now));
  const minute = $derived(clock.getHours() * 60 + clock.getMinutes());

  // Ring geometry: day 1 (or midnight) at the top, clockwise.
  const R = 70;
  const C = 2 * Math.PI * R;
  const frac = (d: number) => (d - 1) / CYCLE_DAYS;
  // Days are scaled to the pregnancy length; this is the real time until the next cycle.
  const left = $derived(Math.max(60_000, cycle * (1 - frac(day))));
  const at = $derived(hours ? minute / 1440 : frac(day));
  // The fertile window, deeper pink towards the peak.
  const shades = $derived(hours ? hourShades(hours) : cycleShades());
  const dot = $derived.by(() => {
    const a = at * 2 * Math.PI - Math.PI / 2;
    return { x: 90 + R * Math.cos(a), y: 90 + R * Math.sin(a) };
  });
</script>

<section class="card cycle">
  <h2>🌸 {hours ? "Your fertile hours" : "Your cycle"}</h2>
  <div class="body">
    <svg viewBox="0 0 180 180" width="170" height="170" aria-label={hours ? `Fertile ${hhmm(hours.startMin)} to ${hhmm(hours.endMin)}` : `Day ${Math.floor(day)} of ${CYCLE_DAYS}`}>
      <circle cx="90" cy="90" r={R} fill="none" stroke="var(--bg-3)" stroke-width="14" />
      <!-- Fertile window, one piece per day (or slice of the hours), shaded by the chance -->
      {#each shades as sh, i (i)}
        {#if sh.strength >= 0.05}
          <circle
            cx="90"
            cy="90"
            r={R}
            fill="none"
            stroke="var(--accent)"
            stroke-opacity={0.12 + 0.83 * sh.strength}
            stroke-width="14"
            stroke-dasharray="{(sh.to - sh.from) * C + 0.5} {C}"
            stroke-dashoffset={-sh.from * C}
            transform="rotate(-90 90 90)"
          />
        {/if}
      {/each}
      <circle cx={dot.x} cy={dot.y} r="10" fill="var(--accent)" stroke="var(--bg-2)" stroke-width="4" />
      {#if hours}
        <text x="90" y="86" text-anchor="middle" class="big">{hhmm(minute)}</text>
        <text x="90" y="106" text-anchor="middle" class="sub">{hhmm(hours.startMin)}–{hhmm(hours.endMin)}</text>
      {:else}
        <text x="90" y="80" text-anchor="middle" class="big">Day {Math.floor(day)}</text>
        <text x="90" y="99" text-anchor="middle" class="sub">of {CYCLE_DAYS}</text>
        <text x="90" y="116" text-anchor="middle" class="real"><title>Real time left in this cycle, whatever the pregnancy length</title>⏳ {humanDuration(left)} left</text>
      {/if}
    </svg>
    <div class="info">
      <span class="pill level {lv}">{LEVEL_LABEL[lv]}</span>
      <strong class="pct">{Math.round(c * 100)}% chance right now</strong>
      <span class="muted small">
        {#if wait <= 0}
          You're in your fertile window{hours ? ` for ${humanDuration(untilWindowEnds(hours, app.now))} more` : ""}.
        {:else}
          Next fertile window in {humanDuration(wait)}.
        {/if}
      </span>
      {#if hours}
        <span class="muted small">Every day from {hhmm(hours.startMin)} to {hhmm(hours.endMin)}, strongest in the middle.</span>
      {:else}
        <span class="muted small">A cycle lasts {humanDuration(cycle)}, scaled to your pregnancy length.</span>
      {/if}
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
  .real {
    font-size: 11px;
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
