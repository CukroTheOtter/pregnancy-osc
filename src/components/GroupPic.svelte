<script lang="ts">
  import AvatarPic from "./AvatarPic.svelte";

  // A group's picture: up to four of its avatars in a little folder, like on a phone.
  let { ids, size = 46 }: { ids: string[]; size?: number } = $props();

  const pad = $derived(Math.max(3, Math.round(size * 0.08)));
  const cell = $derived((size - pad * 3) / 2);
</script>

<span class="folder" class:none={!ids.length} style="width:{size}px;height:{size}px;padding:{pad}px;gap:{pad}px" aria-hidden="true">
  {#each ids.slice(0, 4) as id (id)}
    <AvatarPic {id} size={cell} bare />
  {/each}
  {#if !ids.length}<span class="empty" style="font-size:{size * 0.45}px">💞</span>{/if}
</span>

<style>
  .folder {
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-auto-rows: min-content;
    align-content: start;
    border-radius: 14px;
    background: var(--bg-3);
    box-shadow: 0 0 0 2px var(--line);
    flex-shrink: 0;
    box-sizing: border-box;
  }
  .none {
    grid-template-columns: 1fr;
    place-items: center;
    align-content: center;
  }
</style>
