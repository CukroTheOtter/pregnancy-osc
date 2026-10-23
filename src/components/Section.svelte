<script lang="ts">
  import { untrack, type Snippet } from "svelte";

  // A settings card that folds away. Which ones are open is remembered on this computer.
  let { id, title, open = false, extra, children }: { id: string; title: string; open?: boolean; extra?: Snippet; children: Snippet } = $props();

  const KEY = "pregsys-open-sections";
  function remembered(): Record<string, boolean> {
    try {
      return JSON.parse(localStorage.getItem(KEY) ?? "{}");
    } catch {
      return {};
    }
  }

  let isOpen = $state(untrack(() => remembered()[id] ?? open));

  function toggle() {
    isOpen = !isOpen;
    try {
      localStorage.setItem(KEY, JSON.stringify({ ...remembered(), [id]: isOpen }));
    } catch {
      // Not remembered this time; it still opens and closes.
    }
  }
</script>

<section class="card">
  <button class="head" onclick={toggle} aria-expanded={isOpen}>
    <h2>{title}</h2>
    {#if extra}{@render extra()}{/if}
    <span class="chev" class:open={isOpen} aria-hidden="true">›</span>
  </button>
  {#if isOpen}
    <div class="body">{@render children()}</div>
  {/if}
</section>

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 0;
    margin: 0;
    background: none;
    border: none;
    box-shadow: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .head h2 {
    flex: 1;
    margin: 0;
  }
  .chev {
    font-size: 1.4rem;
    line-height: 1;
    color: var(--muted);
    transition: transform 0.15s;
  }
  .chev.open {
    transform: rotate(90deg);
  }
  .body {
    margin-top: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
</style>
