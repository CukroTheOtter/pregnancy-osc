<script lang="ts">
  // Me tab: a month calendar of your story (WooHoos, fertile days, waits,
  // pregnancies, due dates, births, achievements). Reads only what's saved.
  import { app } from "../lib/app.svelte";
  import { shownName } from "../lib/privacy";
  import { isStopped } from "../lib/scope";
  import { ACHIEVEMENTS } from "../lib/achievements";
  import { BABY_REVEAL_WEEK, GENDER_REVEAL_WEEK, chance, cycleDay, cycleMs, level } from "../lib/fertility";
  import { TERM_WEEKS, dueMs, timeAtProgress } from "../lib/timeline";

  type Item = { emoji: string; text: string };

  const d = $derived(app.data!);
  const today = new Date(app.now);
  let shown = $state({ y: today.getFullYear(), m: today.getMonth() });

  // The week starts on the day your region uses (Monday if it can't tell).
  const firstDay = (() => {
    try {
      const loc = new Intl.Locale(navigator.language) as Intl.Locale & { weekInfo?: { firstDay: number }; getWeekInfo?: () => { firstDay: number } };
      return (loc.getWeekInfo?.() ?? loc.weekInfo)?.firstDay ?? 1;
    } catch {
      return 1;
    }
  })() % 7;
  const weekdays = Array.from({ length: 7 }, (_, i) => new Intl.DateTimeFormat(undefined, { weekday: "narrow" }).format(new Date(2024, 0, 7 + ((firstDay + i) % 7))));
  const monthFmt = new Intl.DateTimeFormat(undefined, { month: "long", year: "numeric" });
  const dayFmt = new Intl.DateTimeFormat(undefined, { weekday: "long", month: "long", day: "numeric" });
  const timeFmt = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" });

  const key = (t: number) => {
    const x = new Date(t);
    return `${x.getFullYear()}-${x.getMonth()}-${x.getDate()}`;
  };

  /** Everything that happened (or will) on each day, by `key`. */
  const items = $derived.by(() => {
    const map = new Map<string, Item[]>();
    const add = (t: number, emoji: string, text: string) => {
      if (!t) return;
      const k = key(t);
      (map.get(k) ?? map.set(k, []).get(k)!).push({ emoji, text });
    };
    for (const f of d.funLog) {
      const who = shownName(d, f.partnerName, f.partnerUserId);
      add(f.timeMs, "💕", `${timeFmt.format(f.timeMs)} WooHoo${who ? ` with ${who}` : ""}${f.worldName ? ` in ${f.worldName}` : ""}`);
    }
    for (const w of d.waiting) add(w.revealMs, "⏳", `${timeFmt.format(w.revealMs)} Find out if it took`);
    const now = app.now;
    for (const p of d.pregnancies) {
      if (isStopped(p)) continue;
      if (p.endedMs == null) {
        add(dueMs(p.timeline, now), "📅", `Due date${p.name ? ` for ${p.name}` : ""}`);
        add(timeAtProgress(p.timeline, now, BABY_REVEAL_WEEK / TERM_WEEKS), "✨", `Week ${BABY_REVEAL_WEEK}: how many babies`);
        add(timeAtProgress(p.timeline, now, GENDER_REVEAL_WEEK / TERM_WEEKS), "🎀", `Week ${GENDER_REVEAL_WEEK}: girl or boy`);
      }
      if (p.born && p.endedMs != null) {
        const name = p.name || (p.babyCount > 1 ? "Your babies" : "Your baby");
        add(p.endedMs, "👶", `${name} ${p.babyCount > 1 ? "were" : "was"} born`);
      }
    }
    for (const a of ACHIEVEMENTS) {
      const at = d.achievements?.[a.id];
      if (at != null && !d.testAchievements?.includes(a.id)) add(at, "🏆", `${a.emoji} ${a.title}`);
    }
    return map;
  });

  /** Pregnancies as [start, end] (end: the birth, or the due date). */
  const spans = $derived(d.pregnancies.filter((p) => !isStopped(p)).map((p) => [p.timeline.startMs, p.endedMs ?? dueMs(p.timeline, app.now)] as const));

  // Fertile days only make sense when a cycle lasts at least two weeks (fast
  // pregnancies have cycles of hours, too short to show by the day).
  const cycle = $derived(d.settings.fertility && d.settings.cycleAnchorMs != null && cycleMs(d.settings) >= 14 * 86_400_000 ? cycleMs(d.settings) : null);
  function fertility(noon: number): "peak" | "high" | null {
    if (cycle == null) return null;
    const l = level(chance(cycleDay(noon, d.settings.cycleAnchorMs!, cycle), d.settings.fertilityPeak), d.settings.fertilityPeak);
    return l === "peak" || l === "high" ? l : null;
  }

  const cells = $derived.by(() => {
    const first = new Date(shown.y, shown.m, 1);
    const lead = (first.getDay() - firstDay + 7) % 7;
    const days = new Date(shown.y, shown.m + 1, 0).getDate();
    const todayKey = key(app.now);
    const out: ({ day: number; key: string; items: Item[]; pregnant: boolean; fertile: "peak" | "high" | null; today: boolean } | null)[] = Array(lead).fill(null);
    for (let day = 1; day <= days; day++) {
      const start = new Date(shown.y, shown.m, day).getTime();
      const end = new Date(shown.y, shown.m, day + 1).getTime();
      const k = key(start);
      const pregnant = spans.some(([a, b]) => a < end && b >= start);
      out.push({ day, key: k, items: items.get(k) ?? [], pregnant, fertile: pregnant ? null : fertility(start + 43_200_000), today: k === todayKey });
    }
    return out;
  });

  let picked = $state<string | null>(null);
  const pickedCell = $derived(cells.find((c) => c?.key === picked) ?? null);
  const legend = $derived.by(() => {
    const seen = new Set(cells.flatMap((c) => c?.items.map((i) => i.emoji) ?? []));
    const out: [string, string][] = [];
    if (cells.some((c) => c?.pregnant)) out.push(["pregnant", "Pregnant"]);
    if (cells.some((c) => c?.fertile === "high")) out.push(["fertile", "Fertile"]);
    if (cells.some((c) => c?.fertile === "peak")) out.push(["peak", "Peak fertility"]);
    const names: Record<string, string> = { "💕": "WooHoo", "⏳": "Find out", "✨": "Baby count", "🎀": "Girl or boy", "📅": "Due date", "👶": "Born","🏆": "Achievement" };
    for (const [e, n] of Object.entries(names)) if (seen.has(e)) out.push([e, n]);
    return out;
  });

  function move(by: number) {
    const x = new Date(shown.y, shown.m + by, 1);
    shown = { y: x.getFullYear(), m: x.getMonth() };
    picked = null;
  }
  const isThisMonth = $derived(shown.y === today.getFullYear() && shown.m === today.getMonth());
