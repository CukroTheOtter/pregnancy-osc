<script lang="ts">
  import { app, errorToast } from "../lib/app.svelte";
  import { api, inTauri } from "../lib/api";
  import { humanDuration } from "../lib/timeline";
  import PersonPic from "./PersonPic.svelte";
  import PersonSearch from "./PersonSearch.svelte";
  import type { PendingConception, Person } from "../lib/types";

  let { w }: { w: PendingConception } = $props();

  const left = $derived(Math.max(0, w.revealMs - app.now));
  const timeFmt = new Intl.DateTimeFormat(undefined, { weekday: "short", hour: "numeric", minute: "2-digit" });
  const others = $derived(w.partner.candidates.filter((c) => c.name !== w.partner.name).length);
  let changing = $state(false);

  // Who it was counts for them on Partners, even if nothing comes of it.
  async function choose(person: Person | null, name: string | null, random = false) {
    try {
      await api.setWaitingPartner(w.timeMs, person, name, random);
      changing = false;
    } catch (e) {
      errorToast(e);
    }
  }
</script>

<section class="card waiting">
  <div class="bubble" aria-hidden="true">💞</div>
  <h1>You WooHoo'd! 💕</h1>
  <div class="who">
    {#if w.partner.name}
      <PersonPic name={w.partner.name} userId={w.partner.userId} size={44} />
      <span>with <strong>{w.partner.name}</strong>{w.partner.status === "random" ? " 🎲" : ""}</span>
      {#if !changing}<button class="ghost small" onclick={() => (changing = true)}>Change</button>{/if}
    {:else}
      <span class="muted">{others ? `${others} people were around` : "Someone was there"}</span>
    {/if}
  </div>
  {#if !w.partner.name || changing}
    <div class="pick">
      <span class="muted small">Who was it?</span>
      <PersonSearch candidates={w.partner.candidates} selected={w.partner.name} onpick={(person, name) => choose(person, name)} />
      <div class="row">
        {#if w.partner.candidates.length > 1}<button class="soft small" onclick={() => choose(null, null, true)}>🎲 Random</button>{/if}
        {#if changing}<button class="ghost small" onclick={() => (changing = false)}>Cancel</button>{/if}
      </div>
    </div>
  {/if}
  <p class="muted small">{timeFmt.format(w.timeMs)}{w.worldName ? ` · ${w.worldName}` : ""}</p>
  <div class="countdown">
    <span class="muted small">Find out if a little one is on the way in</span>
    <strong>{left > 0 ? humanDuration(left) : "a moment…"}</strong>
  </div>
  <p class="muted small">Your belly stays flat until then. 🤫</p>
  {#if !inTauri}
    <button class="ghost small" onclick={() => api.skipWaiting()}>Preview: find out now</button>
  {/if}
</section>

<style>
  .waiting {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 8px;
    padding: 28px 18px;
  }
  .bubble {
    font-size: 3.2rem;
    animation: float 3s ease-in-out infinite;
  }
  @keyframes float {
    50% {
      transform: translateY(-6px) scale(1.04);
    }
  }
  .who {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .pick {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    width: 100%;
    max-width: 360px;
  }
  .countdown {
    display: flex;
    flex-direction: column;
    background: var(--bg-3);
    border-radius: 16px;
    padding: 10px 22px;
    margin: 6px 0;
  }
  .countdown strong {
    font-size: 1.5rem;
  }
</style>
