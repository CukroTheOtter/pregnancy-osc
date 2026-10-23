<script lang="ts">
  import SpeciesIcon from "./SpeciesIcon.svelte";
  import { SPECIES } from "../lib/species";

  let { value = $bindable(), onpick, exclude = [] as string[] }: { value?: string | null; onpick?: (id: string) => void; exclude?: string[] } = $props();
</script>

<div class="picker" role="radiogroup" aria-label="Species">
  {#each SPECIES.filter((s) => !exclude.includes(s.id)) as sp}
    <button
      class="sp"
      class:on={value === sp.id}
      role="radio"
      aria-checked={value === sp.id}
      onclick={() => {
        value = sp.id;
        onpick?.(sp.id);
      }}
    >
      <SpeciesIcon id={sp.id} size={44} />
      <span>{sp.name}</span>
    </button>
  {/each}
</div>

<style>
  .picker { display: grid; grid-template-columns: repeat(auto-fill, minmax(78px, 1fr)); gap: 8px; }
  .sp {
    flex-direction: column;
    background: var(--bg);
    color: var(--text);
    border: 2px solid var(--line);
    border-radius: 16px;
    padding: 8px 4px;
    gap: 2px;
    font-size: 0.8rem;
  }
  .sp.on { border-color: var(--accent); background: var(--bg-3); }
</style>
