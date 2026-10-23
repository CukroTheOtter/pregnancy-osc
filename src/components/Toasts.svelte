<script lang="ts">
  import { app } from "../lib/app.svelte";
</script>

<div class="toasts" aria-live="polite">
  {#each app.toasts as t (t.id)}
    <div class="toast card {t.kind}">
      <strong>{t.kind === "error" ? "⚠️ " : t.kind === "success" ? "✨ " : ""}{t.title}</strong>
      {#if t.text}<div class="small">{t.text}</div>{/if}
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 16px;
    bottom: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 60;
    max-width: min(360px, calc(100vw - 32px));
  }
  .toast { padding: 12px 16px; animation: pop 0.25s ease; }
  .toast.success { border-color: var(--accent); background: linear-gradient(135deg, var(--bg-2), var(--bg-3)); }
  .toast.error { border-color: var(--danger); }
  @keyframes pop { from { transform: translateY(10px) scale(0.96); opacity: 0; } }
</style>
