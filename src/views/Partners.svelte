<script lang="ts">
  import PersonPic from "../components/PersonPic.svelte";
  import { app, confirmAsk, errorToast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { partnerStats, personKey, type PartnerStats } from "../lib/scope";
  import type { Settings } from "../lib/types";
  import { humanDuration, progress, TERM_WEEKS } from "../lib/timeline";

  const list = $derived(app.data ? partnerStats(app.data) : []);
  const totals = $derived({
    // Every WooHoo, also the ones where nobody knows who it was.
    fun: app.data?.funLog.length ?? 0,
    pregnancies: list.reduce((s, x) => s + x.pregnancies, 0),
    kids: list.reduce((s, x) => s + x.kids, 0),
  });
  const dateFmt = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", year: "numeric" });
  /** The search box shows up once the list is this long. */
  const SEARCH_FROM = 6;
  let query = $state("");
  const shown = $derived(query.trim() ? list.filter((x) => x.name.toLowerCase().includes(query.trim().toLowerCase())) : list);
  /** What's going on with each partner right now: pregnancy weeks, or a wait to find out. */
  const now = $derived.by(() => {
    const out = new Map<string, { weeks: number[]; waiting: boolean }>();
    const at = (name: string | null, userId: string | null) => {
      const k = personKey(name, userId);
      if (!out.has(k)) out.set(k, { weeks: [], waiting: false });
      return out.get(k)!;
    };
    for (const p of app.data?.pregnancies ?? []) {
      if (!p.endedMs && p.partner.name) at(p.partner.name, p.partner.userId).weeks.push(Math.floor(progress(p.timeline, app.now) * TERM_WEEKS));
    }
    for (const w of app.data?.waiting ?? []) {
      if (w.partner.name) at(w.partner.name, w.partner.userId).waiting = true;
    }
    return out;
  });
  const pins = $derived(app.data?.settings.pinnedPartners ?? []);
  const pinned = $derived(shown.filter((x) => pins.includes(x.key)));
  const others = $derived(shown.filter((x) => !pins.includes(x.key)));

  async function togglePin(s: PartnerStats) {
    if (!app.data) return;
    const next = pins.includes(s.key) ? pins.filter((k) => k !== s.key) : [...pins, s.key];
    const settings: Settings = { ...($state.snapshot(app.data.settings) as Settings), pinnedPartners: next };
    try {
      await api.saveSettings(settings);
      app.data.settings.pinnedPartners = next;
    } catch (e) {
      errorToast(e);
    }
  }

  async function forget(name: string, userId: string | null) {
    if (await confirmAsk({ title: `Forget ${name}?`, text: "They're removed from your partners, WooHoo history and instance history. Pregnancies with them carry on with an unknown partner.", ok: "Forget", danger: true })) {
      await api.forgetPerson(name, userId);
    }
  }
</script>

{#snippet card(s: PartnerStats, isPinned: boolean)}
  <section class="card partner" class:pinned={isPinned}>
    <div class="row">
      <PersonPic name={s.name} userId={s.userId} size={52} />
      <div class="grow">
        <strong class="name">{s.name}</strong>
        {#if now.get(s.key)?.weeks.length}
          {@const weeks = now.get(s.key)!.weeks}
          <span class="pill live">🤰 {weeks.length > 1 ? `${weeks.length} pregnancies together` : `Week ${weeks[0]} together`}</span>
        {:else if now.get(s.key)?.waiting}
          <span class="pill live">💕 Waiting to find out</span>
        {/if}
        <div class="muted small">
          Last time {app.now - s.lastMs < 60_000 ? "just now" : `${humanDuration(app.now - s.lastMs)} ago`}{s.firstMs !== s.lastMs ? ` · first ${dateFmt.format(s.firstMs)}` : ""}
        </div>
      </div>
      <button class="pin" class:on={isPinned} onclick={() => togglePin(s)} aria-pressed={isPinned} aria-label="{isPinned ? 'Unpin' : 'Pin'} {s.name}" title={isPinned ? "Unpin" : "Pin to the top"}>📌</button>
      {#if s.userId || s.name}
        <button class="ghost small" onclick={() => forget(s.name, s.userId)} aria-label="Forget {s.name}">Forget</button>
      {/if}
    </div>
    <div class="nums">
      <div><strong>{s.fun}</strong><span>💕 WooHoos</span></div>
      <div><strong>{s.pregnancies}</strong><span>🤰 pregnant</span></div>
      <div><strong>{s.kids}</strong><span>👶 kids</span></div>
    </div>
  </section>
{/snippet}

<div class="page">
  <section class="card totals">
    <div><strong>{totals.fun}</strong><span class="muted small">💕 WooHoos</span></div>
    <div><strong>{totals.pregnancies}</strong><span class="muted small">🤰 pregnancies</span></div>
    <div><strong>{totals.kids}</strong><span class="muted small">👶 kids</span></div>
  </section>

  {#if list.length === 0}
    <section class="card empty">
      <div class="big" aria-hidden="true">💞</div>
      <p class="muted">Partners show up here after a WooHoo in-game, or when you pick a partner for a pregnancy.</p>
    </section>
  {/if}

  {#if list.length >= SEARCH_FROM}
    <input class="search" type="search" bind:value={query} placeholder="🔍 Search {list.length} partners" aria-label="Search partners" />
    {#if !shown.length}<p class="muted small none">No partner called "{query.trim()}".</p>{/if}
  {/if}

  {#if pinned.length}
    <h3 class="shelf">📌 Pinned</h3>
    {#each pinned as s (s.key)}{@render card(s, true)}{/each}
    {#if others.length}<h3 class="shelf">Everyone else</h3>{/if}
  {/if}
  {#each others as s (s.key)}{@render card(s, false)}{/each}
</div>

<style>
  .page {
    max-width: 560px;
    margin: 0 auto;
    padding: 6px 16px 40px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .search {
    width: 100%;
  }
  .none {
    margin: 0;
    text-align: center;
  }
  .totals,
  .nums {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
    text-align: center;
  }
  .totals div,
  .nums div {
    display: flex;
    flex-direction: column;
  }
  .totals strong {
    font-size: 1.8rem;
    font-weight: 900;
  }
  .nums {
    margin-top: 12px;
  }
  .nums div {
    background: var(--bg);
    border-radius: 14px;
    padding: 8px 4px;
  }
  .nums strong {
    font-size: 1.3rem;
  }
  .nums span {
    font-size: 0.78rem;
    color: var(--muted);
  }
  .name {
    font-size: 1.1rem;
  }
  .live {
    background: var(--accent);
    color: var(--accent-text);
    margin-left: 4px;
    vertical-align: 2px;
  }
  .shelf {
    margin: 4px 4px -6px;
    font-size: 0.9rem;
    color: var(--muted);
  }
  .pinned {
    box-shadow: 0 0 0 2px var(--accent);
  }
  .pin {
    background: none;
    border: none;
    box-shadow: none;
    padding: 4px 6px;
    font-size: 1.1rem;
    filter: grayscale(1);
    opacity: 0.45;
  }
  .pin:hover {
    opacity: 0.8;
  }
  .pin.on {
    filter: none;
    opacity: 1;
  }
  .empty {
    text-align: center;
  }
  .big {
    font-size: 2.6rem;
  }
</style>
