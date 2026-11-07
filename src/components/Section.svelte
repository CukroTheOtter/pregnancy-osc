<script lang="ts" module>
  /** Settings' search box: sections that don't match hide, ones that do open up. */
  export interface SectionFilter {
    q: string;
    hits: Record<string, boolean>;
  }
  export const FILTER_KEY = Symbol("section-filter");
</script>

<script lang="ts">
  import { getContext, untrack, type Snippet } from "svelte";

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

  // With a search box around (Settings), the body is always there (hidden when folded) so its words can be searched.
  const filter = getContext<SectionFilter | undefined>(FILTER_KEY);
  let el = $state<HTMLElement | null>(null);
  const q = $derived(filter?.q.trim().toLowerCase() ?? "");
  let hit = $state(true);
  $effect(() => {
    const words = q;
    if (!filter || !el) return;
    hit = !words || (el.textContent ?? "").toLowerCase().includes(words);
    filter.hits[id] = hit;
    return () => delete filter.hits[id];
  });
  const searching = $derived(!!q);
  const shown = $derived(searching ? hit : isOpen);
</script>

<section class="card" bind:this={el} hidden={searching && !hit}>
  <button class="head" onclick={toggle} aria-expanded={shown} disabled={searching}>
    <h2>{title}</h2>
    {#if extra}{@render extra()}{/if}
    <span class="chev" class:open={shown} aria-hidden="true">›</span>
  </button>
  {#if filter}
    <div class="body" hidden={!shown}>{@render children()}</div>
  {:else if isOpen}
    <div class="body">{@render children()}</div>
  {/if}
</section>

<style>
  section[hidden],
  .body[hidden] {
    display: none;
  }
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
  .head:disabled {
    cursor: default;
    opacity: 1;
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
