<script lang="ts">
  import Baby from "../components/Baby.svelte";
  import SpeciesIcon from "../components/SpeciesIcon.svelte";
  import { app, currentPregnancy, updatePregnancy } from "../lib/app.svelte";
  import { computeBaby, contextFrom, parents, blendLabel, type TraitResult } from "../lib/genetics";
  import { lookFrom } from "../lib/look";
  import { isTraitVisible, revealFor } from "../lib/reveals";
  import { SPECIES, species, speciesName, TRAITS, traitLabel, traitOptions, type TraitKey } from "../lib/species";

  const p = $derived(currentPregnancy());
  let babyIdx = $state(0);
  let peek = $state(false);
  let rolling = $state(false);
  let shuffle = $state<Record<string, string>>({});
  let editing = $state<TraitKey | null>(null);
  let blendA = $state("human");
  let blendB = $state("fox");
  let blendW = $state(60);

  const ctx = $derived(app.data ? contextFrom(app.data) : null);
  const baby = $derived(p?.babies[Math.min(babyIdx, (p?.babies.length ?? 1) - 1)]);
  const result = $derived(p && baby && ctx ? computeBaby(p, baby, ctx) : null);
  const par = $derived(p && ctx ? parents(p, ctx) : null);
  const spoiler = $derived(app.data?.settings.spoilerMode ?? "surprise");
  const visible = (k: TraitKey) => !!p && !!baby && (peek || isTraitVisible(p, k, spoiler, baby.locks[k] != null));
  const speciesVisible = $derived(!!p && (peek || spoiler === "all" || p.reveals.species != null || !!baby?.manualSpecies));

  function parentValue(role: 0 | 1, key: TraitKey): string {
    if (!par) return "";
    const pr = par[role];
    if (key === "furColor") return pr.furColor;
    if (key === "eyeColor") return pr.eyeColor;
    if (key === "sex") return "";
    return (species(pr.species) as unknown as Record<string, string>)[key];
  }

  function shown(t: TraitResult): string {
    return rolling && shuffle[t.key] ? shuffle[t.key] : t.value;
  }

  function hintText(t: TraitResult): string {
    if (t.from === "locked") return baby?.mode === "manual" ? "Planned" : "Locked";
    if (t.from === "chance") return "Chance";
    const who = t.from === "A" ? par?.[0].label : t.from === "B" ? par?.[1].label : null;
    if (t.from === "blend") return "Blend of both";
    const h = t.hint === "dominant" ? " · dominant" : t.hint === "recessive" ? " · recessive" : "";
    return `From ${who}${h}`;
  }

  async function toggleLock(t: TraitResult) {
    if (!p || !baby) return;
    const i = baby.index;
    await updatePregnancy(p.id, (x) => {
      const b = x.babies[i];
      if (b.locks[t.key] != null) delete b.locks[t.key];
      else b.locks[t.key] = t.value;
    });
  }

  async function setTrait(key: TraitKey, value: string) {
    if (!p || !baby) return;
    const i = baby.index;
    editing = null;
    await updatePregnancy(p.id, (x) => (x.babies[i].locks[key] = value));
  }

  async function roll() {
    if (!p || !baby || rolling) return;
    rolling = true;
    const keys = TRAITS.map((t) => t.key).filter((k) => baby!.locks[k] == null);
    const timer = setInterval(() => {
      const s: Record<string, string> = {};
      for (const k of keys) {
        const opts = traitOptions(k);
        s[k] = opts[Math.floor(Math.random() * opts.length)];
      }
      shuffle = s;
    }, 80);
    const i = baby.index;
    await new Promise((r) => setTimeout(r, 900));
    clearInterval(timer);
    await updatePregnancy(p.id, (x) => x.babies[i].reroll++);
    rolling = false;
    shuffle = {};
  }

  async function setMode(mode: "rolled" | "manual") {
    if (!p || !baby) return;
    const i = baby.index;
    await updatePregnancy(p.id, (x) => {
      const b = x.babies[i];
      b.mode = mode;
      if (mode === "manual" && result) {
        // Planning by hand: freeze everything as it is now.
        for (const t of Object.values(result.traits)) b.locks[t.key] = t.value;
      }
      if (mode === "rolled") b.locks = {};
    });
  }

  async function applyBlend(pure: string | null) {
    if (!p || !baby) return;
    const i = baby.index;
    await updatePregnancy(p.id, (x) => {
      x.babies[i].manualSpecies = pure
        ? { parts: { [pure]: 1 } }
        : blendA === blendB
          ? { parts: { [blendA]: 1 } }
          : { parts: { [blendA]: blendW / 100, [blendB]: (100 - blendW) / 100 } };
    });
  }

  async function clearBlend() {
    if (!p || !baby) return;
    const i = baby.index;
    await updatePregnancy(p.id, (x) => (x.babies[i].manualSpecies = null));
  }

  $effect(() => {
    if (par) {
      blendA = par[0].species;
      blendB = par[1].species === par[0].species ? (par[0].species === "fox" ? "cat" : "fox") : par[1].species;
    }
  });

  const isColor = (k: TraitKey) => k === "furColor" || k === "eyeColor";
