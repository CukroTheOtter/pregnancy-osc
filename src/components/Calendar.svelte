<script lang="ts">
  // Me tab: a month calendar of your story, laid out like a normal calendar
  // app: labelled bars that run across the days they cover (pregnant,
  // fertile, waiting) and labelled chips for one-day things. Tap a day for
  // the full story of that day. Reads only what's saved.
  import { app } from "../lib/app.svelte";
  import { shownName } from "../lib/privacy";
  import { avatarName, isStopped, pregnancyAvatar } from "../lib/scope";
  import { ACHIEVEMENTS } from "../lib/achievements";
  import { BABY_REVEAL_WEEK, GENDER_REVEAL_WEEK, chance, cycleDay, cycleMs, level } from "../lib/fertility";
  import { TERM_WEEKS, dueMs, timeAtProgress } from "../lib/timeline";

  type Kind = "pregnant" | "fertile" | "waiting" | "woohoo" | "milestone" | "due" | "born" | "achievement";
  /** A bar or chip: from day `a` to day `b` (both included, as day numbers since 1970 in local time). */
  type Ev = { kind: Kind; a: number; b: number; label: string; seg?: (from: number, to: number) => string };
  /** One line in a day's details. */
  type Line = { kind: Kind; text: string };

  const MAX_LANES = 3;
  const DAY = 86_400_000;

  const d = $derived(app.data!);
  const today = new Date(app.now);
  let shown = $state({ y: today.getFullYear(), m: today.getMonth() });
  let picked = $state<number | null>(null);

  // Local day numbers (so a bar covers whole days, whatever the time zone).
  const dayOf = (t: number) => {
    const x = new Date(t);
    return Math.round(Date.UTC(x.getFullYear(), x.getMonth(), x.getDate()) / DAY);
  };
  const dateOf = (n: number) => {
    const u = new Date(n * DAY);
    return new Date(u.getUTCFullYear(), u.getUTCMonth(), u.getUTCDate());
  };
  const noonOf = (n: number) => dateOf(n).getTime() + DAY / 2;

  // The week starts on the day your region uses (Monday if it can't tell).
  const firstDay = (() => {
    try {
      const loc = new Intl.Locale(navigator.language) as Intl.Locale & { weekInfo?: { firstDay: number }; getWeekInfo?: () => { firstDay: number } };
      return (loc.getWeekInfo?.() ?? loc.weekInfo)?.firstDay ?? 1;
    } catch {
      return 1;
    }
  })() % 7;
  const weekdays = Array.from({ length: 7 }, (_, i) => new Intl.DateTimeFormat(undefined, { weekday: "short" }).format(new Date(2024, 0, 7 + ((firstDay + i) % 7))));
  const monthFmt = new Intl.DateTimeFormat(undefined, { month: "long", year: "numeric" });
  const dayFmt = new Intl.DateTimeFormat(undefined, { weekday: "long", month: "long", day: "numeric" });
  const shortFmt = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric" });
  const timeFmt = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" });

  const pregnancies = $derived(d.pregnancies.filter((p) => !isStopped(p)));
  // Several pregnancies (per-avatar mode): each bar says which avatar.
  const whose = (p: (typeof pregnancies)[number]) => (pregnancies.length > 1 ? ` (${avatarName(d, pregnancyAvatar(d, p))})` : "");
  /** Who the other parent is (as shown: privacy mode applies), or null. */
  const partnerOf = (p: (typeof pregnancies)[number]) => shownName(d, p.partner?.name ?? null, p.partner?.userId ?? null);
  /** The pregnancy week on a day (1 to 40). */
  const weekOn = (p: (typeof pregnancies)[number], n: number) => Math.min(TERM_WEEKS, Math.max(1, Math.floor(((noonOf(n) - p.timeline.startMs) / p.timeline.durationMs) * TERM_WEEKS) + 1));

  // Fertile days only make sense when a cycle lasts at least two weeks (fast
  // pregnancies have cycles of hours, too short to show by the day).
  const cycle = $derived(d.settings.fertility && d.settings.cycleAnchorMs != null && cycleMs(d.settings) >= 14 * DAY ? cycleMs(d.settings) : null);
  function fertility(n: number): "peak" | "high" | null {
    if (cycle == null) return null;
    const peak = d.settings.fertilityPeak;
    const l = level(chance(cycleDay(noonOf(n), d.settings.cycleAnchorMs!, cycle), peak), peak);
    return l === "peak" || l === "high" ? l : null;
  }
  const pregnantOn = (n: number) => pregnancies.some((p) => dayOf(p.timeline.startMs) <= n && n <= dayOf(p.endedMs ?? dueMs(p.timeline, app.now)));

  /** The days on screen: whole weeks around the shown month. */
  const range = $derived.by(() => {
    const first = dayOf(new Date(shown.y, shown.m, 1).getTime());
    const last = dayOf(new Date(shown.y, shown.m + 1, 0).getTime());
    const lead = (dateOf(first).getDay() - firstDay + 7) % 7;
    const start = first - lead;
    const weeks = Math.ceil((last - start + 1) / 7);
    return { start, end: start + weeks * 7 - 1, weeks };
  });

  const events = $derived.by(() => {
    const out: Ev[] = [];
    const now = app.now;
    for (const p of pregnancies) {
      const a = dayOf(p.timeline.startMs);
      const who = partnerOf(p);
      // Each week's piece of the bar says which week it is, and with whom.
      out.push({ kind: "pregnant", a, b: dayOf(p.endedMs ?? dueMs(p.timeline, now)), label: `Pregnant${who ? ` with ${who}` : ""}`, seg: (from, to) => {
          const [w1, w2] = [weekOn(p, from), weekOn(p, to)];
          return `🤰 Week ${w1 === w2 ? w1 : `${w1}–${w2}`}${who ? ` · ${who}` : ""}`;
        } });
      if (p.endedMs == null) {
        out.push({ kind: "milestone", a: dayOf(timeAtProgress(p.timeline, now, BABY_REVEAL_WEEK / TERM_WEEKS)), b: 0, label: `✨ Wk ${BABY_REVEAL_WEEK}` });
        out.push({ kind: "milestone", a: dayOf(timeAtProgress(p.timeline, now, GENDER_REVEAL_WEEK / TERM_WEEKS)), b: 0, label: `🎀 Wk ${GENDER_REVEAL_WEEK}` });
        out.push({ kind: "due", a: dayOf(dueMs(p.timeline, now)), b: 0, label: "📅 Due" });
      } else if (p.born) {
        out.push({ kind: "born", a: dayOf(p.endedMs), b: 0, label: "👶 Born" });
      }
    }
    for (const w of d.waiting) {
      out.push({ kind: "waiting", a: dayOf(Math.min(w.timeMs, ...(w.funTimes ?? []))), b: dayOf(w.revealMs), label: "⏳ Waiting" });
    }
    // One chip a day for WooHoos and for achievements.
    const perDay = (times: number[]) => {
      const m = new Map<number, number>();
      for (const t of times) m.set(dayOf(t), (m.get(dayOf(t)) ?? 0) + 1);
      return m;
    };
    for (const [n, c] of perDay(d.funLog.map((f) => f.timeMs))) out.push({ kind: "woohoo", a: n, b: 0, label: c > 1 ? `💕 ×${c}` : "💕 WooHoo" });
    const got = ACHIEVEMENTS.filter((a) => d.achievements?.[a.id] != null && !d.testAchievements?.includes(a.id)).map((a) => d.achievements[a.id]);
    for (const [n, c] of perDay(got)) out.push({ kind: "achievement", a: n, b: 0, label: c > 1 ? `🏆 ×${c}` : "🏆 Trophy" });
    // Fertile days in a row make one bar (not while pregnant).
    let run: number | null = null;
    for (let n = range.start; n <= range.end + 1; n++) {
      const f = n <= range.end && !pregnantOn(n) && fertility(n) != null;
      if (f && run == null) run = n;
      if (!f && run != null) {
        out.push({ kind: "fertile", a: run, b: n - 1, label: "🌸 Fertile" });
        run = null;
      }
    }
    for (const e of out) if (e.b < e.a) e.b = e.a;
    return out;
  });

  /** Each week: its days, and its bars in lanes (like a calendar app). */
  const weeks = $derived.by(() => {
    const order: Kind[] = ["pregnant", "waiting", "fertile", "due", "born", "milestone", "woohoo", "achievement"];
    return Array.from({ length: range.weeks }, (_, w) => {
      const ws = range.start + w * 7;
      const we = ws + 6;
      const here = events
        .filter((e) => e.a <= we && e.b >= ws)
        .sort((x, y) => Math.max(x.a, ws) - Math.max(y.a, ws) || y.b - y.a - (x.b - x.a) || order.indexOf(x.kind) - order.indexOf(y.kind));
      const lanes: boolean[][] = [];
      const bars: { e: Ev; label: string; col: number; span: number; lane: number; cutStart: boolean; cutEnd: boolean }[] = [];
      const hidden = Array(7).fill(0);
      for (const e of here) {
        const a = Math.max(e.a, ws) - ws;
        const b = Math.min(e.b, we) - ws;
        let lane = lanes.findIndex((l) => l.slice(a, b + 1).every((x) => !x));
        if (lane < 0) lane = lanes.push(Array(7).fill(false)) - 1;
        if (lane >= MAX_LANES) {
          for (let i = a; i <= b; i++) hidden[i]++;
          continue;
        }
        for (let i = a; i <= b; i++) lanes[lane][i] = true;
        bars.push({ e, label: e.seg ? e.seg(ws + a, ws + b) : e.label, col: a + 1, span: b - a + 1, lane, cutStart: e.a < ws, cutEnd: e.b > we });
      }
      const days = Array.from({ length: 7 }, (_, i) => {
        const n = ws + i;
        const date = dateOf(n);
        return { n, date: date.getDate(), inMonth: date.getMonth() === shown.m, today: n === dayOf(app.now), more: hidden[i] };
      });
      return { days, bars };
    });
  });

  /** Everything about one day, in words. */
  function linesFor(n: number): Line[] {
    const out: Line[] = [];
    const now = app.now;
    for (const p of pregnancies) {
      const a = dayOf(p.timeline.startMs);
      const b = dayOf(p.endedMs ?? dueMs(p.timeline, now));
      if (n < a || n > b) continue;
      const who = partnerOf(p);
      out.push({ kind: "pregnant", text: `Pregnant, week ${weekOn(p, n)}, ${who ? `with ${who}` : "the other parent isn't known"}${whose(p)}${n === a ? ". The first day!" : ""}` });
      if (p.endedMs == null) {
        if (n === dayOf(timeAtProgress(p.timeline, now, BABY_REVEAL_WEEK / TERM_WEEKS))) out.push({ kind: "milestone", text: `Week ${BABY_REVEAL_WEEK}: find out how many babies` });
        if (n === dayOf(timeAtProgress(p.timeline, now, GENDER_REVEAL_WEEK / TERM_WEEKS))) out.push({ kind: "milestone", text: `Week ${GENDER_REVEAL_WEEK}: find out girl or boy` });
        if (n === b) out.push({ kind: "due", text: "Due date" });
      } else if (p.born && n === b) {
        out.push({ kind: "born", text: p.babyCount > 1 ? `${p.babyCount} babies were born` : "Your baby was born" });
      }
    }
    for (const w of d.waiting) {
      const a = dayOf(Math.min(w.timeMs, ...(w.funTimes ?? [])));
      const r = dayOf(w.revealMs);
      if (n < a || n > r) continue;
      out.push({ kind: "waiting", text: n === r ? `Find out if it took, at ${timeFmt.format(w.revealMs)}` : `Waiting to find out if it took (on ${shortFmt.format(w.revealMs)})` });
    }
    const f = pregnantOn(n) ? null : fertility(n);
    if (f) out.push({ kind: "fertile", text: f === "peak" ? "Fertile: the best chance" : "Fertile: a good chance" });
    for (const e of d.funLog.filter((x) => dayOf(x.timeMs) === n).sort((x, y) => x.timeMs - y.timeMs)) {
      const who = shownName(d, e.partnerName, e.partnerUserId);
      out.push({ kind: "woohoo", text: `WooHoo at ${timeFmt.format(e.timeMs)}${who ? ` with ${who}` : ""}${e.worldName ? ` in ${e.worldName}` : ""}` });
    }
    for (const a of ACHIEVEMENTS) {
      const at = d.achievements?.[a.id];
      if (at != null && !d.testAchievements?.includes(a.id) && dayOf(at) === n) out.push({ kind: "achievement", text: `Achievement: ${a.emoji} ${a.title}` });
    }
    return out;
  }

  function move(by: number) {
    const x = new Date(shown.y, shown.m + by, 1);
    shown = { y: x.getFullYear(), m: x.getMonth() };
    picked = null;
  }
  const isThisMonth = $derived(shown.y === today.getFullYear() && shown.m === today.getMonth());
  const pick = (n: number) => (picked = picked === n ? null : n);
