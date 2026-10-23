<script lang="ts">
  import PartnerCard from "../components/PartnerCard.svelte";
  import { app, currentPregnancy, errorToast, toast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { guessSpecies } from "../lib/genetics";
  import { speciesName } from "../lib/species";
  import { humanDuration } from "../lib/timeline";
  import type { Person } from "../lib/types";

  const p = $derived(currentPregnancy());
  let typed = $state("");

  // People from around conception first, then everyone else from recent instances.
  const list = $derived.by(() => {
    const out: Person[] = [...(p?.partner.candidates ?? [])];
    for (const h of app.data?.instance.history ?? []) {
      if (!out.some((o) => (o.userId && h.userId ? o.userId === h.userId : o.name === h.name))) out.push(h);
    }
    return out.sort((a, b) => b.lastSeenMs - a.lastSeenMs);
  });

  async function choose(person: Person | null, name: string | null = null) {
    if (!p) return;
    try {
      await api.setPartner(p.id, person, name);
      toast("Other parent updated", "The babies were recalculated. Anything you set by hand stays the same.", "success");
      typed = "";
    } catch (e) {
      errorToast(e);
    }
  }

  async function forget(person: Person) {
    if (!confirm(`Forget ${person.name}? They'll be removed from your history and any pregnancy that names them.`)) return;
    await api.forgetPerson(person.name, person.userId);
  }

  const here = $derived(app.status?.others ?? []);
</script>

<div class="view">
  <h1>💞 Who was the other parent?</h1>

  {#if p}
    <div class="layout">
      <PartnerCard {p} />

      <div class="card">
        <h3>Pick from people you were with</h3>
        <p class="muted small">Most recent first. This list only comes from VRChat logs on this computer and is never uploaded.</p>
        {#if list.length === 0}
          <p class="muted">No one yet. {app.data?.settings.readLogs ? "People show up here after you share an instance." : "Log reading is off (Settings > Logs)."}</p>
        {/if}
        <ul class="people">
          {#each list as person}
            {@const g = guessSpecies(person.avatarName, app.data?.speciesAnswers, app.data?.communitySpecies, person.userId)}
            <li class:chosen={p.partner.name === person.name}>
              <div class="grow">
                <strong>{person.name}</strong>
                <div class="muted small">
                  {person.worldName ? `${person.worldName} · ` : ""}{humanDuration(app.now - person.lastSeenMs)} ago
                  {#if person.avatarName}· “{person.avatarName}”{/if}
                  {#if g}· looks like a {speciesName(g.species).toLowerCase()}{/if}
                </div>
              </div>
              <button class="small" onclick={() => choose(person)} disabled={p.partner.name === person.name}>{p.partner.name === person.name ? "Chosen" : "It was them"}</button>
              <button class="small ghost" onclick={() => forget(person)} title="Forget this person">Forget</button>
            </li>
          {/each}
        </ul>
        <div class="row" style="margin-top:12px">
          <input class="grow" bind:value={typed} placeholder="Someone else (type a name)" maxlength="60" aria-label="Someone else" />
          <button class="soft" onclick={() => choose(null, typed)} disabled={!typed.trim()}>Use this name</button>
        </div>
        <button class="ghost small" style="margin-top:10px" onclick={() => choose(null, null)}>Set back to unknown</button>
      </div>
    </div>
  {:else}
    <div class="card"><p class="muted">No pregnancy yet.</p></div>
  {/if}

  <div class="card" style="margin-top:18px">
    <h3>In your instance now</h3>
    {#if app.status?.worldName}<p class="muted small">{app.status.worldName}</p>{/if}
    {#if here.length}
      <div class="row">{#each here as h}<span class="pill">{h.name}</span>{/each}</div>
    {:else}
      <p class="muted small">No one else detected.</p>
    {/if}
  </div>
</div>

<style>
  .layout { display: grid; grid-template-columns: minmax(260px, 340px) 1fr; gap: 18px; align-items: start; margin-top: 10px; }
  @media (max-width: 900px) { .layout { grid-template-columns: 1fr; } }
  .people { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 6px; }
  .people li { display: flex; align-items: center; gap: 8px; padding: 8px 10px; border-radius: 12px; background: var(--bg); flex-wrap: wrap; }
  .people li.chosen { box-shadow: inset 0 0 0 2px var(--accent); }
</style>
