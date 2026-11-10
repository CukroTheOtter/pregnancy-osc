<script lang="ts">
  // Me tab: a month calendar of your story, laid out like a normal calendar
  // app: labelled bars that run across the days they cover (pregnant,
  // fertile, waiting) and labelled chips for one-day things. Tap a day for
  // the full story of that day. Kinds can be hidden (remembered on this
  // computer), and the calendar can be exported for other calendar apps
  // (names left out). Reads only what's saved.
  import Section from "./Section.svelte";
  import Modal from "./Modal.svelte";
  import JournalList from "./JournalList.svelte";
  import { app, errorToast, toast } from "../lib/app.svelte";
  import { saveTextFile } from "../lib/api";
  import { shownName } from "../lib/privacy";
  import { avatarName, isStopped, pregnancyAvatar } from "../lib/scope";
  import { ACHIEVEMENTS } from "../lib/achievements";
  import { journalFor } from "../lib/journal";
  import { toIcs, type IcsEvent } from "../lib/ics";
  import { BABY_REVEAL_WEEK, GENDER_REVEAL_WEEK, chance, cycleDay, cycleMs, level } from "../lib/fertility";
  import { TERM_WEEKS, dueMs, timeAtProgress } from "../lib/timeline";
  import type { Pregnancy } from "../lib/types";

  type Kind = "pregnant" | "fertile" | "waiting" | "woohoo" | "journal" | "milestone" | "due" | "born" | "achievement";
  /**
   * A bar or chip from day `a` to day `b` (both included, as day numbers since
   * 1970 in local time). `label` is for the screen, `title` for the export
   * (never a name).
   */
  type Ev = { kind: Kind; a: number; b: number; label: string; title: string; seg?: (from: number, to: number) => string; where?: string };
  /** One line in a day's details; `open` links to a journal entry. */
  type Line = { kind: Kind; text: string; open?: { p: Pregnancy; key: string } };

  const KINDS: { kind: Kind; name: string }[] = [
    { kind: "pregnant", name: "🤰 Pregnant" },
    { kind: "fertile", name: "🌸 Fertile" },
    { kind: "waiting", name: "⏳ Waiting" },
    { kind: "woohoo", name: "💕 WooHoos" },
    { kind: "journal", name: "📔 Journal" },
    { kind: "milestone", name: "✨ Weeks 12 & 20" },
    { kind: "due", name: "📅 Due date" },
    { kind: "born", name: "👶 Born" },
    { kind: "achievement", name: "🏆 Achievements" },
  ];
  const MAX_LANES = 3;
  const DAY = 86_400_000;
  /** Fertile days are a guess about the future: shown this far ahead in the export. */
  const EXPORT_AHEAD_DAYS = 90;

  const d = $derived(app.data!);
  const today = new Date(app.now);
  let shown = $state({ y: today.getFullYear(), m: today.getMonth() });
  let picked = $state<number | null>(null);
  /** The journal open from the calendar (scrolled to that entry). */
  let reading = $state<{ p: Pregnancy; key: string } | null>(null);

  // Hidden kinds, remembered on this computer.
  const HIDDEN_KEY = "pregsys-calendar-hidden";
  let hidden = $state<Kind[]>(
    (() => {
      try {
        return JSON.parse(localStorage.getItem(HIDDEN_KEY) ?? "[]");
      } catch {
        return [];
      }
    })(),
  );
  function toggleKind(k: Kind) {
    hidden = hidden.includes(k) ? hidden.filter((x) => x !== k) : [...hidden, k];
    try {
      localStorage.setItem(HIDDEN_KEY, JSON.stringify(hidden));
    } catch {
      // Not remembered this time.
    }
  }
  const on = (k: Kind) => !hidden.includes(k);
  function showAll() {
    hidden = [];
    try {
      localStorage.removeItem(HIDDEN_KEY);
    } catch {
      // Nothing to forget.
    }
  }
  /** The "Show" panel is open. */
  let filtering = $state(false);

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
  // Several pregnancies (per-avatar mode): the day's details say which avatar.
  const whose = (p: Pregnancy) => (pregnancies.length > 1 ? ` (${avatarName(d, pregnancyAvatar(d, p))})` : "");
  /** Who the other parent is (as shown: privacy mode applies), or null. */
  const partnerOf = (p: Pregnancy) => shownName(d, p.partner?.name ?? null, p.partner?.userId ?? null);
  /** The pregnancy week on a day, counted like Home and the journal (0 to 40). */
  const weekOn = (p: Pregnancy, n: number) => Math.min(TERM_WEEKS, Math.max(0, Math.floor(((noonOf(n) - p.timeline.startMs) / p.timeline.durationMs) * TERM_WEEKS)));
  const lastDay = (p: Pregnancy) => dayOf(p.endedMs ?? dueMs(p.timeline, app.now));

  // Journal entries by pregnancy (the birth and due date have their own chips).
  const journals = $derived(pregnancies.map((p) => ({ p, items: journalFor(d, p, app.now).filter((j) => j.key !== "born" && j.key !== "due") })));

  // Fertile days only make sense when a cycle lasts at least two weeks (fast
  // pregnancies have cycles of hours, too short to show by the day).
  const cycle = $derived(d.settings.fertility && d.settings.cycleAnchorMs != null && cycleMs(d.settings) >= 14 * DAY ? cycleMs(d.settings) : null);
  function fertility(n: number): "peak" | "high" | null {
    if (cycle == null) return null;
    const peak = d.settings.fertilityPeak;
    const l = level(chance(cycleDay(noonOf(n), d.settings.cycleAnchorMs!, cycle), peak), peak);
    return l === "peak" || l === "high" ? l : null;
  }
  const pregnantOn = (n: number) => pregnancies.some((p) => dayOf(p.timeline.startMs) <= n && n <= lastDay(p));

  /** Everything between two days (fertile days only from `fertileFrom`). */
  function eventsBetween(from: number, to: number, fertileFrom = from): Ev[] {
    const out: Ev[] = [];
    const now = app.now;
    const todayN = dayOf(now);
    for (const p of pregnancies) {
      const a = dayOf(p.timeline.startMs);
      const who = partnerOf(p);
      // Each week's piece of the bar says which weeks those are, and with whom.
      out.push({
        kind: "pregnant",
        a,
        b: lastDay(p),
        label: `Pregnant${who ? ` with ${who}` : ""}`,
        title: "🤰 Pregnant",
        seg: (s, e) => {
          const [w1, w2] = [weekOn(p, s), weekOn(p, e)];
          return `🤰 Week ${w1 === w2 ? w1 : `${w1}–${w2}`}${who ? ` · ${who}` : ""}`;
        },
      });
      if (p.endedMs == null) {
        // Weeks 12 and 20 until they come (then the journal's scan covers them).
        for (const [w, e, what] of [[BABY_REVEAL_WEEK, "✨", "how many babies"], [GENDER_REVEAL_WEEK, "🎀", "girl or boy"]] as const) {
          const n = dayOf(timeAtProgress(p.timeline, now, w / TERM_WEEKS));
          if (n >= todayN) out.push({ kind: "milestone", a: n, b: n, label: `${e} Wk ${w}`, title: `${e} Week ${w}: ${what}` });
        }
        const due = lastDay(p);
        out.push({ kind: "due", a: due, b: due, label: "📅 Due", title: "📅 Due date" });
      } else if (p.born) {
        out.push({ kind: "born", a: lastDay(p), b: lastDay(p), label: "👶 Born", title: p.babyCount > 1 ? "👶 Babies born" : "👶 Baby born" });
      }
    }
    for (const w of d.waiting) {
      const a = dayOf(Math.min(w.timeMs, ...(w.funTimes ?? [])));
      out.push({ kind: "waiting", a, b: dayOf(w.revealMs), label: "⏳ Waiting", title: "⏳ Waiting to find out" });
    }
    // One chip a day for WooHoos, journal entries and achievements.
    const perDay = <T,>(xs: T[], time: (x: T) => number) => {
      const m = new Map<number, T[]>();
      for (const x of xs) {
        const n = dayOf(time(x));
        m.set(n, [...(m.get(n) ?? []), x]);
      }
      return m;
    };
    for (const [n, fs] of perDay(d.funLog, (f) => f.timeMs)) {
      const worlds = [...new Set(fs.map((f) => f.worldName).filter(Boolean))].join(", ");
      out.push({ kind: "woohoo", a: n, b: n, label: fs.length > 1 ? `💕 ×${fs.length}` : "💕 WooHoo", title: fs.length > 1 ? `💕 ${fs.length} WooHoos` : "💕 WooHoo", where: worlds || undefined });
    }
    for (const [n, js] of perDay(journals.flatMap((j) => j.items), (j) => j.timeMs)) {
      out.push({ kind: "journal", a: n, b: n, label: js.length > 1 ? `📔 ×${js.length}` : "📔 Journal", title: `📔 ${js.map((j) => j.title || "Note").join(", ")}` });
    }
    const got = ACHIEVEMENTS.filter((a) => d.achievements?.[a.id] != null && !d.testAchievements?.includes(a.id));
    for (const [n, as] of perDay(got, (a) => d.achievements[a.id])) {
      out.push({ kind: "achievement", a: n, b: n, label: as.length > 1 ? `🏆 ×${as.length}` : "🏆 Trophy", title: `🏆 ${as.map((a) => a.title).join(", ")}` });
    }
    // Fertile days in a row make one bar (not while pregnant).
    let run: number | null = null;
    for (let n = Math.max(from, fertileFrom); n <= to + 1; n++) {
      const f = n <= to && !pregnantOn(n) && fertility(n) != null;
      if (f && run == null) run = n;
      if (!f && run != null) {
        out.push({ kind: "fertile", a: run, b: n - 1, label: "🌸 Fertile", title: "🌸 Fertile days" });
        run = null;
      }
    }
    return out.filter((e) => on(e.kind) && e.a <= to && Math.max(e.a, e.b) >= from).map((e) => ({ ...e, b: Math.max(e.a, e.b) }));
  }

  /** The days on screen: whole weeks around the shown month. */
  const range = $derived.by(() => {
    const first = dayOf(new Date(shown.y, shown.m, 1).getTime());
    const last = dayOf(new Date(shown.y, shown.m + 1, 0).getTime());
    const lead = (dateOf(first).getDay() - firstDay + 7) % 7;
    const start = first - lead;
    const weeks = Math.ceil((last - start + 1) / 7);
    return { start, end: start + weeks * 7 - 1, weeks };
  });

  /** Each week: its days, and its bars in lanes (like a calendar app). */
  const weeks = $derived.by(() => {
    const events = eventsBetween(range.start, range.end);
    const order = KINDS.map((k) => k.kind);
    return Array.from({ length: range.weeks }, (_, w) => {
      const ws = range.start + w * 7;
      const we = ws + 6;
      const here = events
        .filter((e) => e.a <= we && e.b >= ws)
        .sort((x, y) => Math.max(x.a, ws) - Math.max(y.a, ws) || y.b - y.a - (x.b - x.a) || order.indexOf(x.kind) - order.indexOf(y.kind));
      const lanes: boolean[][] = [];
      const bars: { e: Ev; label: string; col: number; span: number; lane: number; cutStart: boolean; cutEnd: boolean }[] = [];
      const more = Array(7).fill(0);
      for (const e of here) {
        const a = Math.max(e.a, ws) - ws;
        const b = Math.min(e.b, we) - ws;
        let lane = lanes.findIndex((l) => l.slice(a, b + 1).every((x) => !x));
        if (lane < 0) lane = lanes.push(Array(7).fill(false)) - 1;
        if (lane >= MAX_LANES) {
          for (let i = a; i <= b; i++) more[i]++;
          continue;
        }
        for (let i = a; i <= b; i++) lanes[lane][i] = true;
        bars.push({ e, label: e.seg ? e.seg(ws + a, ws + b) : e.label, col: a + 1, span: b - a + 1, lane, cutStart: e.a < ws, cutEnd: e.b > we });
      }
      const days = Array.from({ length: 7 }, (_, i) => {
        const n = ws + i;
        const date = dateOf(n);
        return { n, date: date.getDate(), inMonth: date.getMonth() === shown.m, today: n === dayOf(app.now), more: more[i] };
      });
      return { days, bars };
    });
  });

  /** Everything about one day, in words. */
  function linesFor(n: number): Line[] {
    const out: Line[] = [];
    const now = app.now;
    const todayN = dayOf(now);
    for (const p of pregnancies) {
      const a = dayOf(p.timeline.startMs);
      const b = lastDay(p);
      if (n < a || n > b) continue;
      const who = partnerOf(p);
      if (on("pregnant")) out.push({ kind: "pregnant", text: `Pregnant, week ${weekOn(p, n)}, ${who ? `with ${who}` : "the other parent isn't known"}${whose(p)}${n === a ? ". The first day!" : ""}` });
      if (p.endedMs == null) {
        if (on("milestone") && n >= todayN && n === dayOf(timeAtProgress(p.timeline, now, BABY_REVEAL_WEEK / TERM_WEEKS))) out.push({ kind: "milestone", text: `Week ${BABY_REVEAL_WEEK}: find out how many babies` });
        if (on("milestone") && n >= todayN && n === dayOf(timeAtProgress(p.timeline, now, GENDER_REVEAL_WEEK / TERM_WEEKS))) out.push({ kind: "milestone", text: `Week ${GENDER_REVEAL_WEEK}: find out girl or boy` });
        if (on("due") && n === b) out.push({ kind: "due", text: "Due date" });
      } else if (on("born") && p.born && n === b) {
        out.push({ kind: "born", text: p.babyCount > 1 ? `${p.babyCount} babies were born` : "Your baby was born" });
      }
    }
    if (on("waiting")) {
      for (const w of d.waiting) {
        const a = dayOf(Math.min(w.timeMs, ...(w.funTimes ?? [])));
        const r = dayOf(w.revealMs);
        if (n < a || n > r) continue;
        out.push({ kind: "waiting", text: n === r ? `Find out if it took, at ${timeFmt.format(w.revealMs)}` : `Waiting to find out if it took (on ${shortFmt.format(w.revealMs)})` });
      }
    }
    const f = on("fertile") && !pregnantOn(n) ? fertility(n) : null;
    if (f) out.push({ kind: "fertile", text: f === "peak" ? "Fertile: the best chance" : "Fertile: a good chance" });
    if (on("woohoo")) {
      for (const e of d.funLog.filter((x) => dayOf(x.timeMs) === n).sort((x, y) => x.timeMs - y.timeMs)) {
        const who = shownName(d, e.partnerName, e.partnerUserId);
        out.push({ kind: "woohoo", text: `WooHoo at ${timeFmt.format(e.timeMs)}${who ? ` with ${who}` : ""}${e.worldName ? ` in ${e.worldName}` : ""}` });
      }
    }
    if (on("journal")) {
      for (const { p, items } of journals) {
        for (const j of items.filter((x) => dayOf(x.timeMs) === n).sort((x, y) => x.timeMs - y.timeMs)) {
          out.push({ kind: "journal", text: `New journal entry: ${j.emoji} ${j.title || "Note"}${whose(p)}`, open: { p, key: j.key } });
        }
      }
    }
    if (on("achievement")) {
      for (const a of ACHIEVEMENTS) {
        const at = d.achievements?.[a.id];
        if (at != null && !d.testAchievements?.includes(a.id) && dayOf(at) === n) out.push({ kind: "achievement", text: `Achievement: ${a.emoji} ${a.title}` });
      }
    }
    return out;
  }

  /** A calendar file of everything shown (the kinds that are on), without anyone's name. */
  async function exportIcs() {
    const todayN = dayOf(app.now);
    const times = [...d.funLog.map((f) => f.timeMs), ...pregnancies.map((p) => p.timeline.startMs), ...Object.values(d.achievements ?? {})];
    const from = times.length ? dayOf(Math.min(...times)) : todayN;
    const to = Math.max(todayN + EXPORT_AHEAD_DAYS, ...pregnancies.map(lastDay), ...d.waiting.map((w) => dayOf(w.revealMs)));
    const events = eventsBetween(from, to, todayN);
    if (!events.length) {
      toast("Nothing to export yet", "Things show up once they happen (or turn some back on).", "info");
      return;
    }
    const list: IcsEvent[] = events.map((e, i) => ({ uid: `${e.kind}-${e.a}-${i}`, start: dateOf(e.a), end: dateOf(e.b), title: e.title, location: e.where }));
    try {
      if (await saveTextFile("pregnancy-calendar", toIcs(list), "ics")) toast("Calendar saved 📅", "Import it in Google Calendar, Outlook or Apple Calendar.", "success");
    } catch (e) {
      errorToast(e);
    }
  }

  function move(by: number) {
    const x = new Date(shown.y, shown.m + by, 1);
    shown = { y: x.getFullYear(), m: x.getMonth() };
    picked = null;
  }
  const isThisMonth = $derived(shown.y === today.getFullYear() && shown.m === today.getMonth());
  const pick = (n: number) => (picked = picked === n ? null : n);
