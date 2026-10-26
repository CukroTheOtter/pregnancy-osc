<script lang="ts">
  import { app, errorToast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import type { Person, Pregnancy, Settings } from "../lib/types";
  import PersonPic from "./PersonPic.svelte";
  import PersonSearch from "./PersonSearch.svelte";
  import { ordinal, statsFor } from "../lib/scope";

  let { p }: { p: Pregnancy } = $props();

  const partner = $derived(p.partner);
  const stats = $derived(app.data ? statsFor(app.data, partner.name, partner.userId) : undefined);
  let changing = $state(false);
  // The list is open when nobody is chosen yet, or when changing.
  const showList = $derived(changing || !partner.name);

  async function choose(person: Person | null, name: string | null = null, random = false) {
    try {
      await api.setPartner(p.id, person, name, random);
      changing = false;
    } catch (e) {
      errorToast(e);
    }
  }

  // Offer pictures once there's someone with a VRChat ID to show.
  const canShowPictures = $derived(
    !app.data?.settings.showPictures && (!!partner.userId || partner.candidates.some((c) => c.userId)),
  );

  async function turnOnPictures() {
    if (!app.data) return;
    const s = { ...($state.snapshot(app.data.settings) as Settings), showPictures: true };
    try {
      await api.saveSettings(s);
      app.data.settings.showPictures = true;
    } catch (e) {
      errorToast(e);
    }
  }

  async function confirmGuess() {
    const me = partner.candidates.find((c) => c.name === partner.name) ?? {
      name: partner.name!,
      userId: partner.userId,
      avatarName: partner.avatarName,
      lastSeenMs: app.now,
    };
    await choose(me);
  }
</script>

<section class="card">
  <h2>💞 Partner</h2>

  {#if partner.name}
    <div class="chosen">
      <PersonPic name={partner.name} userId={partner.userId} size={56} />
      <div class="grow">
        <strong class="name">{partner.name}</strong>
        <div class="muted small">
          {#if partner.status === "guessed"}Guessed: you two were alone in the instance
          {:else if partner.status === "manual"}Typed in by you
          {:else if partner.status === "random"}🎲 Picked at random from {partner.candidates.length} people
          {:else}Confirmed{/if}
        </div>
        {#if stats}
          <div class="stats small">
            💞 {ordinal(stats.pregnancies)} pregnancy together{stats.kids ? ` · 👶 ${stats.kids} ${stats.kids === 1 ? "kid" : "kids"}` : ""}{stats.fun ? ` · 💕 ${stats.fun} ${stats.fun === 1 ? "WooHoo" : "WooHoos"}` : ""}
          </div>
        {/if}
      </div>
    </div>
    {#if partner.status === "guessed" && !changing}
      <div class="row">
        <button onclick={confirmGuess}>That's right</button>
        <button class="soft" onclick={() => (changing = true)}>Change</button>
      </div>
    {:else if !changing}
      <button class="soft small" onclick={() => (changing = true)}>Change</button>
    {/if}
  {:else}
    <p class="muted">Who was it?</p>
  {/if}

  {#if showList}
    <PersonSearch candidates={partner.candidates} selected={partner.name} onpick={(person, name) => choose(person, name)} />
    {#if !partner.candidates.length}
      <p class="muted small">No one else was seen in the instance{app.data?.settings.readLogs ? "" : " (VRChat log reading is off in Settings)"}.</p>
    {/if}
    <div class="row">
      {#if partner.candidates.length > 1}<button class="soft small" onclick={() => choose(null, null, true)}>🎲 Random</button>{/if}
      {#if changing}<button class="ghost small" onclick={() => (changing = false)}>Cancel</button>{/if}
      {#if partner.name}<button class="ghost small" onclick={() => choose(null, null)}>Set to unknown</button>{/if}
    </div>
  {/if}
  {#if canShowPictures}
    <button class="link small" onclick={turnOnPictures}>📷 Show their VRChat profile pictures</button>
  {/if}
</section>

<style>
  .chosen {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 4px 0 12px;
  }
  .stats {
    margin-top: 2px;
    font-weight: 700;
    color: var(--accent);
  }
  .name {
    font-size: 1.15rem;
  }
  .link {
    background: none;
    color: var(--accent);
    padding: 6px 0 0;
  }
</style>