</script>

<div class="view">
  {#if !p || !result || !par || !baby}
    <div class="card"><h2>Gene Lab</h2><p class="muted">Start a pregnancy to plan your little one's traits.</p></div>
  {:else}
    <header class="row">
      <h1 class="grow">🧬 Gene Lab</h1>
      {#if p.babies.length > 1}
        <div class="segmented" role="tablist">
          {#each p.babies as b, i}
            <button class:on={babyIdx === i} role="tab" aria-selected={babyIdx === i} onclick={() => (babyIdx = i)}>{b.name ?? `Baby ${i + 1}`}</button>
          {/each}
        </div>
      {/if}
    </header>

    <div class="layout">
      <section class="card preview">
        <svg viewBox="-70 -75 140 130" class="baby" aria-label="Baby preview">
          <Baby look={lookFrom(result, visible)} awake stage={1} />
        </svg>
        <div class="family">
          <div class="parent">
            <SpeciesIcon id={par[0].species} size={46} />
            <strong>{par[0].label}</strong>
            <span class="muted small">{speciesName(par[0].species)}</span>
          </div>
          <span class="plus">+</span>
          <div class="parent">
            <SpeciesIcon id={par[1].species} size={46} />
            <strong>{par[1].label}</strong>
            <span class="muted small">{speciesName(par[1].species)}{par[1].source === "fallback" ? " (assumed)" : ""}</span>
          </div>
        </div>
        <div class="result">
          {#if speciesVisible}
            <strong>{blendLabel(result.blend)}</strong>
            {#if result.hybrid && !result.manualSpecies}<span class="pill rare">✨ Rare hybrid!</span>{/if}
            {#if result.manualSpecies}<span class="pill">Chosen by you</span>{/if}
          {:else}
            <strong>Species: a surprise</strong>
            <span class="muted small">Revealed by a late scan</span>
          {/if}
        </div>

        <div class="segmented" role="radiogroup" aria-label="Trait mode">
          <button class:on={baby.mode === "rolled"} onclick={() => setMode("rolled")}>🎲 Rolled</button>
          <button class:on={baby.mode === "manual"} onclick={() => setMode("manual")}>✏️ Planned</button>
        </div>
        <div class="row">
          <button onclick={roll} disabled={rolling}>{rolling ? "Rolling…" : "🎲 Reroll unlocked"}</button>
          {#if spoiler !== "all"}
            <button class="soft" onclick={() => (peek = !peek)}>{peek ? "🙈 Hide" : "👀 Peek (spoilers)"}</button>
          {/if}
        </div>

        <details class="override">
          <summary>Choose species yourself</summary>
          <div class="stack" style="margin-top:10px">
            <div class="row">
              <select bind:value={blendA} aria-label="First species">{#each SPECIES as s}<option value={s.id}>{s.name}</option>{/each}</select>
              <select bind:value={blendB} aria-label="Second species">{#each SPECIES as s}<option value={s.id}>{s.name}</option>{/each}</select>
            </div>
            <label class="field">Blend: {blendW}% {speciesName(blendA)} / {100 - blendW}% {speciesName(blendB)}
              <input type="range" min="0" max="100" step="5" bind:value={blendW} />
            </label>
            <div class="row">
              <button class="small" onclick={() => applyBlend(null)}>Use this blend</button>
              <button class="small soft" onclick={() => applyBlend(blendA)}>Pure {speciesName(blendA)}</button>
              {#if baby.manualSpecies}<button class="small ghost" onclick={clearBlend}>Back to the roll</button>{/if}
            </div>
            <span class="muted small">Your choice is marked and never changed automatically.</span>
          </div>
        </details>
      </section>

      <section class="tiles">
        <div class="tile head" aria-hidden="true">
          <span>Trait</span><span>{par[0].label}</span><span>{par[1].label}</span><span>Baby</span>
        </div>
        {#each TRAITS as info}
          {@const t = result.traits[info.key]}
          {@const vis = visible(info.key)}
          <div class="tile" class:rolling={rolling && baby.locks[info.key] == null}>
            <span class="tname">{info.label}</span>
            <span class="pv">
              {#if info.key === "sex"}<span class="muted">-</span>
              {:else if isColor(info.key)}<span class="swatch" style="background:{parentValue(0, info.key)}"></span>
              {:else}{traitLabel(info.key, parentValue(0, info.key))}{/if}
            </span>
            <span class="pv">
              {#if info.key === "sex"}<span class="muted">-</span>
              {:else if isColor(info.key)}<span class="swatch" style="background:{parentValue(1, info.key)}"></span>
              {:else}{traitLabel(info.key, parentValue(1, info.key))}{/if}
            </span>
            <span class="bv">
              {#if vis}
                <button class="val" onclick={() => (editing = editing === info.key ? null : info.key)} title="Change">
                  {#if isColor(info.key)}<span class="swatch" style="background:{shown(t)}"></span>{/if}
                  {traitLabel(info.key, shown(t))}
                </button>
                <span class="hint muted">{hintText(t)}</span>
              {:else}
                <span class="mystery">?</span>
                <span class="hint muted">{spoiler === "hints" ? `${revealFor(info.key)?.hint ?? "Later"} · ${t.hint === "dominant" ? "leans " + (t.from === "A" ? par[0].label : par[1].label) : "could go either way"}` : `Scan at week ${revealFor(info.key)?.week}`}</span>
              {/if}
            </span>
            <button class="icon soft lock" class:on={baby.locks[info.key] != null} onclick={() => toggleLock(t)} aria-label={baby.locks[info.key] != null ? `Unlock ${info.label}` : `Lock ${info.label}`} disabled={!vis}>
              {baby.locks[info.key] != null ? "🔒" : "🔓"}
            </button>
            {#if editing === info.key}
              <div class="options">
                {#each traitOptions(info.key) as opt}
                  <button class="small" class:soft={opt !== t.value} onclick={() => setTrait(info.key, opt)}>
                    {#if isColor(info.key)}<span class="swatch" style="background:{opt}"></span>{/if}
                    {traitLabel(info.key, opt)}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      </section>
    </div>
  {/if}
</div>

<style>
  .layout { display: grid; grid-template-columns: minmax(260px, 340px) 1fr; gap: 18px; margin-top: 10px; }
  @media (max-width: 900px) { .layout { grid-template-columns: 1fr; } }
  .preview { display: flex; flex-direction: column; align-items: center; gap: 12px; text-align: center; align-self: start; }
  .baby { width: 200px; height: 190px; }
  .family { display: flex; align-items: center; gap: 12px; }
  .parent { display: flex; flex-direction: column; align-items: center; max-width: 110px; }
  .plus { font-size: 1.6rem; font-weight: 900; color: var(--accent); }
  .result { display: flex; flex-direction: column; align-items: center; gap: 4px; }
  .rare { background: linear-gradient(90deg, #ffe08a, #ffb3d1); color: #6b3a1a; }
  .override { width: 100%; text-align: left; }
  .override summary { cursor: pointer; font-weight: 700; }
  .tiles { display: flex; flex-direction: column; gap: 8px; }
  .tile {
    display: grid;
    grid-template-columns: 90px 1fr 1fr 1.4fr auto;
    gap: 10px;
    align-items: center;
    background: var(--bg-2);
    border: 1.5px solid var(--line);
    border-radius: 14px;
    padding: 10px 12px;
  }
  .tile.head { background: none; border: none; padding: 0 12px; font-size: 0.8rem; color: var(--muted); font-weight: 700; grid-template-columns: 90px 1fr 1fr 1.4fr 36px; }
  .tile.rolling { animation: jiggle 0.16s infinite alternate; }
  @keyframes jiggle { to { transform: translateY(-2px); } }
  .tname { font-weight: 800; }
  .pv { font-size: 0.88rem; color: var(--muted); display: flex; align-items: center; gap: 6px; }
  .bv { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; }
  .val { background: var(--bg-3); color: var(--text); padding: 5px 10px; font-size: 0.9rem; }
  .hint { font-size: 0.75rem; }
  .mystery { font-size: 1.3rem; font-weight: 900; color: var(--accent); }
  .swatch { width: 18px; height: 18px; border-radius: 50%; border: 2px solid var(--line); display: inline-block; flex-shrink: 0; }
  .lock.on { background: var(--accent); }
  .options { grid-column: 1 / -1; display: flex; flex-wrap: wrap; gap: 6px; padding-top: 6px; }
  @media (max-width: 560px) {
    .tile { grid-template-columns: 1fr auto; }
    .tile.head, .pv { display: none; }
  }
</style>
