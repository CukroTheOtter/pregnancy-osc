<script lang="ts">
  import SpeciesIcon from "./SpeciesIcon.svelte";
  import SpeciesPicker from "./SpeciesPicker.svelte";
  import { app, updatePregnancy, errorToast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { CONFIDENT, guessSpecies } from "../lib/genetics";
  import { speciesName } from "../lib/species";
  import type { Pregnancy } from "../lib/types";

  let { p }: { p: Pregnancy } = $props();

  const partner = $derived(p.partner);
  const guess = $derived(
    partner.name ? guessSpecies(partner.avatarName, app.data!.speciesAnswers, app.data!.communitySpecies, partner.userId) : null,
  );
  let picking = $state(false);

  // Confident guesses apply on their own; shaky ones ask first.
  $effect(() => {
    if (partner.name && !partner.species && guess && guess.confidence >= CONFIDENT) {
      updatePregnancy(p.id, (x) => {
        x.partner.species = guess.species;
        x.partner.speciesSource = guess.source === "keyword" ? "guess" : "confirmed";
      });
    }
  });

  const askConfirm = $derived(!!partner.name && !partner.species && !!guess && guess.confidence < CONFIDENT);
  const askSpecies = $derived(!!partner.name && !partner.species && !guess);

  async function confirmSpecies(species: string) {
    picking = false;
    try {
      const key = partner.userId ?? partner.avatarName ?? partner.name;
      if (key) await api.setSpeciesAnswer(key, species);
      if (partner.avatarName && partner.userId) await api.setSpeciesAnswer(partner.avatarName, species);
      await updatePregnancy(p.id, (x) => {
        x.partner.species = species;
        x.partner.speciesSource = "confirmed";
      });
    } catch (e) {
      errorToast(e);
    }
  }

  async function confirmPartner() {
    await updatePregnancy(p.id, (x) => (x.partner.status = "confirmed"));
  }
</script>

<div class="card partner">
  <div class="row">
    <h3 class="grow">Other parent</h3>
    {#if partner.status === "guessed"}<span class="pill warn">Guessed</span>{/if}
    {#if partner.status === "confirmed"}<span class="pill ok">Confirmed</span>{/if}
    {#if partner.status === "manual"}<span class="pill">Typed in</span>{/if}
    {#if partner.status === "unknown"}<span class="pill off">Unknown</span>{/if}
  </div>

  {#if partner.name}
    <div class="row who">
      <SpeciesIcon id={partner.species ?? p.motherSpecies} size={48} />
      <div class="grow">
        <strong>{partner.name}</strong>
        <div class="muted small">
          {partner.species ? speciesName(partner.species) : "Species unknown"}
          {#if partner.avatarName}· on “{partner.avatarName}”{/if}
        </div>
      </div>
    </div>
    {#if partner.status === "guessed"}
      <p class="small muted">They were the only other person in the instance.</p>
      <div class="row">
        <button class="small" onclick={confirmPartner}>Yes, that's right</button>
        <button class="small soft" onclick={() => (app.view = "people")}>Change</button>
      </div>
    {/if}
  {:else}
    <p class="small muted">
      {partner.candidates.length ? "There were other people around, so it's a mystery for now." : "No one else was detected."}
      The babies use your species until you choose.
    </p>
    <button class="small" onclick={() => (app.view = "people")}>Who was the other parent?</button>
  {/if}

  {#if askConfirm && guess}
    <div class="ask">
      <SpeciesIcon id={guess.species} size={40} />
      <span class="grow">Looks like a <strong>{speciesName(guess.species).toLowerCase()}</strong>, correct?</span>
      <button class="small" onclick={() => confirmSpecies(guess.species)}>Yes</button>
      <button class="small soft" onclick={() => (picking = true)}>No</button>
    </div>
  {:else if askSpecies}
    <div class="ask">
      <span class="grow">What species was {partner.name}?</span>
      <button class="small soft" onclick={() => (picking = true)}>Choose</button>
    </div>
  {:else if partner.name && partner.species && partner.speciesSource !== "confirmed"}
    <button class="link small" onclick={() => (picking = true)}>Not a {speciesName(partner.species).toLowerCase()}?</button>
  {/if}
  {#if picking}
    <div style="margin-top:10px"><SpeciesPicker value={partner.species} onpick={confirmSpecies} /></div>
  {/if}
</div>

<style>
  .who { margin: 6px 0 10px; }
  .ask {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 12px;
    padding: 10px 12px;
    background: var(--bg-3);
    border-radius: 14px;
    flex-wrap: wrap;
  }
  .link { background: none; color: var(--accent); padding: 4px 0; margin-top: 8px; }
</style>
