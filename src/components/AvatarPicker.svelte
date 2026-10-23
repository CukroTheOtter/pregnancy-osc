<script lang="ts">
  import AvatarPic from "./AvatarPic.svelte";
  import { app } from "../lib/app.svelte";
  import { matches, seenText } from "../lib/avatars";
  import type { AvatarProfile } from "../lib/types";

  // Search-and-pick list of avatars (adding to a group, making a new one).
  let { choices, onpick, placeholder = "Search your avatars…", focus = false }: { choices: AvatarProfile[]; onpick: (id: string) => void; placeholder?: string; focus?: boolean } = $props();

  let query = $state("");
  const d = $derived(app.data!);
  const shown = $derived(choices.filter((p) => matches(d, p, query)).slice(0, query.trim() ? 8 : 5));
</script>

<div class="picker">
  <!-- svelte-ignore a11y_autofocus -->
  <input bind:value={query} {placeholder} autofocus={focus} aria-label="Search avatars" />
  {#each shown as p (p.id)}
    {@const g = p.groupId ? d.groups[p.groupId] : null}
    <button class="choice" onclick={() => { query = ""; onpick(p.id); }}>
      <AvatarPic id={p.id} size={32} />
      <span class="grow">
        <strong>{p.name ?? "Unnamed avatar"}</strong>
        <span class="muted small">{g ? `in ${g.name} · ` : ""}{seenText(p.lastSeenMs, app.now)}</span>
      </span>
      <span class="plus" aria-hidden="true">+</span>
    </button>
  {:else}
    <p class="muted small">{choices.length ? "No avatar matches that." : "No other avatars to add."}</p>
  {/each}
  {#if !query.trim() && choices.length > shown.length}
    <span class="muted small">Showing the {shown.length} worn most recently. Search to find the other {choices.length - shown.length}.</span>
  {/if}
</div>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: 4px;
    background: var(--bg-3);
    border-radius: 14px;
    padding: 8px;
  }
  .picker input {
    margin-bottom: 4px;
  }
  .choice {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    text-align: left;
    white-space: normal;
    background: var(--bg);
    color: var(--text);
    border: none;
    box-shadow: none;
    border-radius: 12px;
    padding: 6px 10px;
  }
  .choice .grow {
    display: flex;
    flex-direction: column;
    font-weight: 600;
  }
  .plus {
    font-weight: 900;
    font-size: 1.2rem;
    color: var(--accent);
  }
</style>
