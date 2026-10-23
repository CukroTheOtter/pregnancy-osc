<script lang="ts">
  import type { Snippet } from "svelte";

  let { title, onclose, children, wide = false }: { title: string; onclose: () => void; children: Snippet; wide?: boolean } = $props();

  function key(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }
</script>

<svelte:window onkeydown={key} />

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="modal card" class:wide role="dialog" aria-modal="true" aria-label={title}>
    <header>
      <h2>{title}</h2>
      <button class="icon soft" onclick={onclose} aria-label="Close">✕</button>
    </header>
    <div class="body">{@render children()}</div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(40, 20, 35, 0.35);
    backdrop-filter: blur(3px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
    padding: 16px;
    animation: fade 0.15s ease;
  }
  .modal {
    width: min(520px, 100%);
    max-height: calc(100vh - 32px);
    overflow: auto;
    animation: rise 0.2s ease;
  }
  .modal.wide { width: min(860px, 100%); }
  header { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-bottom: 10px; }
  header h2 { margin: 0; }
  @keyframes fade { from { opacity: 0; } }
  @keyframes rise { from { transform: translateY(12px); opacity: 0; } }
</style>