</script>

<section class="card">
  <div class="row head">
    <h2 class="grow">📅 Calendar</h2>
    <button class="small ghost" onclick={() => move(-1)} aria-label="Previous month">‹</button>
    <button class="small ghost month" onclick={() => { shown = { y: today.getFullYear(), m: today.getMonth() }; picked = null; }} disabled={isThisMonth} title="Back to this month">{monthFmt.format(new Date(shown.y, shown.m, 1))}</button>
    <button class="small ghost" onclick={() => move(1)} aria-label="Next month">›</button>
  </div>

  <div class="grid">
    {#each weekdays as w, i (i)}<span class="wd">{w}</span>{/each}
    {#each cells as c, i (c?.key ?? `x${i}`)}
      {#if c}
        {@const icons = [...new Set(c.items.map((x) => x.emoji))]}
        <button
          class="day"
          class:pregnant={c.pregnant}
          class:fertile={c.fertile === "high"}
          class:peak={c.fertile === "peak"}
          class:today={c.today}
          class:picked={picked === c.key}
          onclick={() => (picked = picked === c.key ? null : c.key)}
          aria-label="{dayFmt.format(new Date(shown.y, shown.m, c.day))}{c.items.length ? `, ${c.items.length} thing${c.items.length > 1 ? "s" : ""}` : ""}"
        >
          <span class="n">{c.day}</span>
          {#if icons.length}<span class="icons">{icons.slice(0, 3).join("")}</span>{/if}
        </button>
      {:else}
        <span></span>
      {/if}
    {/each}
  </div>

  {#if pickedCell}
    <div class="details">
      <strong class="small">{dayFmt.format(new Date(shown.y, shown.m, pickedCell.day))}</strong>
      {#if pickedCell.pregnant}<span class="small muted">🤰 Pregnant</span>{/if}
      {#if pickedCell.fertile}<span class="small muted">🌸 {pickedCell.fertile === "peak" ? "Peak fertility" : "Fertile"}</span>{/if}
      {#each pickedCell.items as it, i (i)}<span class="small">{it.emoji} {it.text}</span>{/each}
      {#if !pickedCell.items.length && !pickedCell.pregnant && !pickedCell.fertile}<span class="small muted">Nothing this day.</span>{/if}
    </div>
  {/if}

  {#if legend.length}
    <div class="legend muted small">
      {#each legend as [e, n] (n)}
        {#if e === "pregnant" || e === "fertile" || e === "peak"}<span><i class="swatch {e}"></i> {n}</span>{:else}<span>{e} {n}</span>{/if}
      {/each}
    </div>
  {/if}
</section>

<style>
  h2 {
    margin: 0;
  }
  .head {
    gap: 2px;
    margin-bottom: 8px;
  }
  .month {
    min-width: 8.5em;
  }
  .month:disabled {
    opacity: 1;
    cursor: default;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 4px;
  }
  .wd {
    text-align: center;
    font-size: 0.72rem;
    font-weight: 700;
    color: var(--muted);
  }
  .day {
    all: unset;
    box-sizing: border-box;
    cursor: pointer;
    height: 46px;
    min-width: 0;
    border-radius: 10px;
    background: var(--bg);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1px;
    border: 2px solid transparent;
  }
  .day:hover,
  .day:focus-visible {
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
  }
  .day.pregnant {
    background: color-mix(in srgb, var(--accent) 22%, var(--bg));
  }
  .day.fertile {
    border-style: dashed;
    border-color: color-mix(in srgb, var(--accent) 55%, transparent);
  }
  .day.peak {
    border-color: var(--accent);
  }
  .day.today .n {
    background: var(--accent);
    color: var(--on-accent, #fff);
    border-radius: 999px;
    padding: 0 6px;
  }
  .day.picked {
    border-color: var(--accent);
    border-style: solid;
  }
  .n {
    font-size: 0.8rem;
    font-weight: 700;
  }
  .icons {
    font-size: 0.7rem;
    line-height: 1;
    white-space: nowrap;
    overflow: hidden;
    max-width: 100%;
  }
  .details {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin-top: 10px;
    padding: 8px 10px;
    border-radius: 12px;
    background: var(--bg);
  }
  .swatch {
    display: inline-block;
    width: 12px;
    height: 12px;
    border-radius: 4px;
    vertical-align: -2px;
    background: var(--bg);
    border: 2px solid transparent;
  }
  .swatch.pregnant {
    background: color-mix(in srgb, var(--accent) 22%, var(--bg));
  }
  .swatch.fertile {
    border: 2px dashed color-mix(in srgb, var(--accent) 55%, transparent);
  }
  .swatch.peak {
    border-color: var(--accent);
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    margin-top: 10px;
  }
</style>