</script>

<section class="card">
  <div class="row head">
    <h2 class="grow">📅 Calendar</h2>
    <button class="small ghost" onclick={() => move(-1)} aria-label="Previous month">‹</button>
    <button class="small ghost month" onclick={() => { shown = { y: today.getFullYear(), m: today.getMonth() }; picked = null; }} disabled={isThisMonth} title="Back to this month">{monthFmt.format(new Date(shown.y, shown.m, 1))}</button>
    <button class="small ghost" onclick={() => move(1)} aria-label="Next month">›</button>
  </div>

  <div class="cal">
    <div class="weekdays">
      {#each weekdays as w, i (i)}<span>{w}</span>{/each}
    </div>
    {#each weeks as wk, wi (wi)}
      <div class="week" style="--lanes:{MAX_LANES}">
        {#each wk.days as day, i (day.n)}
          <button
            class="cell"
            class:out={!day.inMonth}
            class:picked={picked === day.n}
            style="grid-column:{i + 1}"
            onclick={() => pick(day.n)}
            aria-label={dayFmt.format(dateOf(day.n))}
          >
            <span class="num" class:today={day.today}>{day.date}</span>
            {#if day.more}<span class="more">+{day.more}</span>{/if}
          </button>
        {/each}
        {#each wk.bars as bar, bi (bi)}
          <button
            class="bar {bar.e.kind}"
            class:cut-start={bar.cutStart}
            class:cut-end={bar.cutEnd}
            style="grid-column:{bar.col} / span {bar.span};grid-row:{bar.lane + 2}"
            onclick={() => pick(Math.max(bar.e.a, wk.days[0].n))}
            title={bar.e.label}
          >{bar.label}</button>
        {/each}
      </div>
    {/each}
  </div>

  {#if picked != null}
    {@const lines = linesFor(picked)}
    <div class="details">
      <div class="row">
        <strong class="grow">{dayFmt.format(dateOf(picked))}</strong>
        <button class="small ghost" onclick={() => (picked = null)} aria-label="Close">✕</button>
      </div>
      {#each lines as l, i (i)}
        <span class="line"><i class="dot {l.kind}"></i>{l.text}</span>
      {:else}
        <span class="muted small">Nothing on this day.</span>
      {/each}
    </div>
  {:else}
    <p class="muted small tip">Tap a day to see what happened.</p>
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
  .cal {
    border-radius: 12px;
    overflow: hidden;
    background: var(--bg);
  }
  .weekdays,
  .week {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
  }
  .weekdays span {
    text-align: center;
    font-size: 0.72rem;
    font-weight: 700;
    color: var(--muted);
    padding: 4px 0;
  }
  .week {
    /* Row 1: the date; then one row per lane of bars; then room for "+N". */
    grid-template-rows: 22px repeat(var(--lanes), 18px) 14px;
    row-gap: 2px;
    border-top: 1px solid color-mix(in srgb, var(--muted) 18%, transparent);
  }
  .cell {
    all: unset;
    box-sizing: border-box;
    cursor: pointer;
    grid-row: 1 / -1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
    padding: 2px 0 1px;
    border-left: 1px solid color-mix(in srgb, var(--muted) 12%, transparent);
  }
  .cell:first-child {
    border-left: none;
  }
  .cell:hover,
  .cell:focus-visible {
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }
  .cell.picked {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
  }
  .cell.out .num {
    opacity: 0.4;
  }
  .num {
    font-size: 0.76rem;
    font-weight: 700;
    min-width: 20px;
    text-align: center;
    line-height: 20px;
    border-radius: 999px;
  }
  .num.today {
    background: var(--accent);
    color: #fff;
  }
  .more {
    font-size: 0.66rem;
    font-weight: 700;
    color: var(--muted);
    line-height: 1;
  }
  .bar {
    all: unset;
    box-sizing: border-box;
    cursor: pointer;
    z-index: 1;
    margin: 0 2px;
    padding: 0 5px;
    border-radius: 6px;
    font-size: 0.68rem;
    font-weight: 700;
    line-height: 18px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text);
    background: color-mix(in srgb, var(--c) 35%, var(--bg-2));
    border-left: 3px solid var(--c);
  }
  .bar.cut-start {
    margin-left: 0;
    border-left: none;
    border-top-left-radius: 0;
    border-bottom-left-radius: 0;
  }
  .bar.cut-end {
    margin-right: 0;
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
  }
  .bar:hover,
  .bar:focus-visible {
    background: color-mix(in srgb, var(--c) 55%, var(--bg-2));
  }
  /* One colour per kind of thing. */
  .pregnant {
    --c: var(--accent);
  }
  .fertile {
    --c: var(--tri3);
  }
  .waiting {
    --c: var(--warn);
  }
  .woohoo {
    --c: var(--danger);
  }
  .milestone {
    --c: var(--tri2);
  }
  .due {
    --c: var(--tier-rare);
  }
  .born {
    --c: var(--ok);
  }
  .achievement {
    --c: var(--tier-legendary);
  }
  .details {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 10px;
    padding: 8px 10px;
    border-radius: 12px;
    background: var(--bg);
  }
  .line {
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-size: 0.85rem;
  }
  .dot {
    flex: none;
    width: 9px;
    height: 9px;
    border-radius: 3px;
    background: var(--c);
  }
  .tip {
    margin: 8px 0 0;
    text-align: center;
  }
</style>
