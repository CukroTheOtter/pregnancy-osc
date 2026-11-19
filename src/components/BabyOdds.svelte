<script lang="ts">
  // How many babies: the three presets or a litter (pregsys-core `BabyOdds`).
  // Settings uses it for everyone; an avatar's panel adds "Use my default".
  import { MAX_BABIES, MULTIPLES_LABEL, SPREAD_LABEL, litterOddsText } from "../lib/fertility";
  import type { Litter, LitterSpread, Multiples } from "../lib/types";

  let {
    multiples,
    litter,
    onchange,
    inherit = null,
  }: {
    multiples: Multiples | null;
    litter: Litter | null;
    onchange: (multiples: Multiples | null, litter: Litter | null) => void;
    /** Per avatar: what the default is right now ("Playful"); null on the Settings page. */
    inherit?: string | null;
  } = $props();

  const choice = $derived(litter ? "litter" : (multiples ?? "default"));
  // The slider moves freely; the change is saved when it's let go.
  let usual = $state(4);
  $effect(() => {
    usual = litter?.usual ?? 4;
  });
  const spread = $derived<LitterSpread>(litter?.spread ?? "some");

  function pick(c: string) {
    if (c === "default") onchange(null, null);
    else if (c === "litter") onchange(multiples, { usual: litter?.usual ?? 4, spread });
    else onchange(c as Multiples, null);
  }
</script>

{#if inherit != null}
  <label class="field">How many babies
    <select value={choice} onchange={(e) => pick((e.target as HTMLSelectElement).value)}>
      <option value="default">Use my default ({inherit})</option>
      {#each Object.entries(MULTIPLES_LABEL) as [k, m]}<option value={k}>{m.name}</option>{/each}
      <option value="litter">Litter</option>
    </select>
  </label>
{:else}
  <div class="choices four">
    {#each Object.entries(MULTIPLES_LABEL) as [k, m]}
      <button class="opt" class:on={choice === k} onclick={() => pick(k)}><strong>{m.name}</strong><span>{m.hint}</span></button>
    {/each}
    <button class="opt" class:on={choice === "litter"} onclick={() => pick("litter")}><strong>Litter</strong><span>Pick how many are usual, up to {MAX_BABIES}</span></button>
  </div>
{/if}

{#if litter}
  <div class="litter">
    <label class="field">Usually <strong>{usual}</strong>
      <input type="range" min="1" max={MAX_BABIES} step="1" bind:value={usual} onchange={() => onchange(multiples, { usual, spread })} />
    </label>
    <div class="field">Varies
      <div class="segmented">
        {#each Object.entries(SPREAD_LABEL) as [k, l]}
          <button class:on={spread === k} onclick={() => onchange(multiples, { usual, spread: k as LitterSpread })}>{l}</button>
        {/each}
      </div>
    </div>
    <span class="hint">Chances: {litterOddsText({ usual, spread })}</span>
  </div>
{/if}

<style>
  .choices.four {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 8px;
  }
  .opt {
    flex-direction: column;
    align-items: flex-start;
    text-align: left;
    white-space: normal;
    gap: 1px;
    background: var(--bg);
    color: var(--text);
    border: 2px solid var(--line);
    border-radius: 14px;
    padding: 9px 10px;
  }
  .opt span {
    font-weight: 600;
    font-size: 0.75rem;
    color: var(--muted);
  }
  .opt.on {
    border-color: var(--accent);
    background: var(--bg-3);
  }
  .litter .hint {
    font-weight: 400;
    color: var(--muted);
    font-size: 0.8rem;
  }
  .litter {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 8px;
  }
  .litter input[type="range"] {
    width: 100%;
  }
</style>
