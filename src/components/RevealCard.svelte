<script lang="ts">
  import { app, errorToast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { dueMs, humanDuration } from "../lib/timeline";
  import type { Pregnancy, RevealResult } from "../lib/types";

  let { r, p }: { r: RevealResult; p: Pregnancy | undefined } = $props();
  const dateFmt = new Intl.DateTimeFormat(undefined, { weekday: "long", month: "long", day: "numeric" });

  async function ok() {
    try {
      await api.dismissReveal();
    } catch (e) {
      errorToast(e);
    }
  }
</script>

<section class="card reveal" class:yes={r.positive}>
  {#if r.positive}
    <div class="big" aria-hidden="true">💕</div>
    <h1>You're pregnant!</h1>
    <p class="muted">From your WooHoo {humanDuration(app.now - r.eventMs)} ago.</p>
    {#if p}
      <div class="due">
        <span class="muted small">Estimated due date</span>
        <strong>{dateFmt.format(dueMs(p.timeline, app.now))}</strong>
      </div>
    {/if}
  {:else}
    <div class="big" aria-hidden="true">🤍</div>
    <h1>Not this time</h1>
    <p class="muted">No little one from that WooHoo. There's always next time 😉</p>
  {/if}
  <button onclick={ok}>{r.positive ? "Yay! 🎉" : "OK"}</button>
</section>

<style>
  .reveal {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 8px;
    padding: 28px 18px;
    animation: pop 0.5s ease;
  }
  .reveal.yes {
    border-color: var(--accent);
    background: linear-gradient(160deg, var(--bg-2), var(--bg-3));
  }
  .big {
    font-size: 3.4rem;
  }
  .due {
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border-radius: 16px;
    padding: 10px 22px;
    margin: 6px 0 10px;
  }
  .due strong {
    font-size: 1.2rem;
  }
  @keyframes pop {
    from {
      transform: scale(0.92);
      opacity: 0;
    }
  }
</style>