</script>

<Section id="me-calendar" title="📅 Calendar" open>
  <div class="row nav">
    <div class="row months">
      <button class="small ghost" onclick={() => move(-1)} aria-label="Previous month">‹</button>
      <button class="small ghost month" onclick={() => { shown = { y: today.getFullYear(), m: today.getMonth() }; picked = null; }} disabled={isThisMonth} title="Back to this month">{monthFmt.format(new Date(shown.y, shown.m, 1))}</button>
      <button class="small ghost" onclick={() => move(1)} aria-label="Next month">›</button>
    </div>
    <button class="small soft show" class:open={filtering} aria-expanded={filtering} onclick={() => (filtering = !filtering)}>
      Show{#if hidden.length}<span class="count">{KINDS.length - hidden.length}/{KINDS.length}</span>{/if}
      <span class="caret" aria-hidden="true">▾</span>
    </button>
  </div>

  {#if filtering}
    <div class="kinds" role="group" aria-label="What the calendar shows">
      {#each KINDS as k (k.kind)}
        <button class="kind {k.kind}" class:off={!on(k.kind)} role="checkbox" aria-checked={on(k.kind)} onclick={() => toggleKind(k.kind)}>
          <span class="check" aria-hidden="true">{on(k.kind) ? "✓" : ""}</span>
          <span class="name">{k.name}</span>
        </button>
      {/each}
      {#if hidden.length}
        <button class="link small all" onclick={showAll}>Show everything</button>
      {/if}
    </div>
  {/if}

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
        <span class="line">
          <i class="dot {l.kind}"></i>
          <span class="grow">{l.text}</span>
          {#if l.open}{@const o = l.open}<button class="small soft" onclick={() => (reading = o)}>Open</button>{/if}
        </span>
      {:else}
        <span class="muted small">Nothing on this day.</span>
      {/each}
    </div>
  {/if}

  <div class="row foot">
    <span class="muted small grow">{picked == null ? "Tap a day to see what happened." : ""}</span>
    <button class="small ghost" onclick={exportIcs} title="A calendar file for Google Calendar, Outlook or Apple Calendar. Names are left out.">📤 Export</button>
  </div>
</Section>

{#if reading}
  {@const r = reading}
  <Modal title="📔 Journal" onclose={() => (reading = null)}>
    <JournalList p={r.p} items={journalFor(d, r.p, app.now)} focus={r.key} />
  </Modal>
{/if}

<style>
  .nav {
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }
  .months {
    gap: 2px;
  }
  .show {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .count {
    font-size: 0.7rem;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--accent);
    color: #fff;
  }
  .caret {
    font-size: 0.7rem;
    transition: transform 0.15s;
  }
  .show.open .caret {
    transform: rotate(180deg);
  }
  .month {
    min-width: 9.5em;
  }
  .month:disabled {
    opacity: 1;
    cursor: default;
  }
  .kinds {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 4px;
    padding: 8px;
    border-radius: 12px;
    background: var(--bg);
  }
  .kind {
    all: unset;
    box-sizing: border-box;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-radius: 10px;
    font-size: 0.82rem;
    font-weight: 700;
    color: var(--text);
    transition: background 0.12s, opacity 0.12s;
  }
  .kind:hover,
  .kind:focus-visible {
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
  .kind:focus-visible {
    outline: 2px solid var(--accent);
  }
  .check {
    flex: none;
    width: 18px;
    height: 18px;
    border-radius: 6px;
    display: grid;
    place-items: center;
    font-size: 0.72rem;
    font-weight: 900;
    color: color-mix(in srgb, var(--c), #000 60%);
    background: var(--c);
    border: 2px solid var(--c);
    box-sizing: border-box;
  }
  .kind.off .check {
    background: transparent;
  }
  .kind.off .name {
    opacity: 0.55;
  }
  .all {
    grid-column: 1 / -1;
    justify-self: start;
    margin: 2px 8px 0;
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
  .journal {
    --c: var(--tri1);
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
    padding: 8px 10px;
    border-radius: 12px;
    background: var(--bg);
  }
  .line {
    display: flex;
    align-items: center;
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
  .foot {
    align-items: center;
  }
</style>
