<script lang="ts">
  import PersonPic from "../components/PersonPic.svelte";
  import { app, confirmAsk } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { partnerStats } from "../lib/scope";
  import { humanDuration } from "../lib/timeline";

  const list = $derived(app.data ? partnerStats(app.data) : []);
  const totals = $derived({
    fun: list.reduce((s, x) => s + x.fun, 0),
    pregnancies: list.reduce((s, x) => s + x.pregnancies, 0),
    kids: list.reduce((s, x) => s + x.kids, 0),
  });
  const dateFmt = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", year: "numeric" });

  async function forget(name: string, userId: string | null) {
    if (await confirmAsk({ title: `Forget ${name}?`, text: "They're removed from your partners, WooHoo history and instance history. Pregnancies with them carry on with an unknown partner.", ok: "Forget", danger: true })) {
      await api.forgetPerson(name, userId);
    }
  }
</script>

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

  {#each list as s (s.key)}
    <section class="card partner">
      <div class="row">
        <PersonPic name={s.name} userId={s.userId} size={52} />
        <div class="grow">
          <strong class="name">{s.name}</strong>
          <div class="muted small">
            Last time {humanDuration(app.now - s.lastMs)} ago{s.firstMs !== s.lastMs ? ` · first ${dateFmt.format(s.firstMs)}` : ""}
          </div>
        </div>
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
  {/each}
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
  .empty {
    text-align: center;
  }
  .big {
    font-size: 2.6rem;
  }
</style>
