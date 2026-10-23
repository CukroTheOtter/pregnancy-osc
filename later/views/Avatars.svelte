<script lang="ts">
  import { onMount } from "svelte";
  import SpeciesIcon from "../components/SpeciesIcon.svelte";
  import { app, activePregnancies, errorToast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { SPECIES, speciesName } from "../lib/species";
  import { humanDuration } from "../lib/timeline";
  import type { AvatarProfile, Mapping, Settings } from "../lib/types";

  let open = $state<string | null>(null);
  let mapping = $state<Mapping | null>(null);
  onMount(async () => {
    try {
      mapping = (await api.getMapping()).mapping;
    } catch {
      /* the mapping check is optional */
    }
  });

  const profiles = $derived(Object.values(app.data?.avatars ?? {}).sort((a, b) => b.lastSeenMs - a.lastSeenMs));
  const perAvatar = $derived(app.data?.settings.syncScope === "perAvatar");

  async function save(p: AvatarProfile, change: (x: AvatarProfile) => void) {
    const copy = structuredClone($state.snapshot(p)) as AvatarProfile;
    change(copy);
    try {
      await api.saveAvatar(copy);
    } catch (e) {
      errorToast(e);
    }
  }

  async function remove(p: AvatarProfile) {
    if (confirm(`Forget the profile for ${p.name ?? p.id}?`)) await api.deleteAvatar(p.id);
  }

  async function setScope(perAvatar: boolean) {
    const s: Settings = { ...($state.snapshot(app.data!.settings) as Settings), syncScope: perAvatar ? "perAvatar" : "global" };
    await api.saveSettings(s);
  }

  function has(p: AvatarProfile, key: string): boolean | null {
    if (!mapping || p.params.length === 0) return null;
    const d = mapping.parameters[key];
    if (!d?.name) return null;
    return [d.name, ...(d.aliases ?? [])].some((n) => p.params.includes(n));
  }

  const CHECK = ["stage", "hideBelly", "overdue", "appActive", "appHeartbeat", "babyCount", "eggMode", "species", "revealPulse", "kick"];
</script>

<div class="view">
  <h1>🎭 Avatars</h1>
  <div class="card">
    <h3>One pregnancy for every avatar?</h3>
    <div class="segmented">
      <button class:on={!perAvatar} onclick={() => setScope(false)}>Same on all avatars</button>
      <button class:on={perAvatar} onclick={() => setScope(true)}>Each avatar has its own</button>
    </div>
    <p class="muted small" style="margin-top:8px">
      Either way, each avatar keeps its own settings below (species, belly size, hide belly). They're re-sent every time you switch avatars.
    </p>
  </div>

  {#if profiles.length === 0}
    <div class="card" style="margin-top:14px">
      <p class="muted">Avatars show up here when you switch to them in VRChat (with OSC on).</p>
    </div>
  {/if}

  <div class="list">
    {#each profiles as p (p.id)}
      {@const current = app.data?.currentAvatarId === p.id}
      <div class="card" class:current>
        <div class="row">
          <SpeciesIcon id={p.species ?? app.data?.settings.userSpecies ?? "human"} size={46} />
          <div class="grow">
            <strong>{p.name ?? "Unnamed avatar"}</strong>
            {#if current}<span class="pill ok">Wearing now</span>{/if}
            <div class="muted small">{p.species ? speciesName(p.species) : `Uses your default (${speciesName(app.data?.settings.userSpecies)})`} · seen {humanDuration(app.now - p.lastSeenMs)} ago</div>
          </div>
          <button class="small soft" onclick={() => (open = open === p.id ? null : p.id)}>{open === p.id ? "Close" : "Edit"}</button>
        </div>

        {#if open === p.id}
          <div class="edit grid2">
            <label class="field">Name<input value={p.name ?? ""} onchange={(e) => save(p, (x) => (x.name = (e.target as HTMLInputElement).value || null))} /></label>
            <label class="field">Species on this avatar
              <select value={p.species ?? ""} onchange={(e) => save(p, (x) => (x.species = (e.target as HTMLSelectElement).value || null))}>
                <option value="">Use my default</option>
                {#each SPECIES as s}<option value={s.id}>{s.name}</option>{/each}
              </select>
            </label>
            <label class="field">Type
              <select value={p.kind ?? ""} onchange={(e) => save(p, (x) => (x.kind = ((e.target as HTMLSelectElement).value || null) as AvatarProfile["kind"]))}>
                <option value="">Default</option><option value="human">Human</option><option value="furry">Furry</option><option value="egg">Egg</option>
              </select>
            </label>
            <label class="field">Belly size {Math.round((p.bellyScale ?? app.data!.settings.bellyScale) * 100)}%
              <input type="range" min="0.3" max="1.5" step="0.05" value={p.bellyScale ?? app.data!.settings.bellyScale} onchange={(e) => save(p, (x) => (x.bellyScale = +(e.target as HTMLInputElement).value))} />
              <span class="hint">{p.bellyScale == null ? "Using the default" : "Custom for this avatar"}</span>
            </label>
            <label class="field">Hide belly
              <select value={p.hideBelly == null ? "" : String(p.hideBelly)} onchange={(e) => { const v = (e.target as HTMLSelectElement).value; save(p, (x) => (x.hideBelly = v === "" ? null : v === "true")); }}>
                <option value="">Default ({app.data!.settings.hideBelly ? "hidden" : "shown"})</option><option value="false">Show</option><option value="true">Hide</option>
              </select>
            </label>
            <label class="field">Womb sounds
              <select value={p.fetalSounds == null ? "" : String(p.fetalSounds)} onchange={(e) => { const v = (e.target as HTMLSelectElement).value; save(p, (x) => (x.fetalSounds = v === "" ? null : v === "true")); }}>
                <option value="">Default (leave alone)</option><option value="true">On</option><option value="false">Off</option>
              </select>
            </label>
            {#if perAvatar}
              <label class="field">Pregnancy on this avatar
                <select value={p.followGlobal ? "__global" : (p.pregnancyId ?? "")} onchange={(e) => { const v = (e.target as HTMLSelectElement).value; save(p, (x) => { x.followGlobal = v === "__global"; x.pregnancyId = v && v !== "__global" ? v : null; }); }}>
                  <option value="">None</option>
                  <option value="__global">Follow the main pregnancy</option>
                  {#each activePregnancies() as preg}<option value={preg.id}>{preg.name}</option>{/each}
                </select>
              </label>
            {/if}
          </div>

          {#if p.params.length}
            <div class="params">
              <strong class="small">Parameters this avatar has</strong>
              <div class="row">
                {#each CHECK as key}
                  {@const h = has(p, key)}
                  {#if h != null}
                    <span class="pill" class:ok={h} class:off={!h} title={mapping?.parameters[key]?.name}>{h ? "✓" : "–"} {key}</span>
                  {/if}
                {/each}
              </div>
              {#if has(p, "stage") === false}
                <p class="small" style="color:var(--danger)">This avatar has no belly parameter that matches the mapping. Check Settings > OSC parameters.</p>
              {/if}
            </div>
          {/if}
          <div class="row" style="justify-content:flex-end;margin-top:10px">
            <span class="muted small grow">{p.id}</span>
            <button class="small ghost" onclick={() => remove(p)}>Forget profile</button>
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .list { display: flex; flex-direction: column; gap: 12px; margin-top: 14px; }
  .current { border-color: var(--accent); }
  .edit { margin-top: 14px; padding-top: 14px; border-top: 1.5px dashed var(--line); }
  .params { margin-top: 12px; display: flex; flex-direction: column; gap: 6px; }
</style>
