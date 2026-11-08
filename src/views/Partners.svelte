<script lang="ts">
  import { shownName } from "../lib/privacy";
  import PersonPic from "../components/PersonPic.svelte";
  import PartnerPregnancies from "../components/PartnerPregnancies.svelte";
  import PartnerWooHoos from "../components/PartnerWooHoos.svelte";
  import PersonSearch from "../components/PersonSearch.svelte";
  import { app, confirmAsk, errorToast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { partnerStats, personKey, type PartnerStats } from "../lib/scope";
  import type { Person } from "../lib/types";
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
  // The partner card that's open, and which list (one at a time).
  let opened = $state<{ key: string; what: "pregnancies" | "woohoos" } | null>(null);
  function toggle(key: string, what: "pregnancies" | "woohoos") {
    opened = opened?.key === key && opened.what === what ? null : { key, what };
  }
  // Your partners (💞, added by hand) on top, then everyone else from your history.
  const mine = $derived(shown.filter((x) => x.mine));
  const others = $derived(shown.filter((x) => !x.mine));

  let adding = $state(false);
  // To pick from: who's here now, then people met before (past partners come too).
  const here = $derived((app.data?.instance.present ?? []).filter((x) => x.userId !== app.data?.instance.local?.userId));
  const met = $derived(app.data?.instance.history ?? []);

  async function setMine(name: string, userId: string | null, on: boolean) {
    if (!on && !(await confirmAsk({ title: `Remove ${shownName(app.data, name, userId)} from your partners?`, text: app.data?.settings.partnersOnly ? "Their history stays. With Only my partners on, WooHoos with them won't count any more." : "Their history stays; they move down to the others.", ok: "Remove" }))) return;
    try {
      await api.setMyPartner(name, userId, on);
    } catch (e) {
      errorToast(e);
    }
  }

  async function add(person: Person | null, name: string | null) {
    const n = person?.name ?? name;
    if (!n) return;
    await setMine(n, person?.userId ?? null, true);
    adding = false;
  }

  async function forget(name: string, userId: string | null) {
    if (await confirmAsk({ title: `Forget ${shownName(app.data, name, userId)}?`, text: "They're removed from your partners, WooHoo history and instance history. Pregnancies with them carry on with an unknown partner.", ok: "Forget", danger: true })) {
      await api.forgetPerson(name, userId);
    }
  }
</script>

{#snippet card(s: PartnerStats)}
  {@const hasPreg = !!s.pregnancies || !!now.get(s.key)?.waiting}
  {@const live = (now.get(s.key)?.weeks.length ?? 0) + (now.get(s.key)?.waiting ? 1 : 0)}
  {@const showing = opened?.key === s.key ? opened.what : null}
  <section class="card partner" class:mine={s.mine}>
    <div class="row">
      <PersonPic name={s.name} userId={s.userId} size={52} />
      <div class="grow">
        <strong class="name">{shownName(app.data, s.name, s.userId)}</strong>
        {#if now.get(s.key)?.weeks.length}
          {@const weeks = now.get(s.key)!.weeks}
          <span class="pill live">🤰 {weeks.length > 1 ? `${weeks.length} pregnancies together` : `Week ${weeks[0]} together`}</span>
        {:else if now.get(s.key)?.waiting}
          <span class="pill live">💕 Waiting to find out</span>
        {/if}
        <div class="muted small">
          {#if !s.fun && !s.pregnancies}
            Added {dateFmt.format(s.firstMs)} · nothing yet
          {:else}
            Last time {app.now - s.lastMs < 60_000 ? "just now" : `${humanDuration(app.now - s.lastMs)} ago`}{s.firstMs !== s.lastMs ? ` · first ${dateFmt.format(s.firstMs)}` : ""}
          {/if}
        </div>
      </div>
      {#if s.mine}
        <button class="heart" onclick={() => setMine(s.name, s.userId, false)} aria-label="Remove {shownName(app.data, s.name, s.userId)} from your partners" title="Your partner (tap to remove)">💞</button>
      {:else}
        <button class="soft small" onclick={() => setMine(s.name, s.userId, true)} aria-label="Make {shownName(app.data, s.name, s.userId)} your partner">💞 Make partner</button>
      {/if}
      {#if s.userId || s.name}
        <button class="ghost small" onclick={() => forget(s.name, s.userId)} aria-label="Forget {shownName(app.data, s.name, s.userId)}">Forget</button>
      {/if}
    </div>
    <div class="nums">
      <div><strong>{s.fun}</strong><span>💕 WooHoos</span></div>
      <div><strong>{s.pregnancies}</strong><span>🤰 pregnant</span></div>
      <div><strong>{s.kids}</strong><span>👶 kids</span></div>
    </div>
    {#if hasPreg || s.fun}
      <div class="mores">
        {#if hasPreg}
          <button class="soft small more" class:on={showing === "pregnancies"} onclick={() => toggle(s.key, "pregnancies")} aria-expanded={showing === "pregnancies"}>
            🤰 Pregnancies{live ? ` (${live} now)` : ""}
            <span class="chev" class:open={showing === "pregnancies"} aria-hidden="true">›</span>
          </button>
        {/if}
        {#if s.fun}
          <button class="soft small more" class:on={showing === "woohoos"} onclick={() => toggle(s.key, "woohoos")} aria-expanded={showing === "woohoos"}>
            💕 WooHoo history
            <span class="chev" class:open={showing === "woohoos"} aria-hidden="true">›</span>
          </button>
        {/if}
      </div>
      {#if showing === "pregnancies"}<PartnerPregnancies partnerKey={s.key} />{/if}
      {#if showing === "woohoos"}<PartnerWooHoos partnerKey={s.key} />{/if}
    {/if}
  </section>
{/snippet}

<div class="page">
  {#if list.length || totals.fun}
    <section class="card totals">
      <div><strong>{totals.fun}</strong><span class="muted small">💕 WooHoos</span></div>
      <div><strong>{totals.pregnancies}</strong><span class="muted small">🤰 pregnancies</span></div>
      <div><strong>{totals.kids}</strong><span class="muted small">👶 kids</span></div>
    </section>
  {/if}

  {#if adding}
    <section class="card adder">
      <strong>Add a partner</strong>
      <span class="muted small">Pick someone you've met, or type a name.</span>
      <PersonSearch candidates={here} {met} onpick={add} />
      <button class="ghost small" onclick={() => (adding = false)}>Cancel</button>
    </section>
  {:else}
    <button class="soft add" onclick={() => (adding = true)}>＋ Add partner</button>
  {/if}

  {#if list.length >= SEARCH_FROM}
    <input class="search" type="search" bind:value={query} placeholder="🔍 Search {list.length} people" aria-label="Search partners" />
    {#if !shown.length}<p class="muted small none">Nobody called "{query.trim()}".</p>{/if}
  {/if}

  {#if !query.trim() || mine.length}
    <h3 class="shelf">💞 My partners</h3>
  {/if}
  {#each mine as s (s.key)}{@render card(s)}{/each}
  {#if !mine.length && !query.trim()}
    <section class="card empty">
      <p class="muted small">
        {#if app.data?.settings.partnersOnly}⚠️ Only WooHoos with your partners count (Settings), and you have none yet.{:else}Nobody yet.{/if}
        Add someone{others.length ? ", or tap 💞 Make partner on anyone below" : ""}.
      </p>
    </section>
  {/if}

  {#if others.length}
    <h3 class="shelf">Others you've WooHoo'd with</h3>
    {#each others as s (s.key)}{@render card(s)}{/each}
  {/if}
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
  .mine {
    box-shadow: 0 0 0 2px var(--accent);
  }
  .heart {
    background: none;
    border: none;
    box-shadow: none;
    padding: 4px 6px;
    font-size: 1.2rem;
  }
  .heart:hover {
    opacity: 0.7;
  }
  .add {
    align-self: center;
  }
  .empty {
    text-align: center;
  }
  .adder {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .adder > button {
    align-self: flex-start;
  }
  .empty p {
    margin: 0;
  }
  .mores {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 8px;
    margin-top: 10px;
  }
  .more {
    width: 100%;
    justify-content: space-between;
  }
  .more.on {
    box-shadow: 0 0 0 2px var(--accent);
  }
  .chev {
    font-size: 1.2rem;
    line-height: 1;
    transition: transform 0.15s;
  }
  .chev.open {
    transform: rotate(90deg);
  }
</style>
