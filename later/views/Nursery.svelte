<script lang="ts">
  import Baby from "../components/Baby.svelte";
  import { app, currentPregnancy, updatePregnancy } from "../lib/app.svelte";
  import { babyViews } from "../lib/look";
  import { isTraitVisible } from "../lib/reveals";
  import { TRAITS, traitLabel, traitOptions, type TraitKey } from "../lib/species";
  import { blendLabel } from "../lib/genetics";
  import { progress } from "../lib/timeline";

  const p = $derived(currentPregnancy());
  const views = $derived(p && app.data ? babyViews(p, app.data) : []);
  const spoiler = $derived(app.data?.settings.spoilerMode ?? "surprise");

  /** Tap a trait to cycle it (the quick path; the Gene Lab has the full editor). */
  async function cycle(i: number, key: TraitKey, current: string) {
    if (!p) return;
    const opts = traitOptions(key);
    const next = opts[(opts.indexOf(current) + 1) % opts.length];
    await updatePregnancy(p.id, (x) => (x.babies[i].locks[key] = next));
  }

  async function rename(i: number, name: string) {
    if (!p) return;
    await updatePregnancy(p.id, (x) => (x.babies[i].name = name.trim() || null));
  }

  async function setCount(n: number) {
    if (!p) return;
    await updatePregnancy(p.id, (x) => (x.babyCount = n));
  }
</script>

<div class="view">
  {#if !p}
    <div class="card"><h2>Nursery</h2><p class="muted">No pregnancy yet.</p></div>
  {:else}
    <header class="row">
      <h1 class="grow">🧸 Nursery</h1>
      <label class="row small">
        Babies
        <select value={p.babyCount} onchange={(e) => setCount(+(e.target as HTMLSelectElement).value)}>
          {#each [1, 2, 3, 4, 5, 6, 7, 8] as n}<option value={n}>{n}</option>{/each}
        </select>
      </label>
    </header>
    <p class="muted small">Tap a trait to change it. Changed traits are kept and never rerolled. Hidden traits unlock with scans.</p>

    <div class="grid">
      {#each views as v, i}
        <div class="card baby">
          <svg viewBox="-70 -75 140 130" aria-label={v.name}>
            <Baby look={v.look} awake stage={Math.max(0.3, progress(p.timeline, app.now))} />
          </svg>
          <input class="name" value={p.babies[i].name ?? ""} placeholder={`Baby ${i + 1}`} onchange={(e) => rename(i, (e.target as HTMLInputElement).value)} aria-label="Baby name" />
          {#if spoiler === "all" || p.reveals.species || v.result.manualSpecies}
            <span class="pill">{blendLabel(v.result.blend)}</span>
          {/if}
          <div class="chips">
            {#each TRAITS as t}
              {@const vis = isTraitVisible(p, t.key, spoiler, p.babies[i].locks[t.key] != null)}
              {#if vis}
                <button class="chip" class:kept={p.babies[i].locks[t.key] != null} onclick={() => cycle(i, t.key, v.result.traits[t.key].value)} title="Tap to change {t.label.toLowerCase()}">
                  {#if t.kind === "color"}<span class="sw" style="background:{v.result.traits[t.key].value}"></span>{/if}
                  {traitLabel(t.key, v.result.traits[t.key].value)}
                </button>
              {:else}
                <span class="chip hidden">{t.label}: ?</span>
              {/if}
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(250px, 1fr)); gap: 16px; margin-top: 12px; }
  .baby { display: flex; flex-direction: column; align-items: center; gap: 8px; text-align: center; }
  .baby svg { width: 170px; height: 160px; }
  .name { text-align: center; font-weight: 800; font-size: 1.1rem; border-color: transparent; background: transparent; width: 100%; }
  .name:hover, .name:focus { border-color: var(--line); background: var(--bg); }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; justify-content: center; }
  .chip { background: var(--bg-3); color: var(--text); padding: 4px 10px; font-size: 0.8rem; font-weight: 700; border-radius: 999px; display: inline-flex; align-items: center; gap: 5px; }
  .chip.kept { box-shadow: inset 0 0 0 2px var(--accent); }
  .chip.hidden { color: var(--muted); background: transparent; border: 1.5px dashed var(--line); }
  .sw { width: 12px; height: 12px; border-radius: 50%; display: inline-block; }
</style>
