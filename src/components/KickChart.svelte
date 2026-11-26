<script lang="ts">
  import { app, errorToast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { busiest, HOUR_MS, kickBins, kickHours, smoothPath, type KickHour } from "../lib/kicks";
  import type { KickChartStyle, Pregnancy, Settings } from "../lib/types";

  // Belly activity this VRChat session (or the last one once VRChat closes),
  // as a line, dots or bars (picked here). No session known yet: the last 24 hours.
  let { p }: { p: Pregnancy } = $props();

  const STYLES: { k: KickChartStyle; name: string }[] = [
    { k: "line", name: "Line" },
    { k: "dots", name: "Dots" },
    { k: "bars", name: "Bars" },
  ];
  const style = $derived<KickChartStyle>(app.data?.settings.kickChart ?? "line");

  async function pickStyle(k: KickChartStyle) {
    if (!app.data || k === style) return;
    const s = { ...($state.snapshot(app.data.settings) as Settings), kickChart: k };
    try {
      await api.saveSettings(s);
      app.data.settings.kickChart = k;
    } catch (e) {
      errorToast(e);
    }
  }

  const since = $derived(app.status?.vrchatSinceMs ?? null);
  const live = $derived(!!since && !!app.status?.vrchatRunning);
  const view = $derived.by(() => {
    const kicks = p.kicks ?? [];
    if (!since) {
      const bins = kickHours(kicks, app.now);
      return { bins, binMs: HOUR_MS, from: null, start: bins[0].startMs, to: app.now };
    }
    const until = live ? null : (app.status?.vrchatUntilMs ?? null);
    // A session with no known end (the app closed first) ends at its last kick.
    const to = live ? app.now : (until ?? kicks.filter((k) => k.timeMs >= since).at(-1)?.timeMs ?? since);
    const b = kickBins(kicks, since, to);
    return { ...b, from: since, start: b.bins[0].startMs, to };
  });
  const hours = $derived(view.bins);
  const top = $derived(Math.max(1, ...hours.map((h) => h.total)));
  const total = $derived(hours.reduce((s, h) => s + h.total, 0));
  const peak = $derived(busiest(hours));
  const when = $derived(!view.from ? "in 24 h" : live ? "this session" : "last session");
  const shown = $derived((p.kicks ?? []).filter((k) => k.timeMs >= (view.from ?? view.start) && k.timeMs <= view.to));
  let picked = $state<number | null>(null);

  // Drawing area, in pixels (the chart is as wide as the card).
  let width = $state(320);
  const H = 104;
  const BASE = H - 8;
  const TOP = 24;
  const end = $derived(view.start + hours.length * view.binMs);
  const xAt = (ms: number) => ((ms - view.start) / (end - view.start)) * width;
  const bw = $derived(width / hours.length);
  const cx = (i: number) => bw * (i + 0.5);
  const yOf = (n: number) => BASE - (n / top) * (BASE - TOP);
  const points = $derived(hours.map((h, i) => [cx(i), yOf(h.total)] as [number, number]));
  const line = $derived(smoothPath(points));
  // The line's bands, stacked from the floor: little, then strong, then big on top.
  const close = (d: string) => `${d} L${points.at(-1)![0]},${BASE} L${points[0][0]},${BASE} Z`;
  const bands = $derived([
    { k: "extreme", d: close(line) },
    { k: "strong", d: close(smoothPath(hours.map((h, i) => [cx(i), yOf(h.weak + h.strong)]))) },
    { k: "weak", d: close(smoothPath(hours.map((h, i) => [cx(i), yOf(h.weak)]))) },
  ]);
  // Bars: one section per strength, little at the bottom.
  const barW = $derived(Math.max(3, Math.min(12, bw - 3)));
  const sections = (h: KickHour) => {
    let y = BASE;
    return (["weak", "strong", "extreme"] as const)
      .filter((k) => h[k])
      .map((k) => {
        const hgt = (h[k] / top) * (BASE - TOP);
        y -= hgt;
        return { k, y, h: hgt };
      });
  };

  // Dots: each kick sits a little above or below the track, the same way every time.
  const R = { weak: 4, strong: 6, extreme: 8.5 } as const;
  const jitter = (t: number) => ((((t / 1000) * 2654435761) % 1000) / 1000 - 0.5) * 2;

  const hourFmt = new Intl.DateTimeFormat(undefined, { hour: "numeric" });
  const timeFmt = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" });
  const fmt = (ms: number) => (view.binMs < HOUR_MS || new Date(ms).getMinutes() ? timeFmt : hourFmt).format(ms);
  const span = (h: KickHour) => `${fmt(h.startMs)} – ${fmt(h.startMs + view.binMs)}`;
  const plural = (n: number, w: string) => `${n} ${w}${n === 1 ? "" : "s"}`;
  // Axis labels: a few evenly spaced bars (just the first on a short session).
  const ticks = $derived(hours.length >= 8 ? [0, 1, 2, 3].map((q) => Math.round((q * hours.length) / 4)) : [0]);
  const barWord = $derived(view.binMs >= HOUR_MS ? (view.binMs === HOUR_MS ? "hour" : `${view.binMs / HOUR_MS} hours`) : `${view.binMs / 60_000} minutes`);
  const label = $derived(view.from ? (live ? "Current session" : "Last session") : "Last 24 hours");

  function detail(h: KickHour): string {
    if (!h.total) return `${span(h)}: no kicks`;
    const parts = [h.extreme && `${h.extreme} big`, h.strong && `${h.strong} strong`, h.weak && `${h.weak} little`].filter(Boolean);
    return `${span(h)}: ${plural(h.total, "kick")} (${parts.join(", ")})`;
  }
  const marked = $derived(picked !== null && hours[picked] ? picked : peak ? hours.indexOf(peak) : -1);
</script>

<div class="chart-wrap">
  <div class="head small">
    <strong>{label}</strong>
    {#if view.from}
      <span class="muted">· {live ? `since ${timeFmt.format(view.from)}` : `${timeFmt.format(view.from)} – ${timeFmt.format(view.to)}`}</span>
    {/if}
    {#if live}<span class="live" title="VRChat is running"></span>{/if}
    <span class="grow"></span>
    <div class="styles" role="radiogroup" aria-label="Chart style">
      {#each STYLES as s (s.k)}
        <button class:on={style === s.k} role="radio" aria-checked={style === s.k} onclick={() => pickStyle(s.k)}>{s.name}</button>
      {/each}
    </div>
  </div>

  <div class="plot" bind:clientWidth={width}>
    <svg {width} height={H} role="group" aria-label="Kicks per {barWord}, {label.toLowerCase()}">
      {#each [0.25, 0.5, 0.75] as g}<line class="grid" x1="0" x2={width} y1={BASE - g * (BASE - TOP)} y2={BASE - g * (BASE - TOP)} />{/each}
      <line class="base" x1="0" x2={width} y1={BASE} y2={BASE} />
      {#if picked !== null && hours[picked]}<rect class="pick" x={picked * bw} y="0" width={bw} height={H} rx="8" />{/if}

      {#key style}
        {#if style === "line"}
          {#each bands as b (b.k)}<path class="band {b.k} rise" d={b.d} />{/each}
          {#key hours.length}<path class="stroke draw" d={line} pathLength="1" />{/key}
          {#if marked >= 0}<circle class="pt" cx={cx(marked)} cy={yOf(hours[marked].total)} r="4.5" />{/if}
          {#if live}<circle class="now" cx={points.at(-1)![0]} cy={points.at(-1)![1]} r="5" />{/if}
        {:else if style === "dots"}
          <line class="track" x1="0" x2={width} y1={(BASE + TOP) / 2} y2={(BASE + TOP) / 2} />
          {#each shown as k, i (k.timeMs)}
            <circle
              class="kick bubble-up {k.strength}"
              style="animation-delay:{Math.min(i * 35, 900)}ms"
              cx={xAt(k.timeMs)}
              cy={(BASE + TOP) / 2 + jitter(k.timeMs) * (k.strength === "weak" ? 26 : 18)}
              r={R[k.strength]}
            />
          {/each}
          {#if live}<circle class="now" cx={width - 3} cy={(BASE + TOP) / 2} r="3.5" />{/if}
        {:else}
          {#each hours as h, i (h.startMs)}
            {#if h.total}
              <g class="bounce" style="animation-delay:{Math.min(i * 30, 700)}ms">
                <clipPath id="kc-{p.id}-{i}"><rect x={cx(i) - barW / 2} y={yOf(h.total)} width={barW} height={BASE - yOf(h.total)} rx={barW / 2} /></clipPath>
                <g clip-path="url(#kc-{p.id}-{i})">
                  {#each sections(h) as s (s.k)}
                    {@const gap = s.y > yOf(h.total) + 0.5 ? 1.5 : 0}
                    <rect class="seg {s.k}" x={cx(i) - barW / 2} y={s.y + gap} width={barW} height={Math.max(0, s.h - gap)} />
                  {/each}
                </g>
              </g>
            {:else}
              <circle class="rest" cx={cx(i)} cy={BASE - 3} r="1.6" />
            {/if}
          {/each}
        {/if}
      {/key}

      {#if marked >= 0 && style !== "dots"}
        {@const bx = Math.min(width - 12, Math.max(12, cx(marked)))}
        {@const by = yOf(hours[marked].total) - 15}
        <rect class="bubble" x={bx - 11} y={by - 8} width="22" height="14" rx="7" />
        <text class="bubble-text" x={bx} y={by + 2.5}>{hours[marked].total}</text>
      {/if}

      {#each hours as h, i (h.startMs)}
        <rect class="hit" role="button" tabindex="-1" aria-label={detail(h)} x={i * bw} y="0" width={bw} height={H}
          onclick={() => (picked = picked === i ? null : i)} onkeydown={() => {}}><title>{detail(h)}</title></rect>
      {/each}
    </svg>
    <div class="axis muted" aria-hidden="true">
      {#each ticks as i}<span style="left:{(i / hours.length) * 100}%">{fmt(hours[i].startMs)}</span>{/each}
      {#if hours.length > 2}<span class="end">{live || !view.from ? "now" : fmt(view.to)}</span>{/if}
    </div>
  </div>

  <p class="small summary">
    {#if picked !== null && hours[picked]}
      {detail(hours[picked])}
    {:else if peak}
      {plural(total, "kick")} {when} · busiest {span(peak)}
    {:else if live}
      No kicks yet this session.
    {:else}
      No kicks {view.from ? "last session" : "in the last 24 hours"}.
    {/if}
  </p>
  <div class="legend muted small" aria-hidden="true">
    <span class="key"><i class="weak"></i>little</span><span class="key"><i class="strong"></i>strong</span><span class="key"><i class="extreme"></i>big</span>
    <span class="grow"></span><span>{view.from ? "Quiet stretches: baby was resting." : "Quiet hours: you weren't in VRChat, or baby was resting."}</span>
  </div>
</div>

<style>
  .chart-wrap {
    margin-top: 12px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 8px;
  }
  .live {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--accent) 60%, transparent);
    animation: ping 2s ease-out infinite;
  }
  @keyframes ping {
    70% {
      box-shadow: 0 0 0 6px transparent;
    }
    100% {
      box-shadow: 0 0 0 0 transparent;
    }
  }
  .styles {
    display: flex;
    gap: 2px;
    padding: 2px;
    border-radius: 999px;
    background: var(--bg);
  }
  .styles button {
    min-width: 28px;
    height: 22px;
    padding: 0 6px;
    border: none;
    box-shadow: none;
    border-radius: 999px;
    background: transparent;
    color: var(--muted, inherit);
    font-size: 0.72rem;
    font-weight: 800;
    line-height: 1;
    cursor: pointer;
  }
  .styles button.on {
    background: var(--accent);
    color: var(--bg);
  }
  .plot {
    padding: 4px 0 6px;
    border-radius: 16px;
    background: var(--bg);
    overflow: hidden;
  }
  svg {
    display: block;
  }
  .grid {
    stroke: color-mix(in srgb, var(--text) 7%, transparent);
  }
  .base {
    stroke: color-mix(in srgb, var(--text) 14%, transparent);
  }
  .pick {
    fill: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .hit {
    fill: transparent;
    cursor: pointer;
    outline: none;
  }
  /* Line bands: solid soft tones so the stacked layers don't blend. */
  .band.weak {
    fill: color-mix(in srgb, color-mix(in srgb, var(--accent) 45%, var(--text)) 55%, var(--bg));
  }
  .band.strong {
    fill: color-mix(in srgb, var(--accent-2) 65%, var(--bg));
  }
  .band.extreme {
    fill: color-mix(in srgb, var(--accent) 75%, var(--bg));
  }
  .rise {
    transform-box: fill-box;
    transform-origin: bottom;
    animation: rise 0.7s ease-out both;
  }
  .band.strong.rise {
    animation-delay: 0.08s;
  }
  .band.weak.rise {
    animation-delay: 0.16s;
  }
  @keyframes rise {
    from {
      transform: scaleY(0);
      opacity: 0;
    }
  }
  .stroke {
    fill: none;
    stroke: var(--accent);
    stroke-width: 2.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .draw {
    stroke-dasharray: 1;
    animation: draw 0.9s ease-out both;
  }
  @keyframes draw {
    from {
      stroke-dashoffset: 1;
    }
    to {
      stroke-dashoffset: 0;
    }
  }
  .pt {
    fill: var(--accent);
    stroke: var(--bg);
    stroke-width: 2;
  }
  .now {
    fill: #fff;
    stroke: var(--accent);
    stroke-width: 2;
  }
  .track {
    stroke: color-mix(in srgb, var(--text) 12%, transparent);
    stroke-width: 2;
    stroke-linecap: round;
  }
  /* Dots float up from below the track and settle, one after another. */
  .bubble-up {
    transform-box: fill-box;
    transform-origin: center;
    animation: bubble-up 0.6s cubic-bezier(0.3, 1.5, 0.6, 1) both;
  }
  @keyframes bubble-up {
    from {
      transform: translateY(36px) scale(0.3);
      opacity: 0;
    }
    60% {
      opacity: 1;
    }
  }
  /* Bars spring up from the floor. */
  .bounce {
    transform-box: fill-box;
    transform-origin: bottom;
    animation: bounce 0.65s both;
  }
  @keyframes bounce {
    0% {
      transform: scaleY(0);
      animation-timing-function: ease-out;
    }
    55% {
      transform: scaleY(1.15);
      animation-timing-function: ease-in-out;
    }
    78% {
      transform: scaleY(0.94);
    }
    100% {
      transform: scaleY(1);
    }
  }
  .kick.weak,
  .seg.weak,
  .legend .weak {
    fill: color-mix(in srgb, var(--accent) 45%, var(--text));
    background: color-mix(in srgb, var(--accent) 45%, var(--text));
  }
  .kick.strong,
  .seg.strong,
  .legend .strong {
    fill: var(--accent-2);
    background: var(--accent-2);
  }
  .kick.extreme,
  .seg.extreme,
  .legend .extreme {
    fill: var(--accent);
    background: var(--accent);
  }
  .rest {
    fill: color-mix(in srgb, var(--text) 20%, transparent);
  }
  .bubble {
    fill: var(--accent);
  }
  .bubble-text {
    fill: var(--bg);
    font-size: 9.5px;
    font-weight: 800;
    text-anchor: middle;
  }
  .axis {
    position: relative;
    height: 14px;
    margin: 2px 8px 0;
    font-size: 0.68rem;
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
    margin: 8px 0 4px;
    font-weight: 700;
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 6px;
    align-items: center;
  }
  .key {
    padding: 1px 8px 1px 6px;
    border-radius: 999px;
    background: var(--bg);
  }
  .legend i {
    display: inline-block;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    margin-right: 4px;
    vertical-align: -1px;
  }
  @media (prefers-reduced-motion: reduce) {
    .live,
    .draw,
    .rise,
    .bubble-up,
    .bounce {
      animation: none;
    }
  }
</style>
