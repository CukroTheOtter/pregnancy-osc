<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { busiest, kickHours, type KickHour } from "../lib/kicks";
  import type { Pregnancy } from "../lib/types";

  // Belly activity over the last 24 hours, like a heart rate chart.
  let { p }: { p: Pregnancy } = $props();

  // Redrawn once a minute, not every tick.
  const minute = $derived(Math.floor(app.now / 60_000) * 60_000);
  const hours = $derived(kickHours(p.kicks ?? [], minute));
  const top = $derived(Math.max(1, ...hours.map((h) => h.total)));
  const total = $derived(hours.reduce((s, h) => s + h.total, 0));
  const peak = $derived(busiest(hours));
  let picked = $state<number | null>(null);

  const hourFmt = new Intl.DateTimeFormat(undefined, { hour: "numeric" });
  const span = (h: KickHour) => `${hourFmt.format(h.startMs)} – ${hourFmt.format(h.startMs + 3_600_000)}`;
  const plural = (n: number, w: string) => `${n} ${w}${n === 1 ? "" : "s"}`;

  function detail(h: KickHour): string {
    if (!h.total) return `${span(h)}: no kicks`;
    const parts = [h.extreme && `${h.extreme} big`, h.strong && `${h.strong} strong`, h.weak && `${h.weak} little`].filter(Boolean);
    return `${span(h)}: ${plural(h.total, "kick")} (${parts.join(", ")})`;
  }
</script>

<div class="chart-wrap">
  <div class="bars" role="group" aria-label="Kicks per hour, last 24 hours">
    {#each hours as h, i (h.startMs)}
      <button
        class="bar"
        class:now={i === 23}
        class:on={picked === i}
        onclick={() => (picked = picked === i ? null : i)}
        aria-label={detail(h)}
        title={detail(h)}
      >
        <span class="pile" style="height:{(h.total / top) * 100}%">
          {#if h.extreme}<span class="seg extreme" style="flex:{h.extreme}"></span>{/if}
          {#if h.strong}<span class="seg strong" style="flex:{h.strong}"></span>{/if}
          {#if h.weak}<span class="seg weak" style="flex:{h.weak}"></span>{/if}
        </span>
      </button>
    {/each}
  </div>
  <div class="axis muted" aria-hidden="true">
    {#each [0, 6, 12, 18] as i}<span style="left:{(i / 24) * 100}%">{hourFmt.format(hours[i].startMs)}</span>{/each}
    <span class="end">now</span>
  </div>

  <p class="small summary">
    {#if picked !== null}
      {detail(hours[picked])}
    {:else if peak}
      {plural(total, "kick")} in 24 h · busiest {span(peak)}
    {:else}
      No kicks in the last 24 hours.
    {/if}
  </p>
  <div class="legend muted small" aria-hidden="true">
    <span><i class="weak"></i>little</span><span><i class="strong"></i>strong</span><span><i class="extreme"></i>big</span>
    <span class="grow"></span><span>Empty hours: you weren't in VRChat, or baby was resting.</span>
  </div>
</div>

<style>
  .chart-wrap {
    margin-top: 12px;
  }
  .bars {
    display: grid;
    grid-template-columns: repeat(24, 1fr);
    gap: 3px;
    height: 88px;
    align-items: end;
    padding: 6px;
    border-radius: 14px;
    background: var(--bg);
  }
  .bar {
    height: 100%;
    padding: 0;
    margin: 0;
    background: transparent;
    border: none;
    box-shadow: none;
    border-radius: 6px;
    display: flex;
    align-items: flex-end;
    cursor: pointer;
  }
  .bar:hover,
  .bar.on {
    background: var(--bg-3);
  }
  .bar.now {
    box-shadow: inset 0 -2px 0 var(--accent);
  }
  .pile {
    width: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-radius: 5px;
    overflow: hidden;
    transition: height 0.3s ease;
  }
  .seg {
    min-height: 3px;
  }
  .weak {
    background: color-mix(in srgb, var(--accent) 40%, var(--bg-3));
  }
  .strong {
    background: var(--accent-2);
  }
  .extreme {
    background: var(--accent);
  }
  .axis {
    position: relative;
    height: 16px;
    margin: 2px 6px 0;
    font-size: 0.7rem;
    font-weight: 700;
  }
  .axis span {
    position: absolute;
    top: 0;
  }
  .axis .end {
    right: 0;
  }
  .summary {
    margin: 6px 0 2px;
    font-weight: 700;
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    align-items: center;
  }
  .legend i {
    display: inline-block;
    width: 9px;
    height: 9px;
    border-radius: 3px;
    margin-right: 4px;
    vertical-align: -1px;
  }
  @media (prefers-reduced-motion: reduce) {
    .pile {
      transition: none;
    }
  }
</style>
