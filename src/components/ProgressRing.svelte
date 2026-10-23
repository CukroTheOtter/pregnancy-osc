<script lang="ts">
  import type { Snippet } from "svelte";

  let { value = 0, color = "var(--accent)", size = 180, stroke = 14, children }: { value?: number; color?: string; size?: number; stroke?: number; children?: Snippet } = $props();
  const r = $derived((size - stroke) / 2);
  const c = $derived(2 * Math.PI * r);
  const v = $derived(Math.max(0, Math.min(1, value)));
</script>

<div class="ring" style="width:{size}px;height:{size}px">
  <svg viewBox="0 0 {size} {size}" width={size} height={size} aria-hidden="true">
    <circle cx={size / 2} cy={size / 2} {r} fill="none" stroke="var(--bg-3)" stroke-width={stroke} />
    <circle
      cx={size / 2}
      cy={size / 2}
      {r}
      fill="none"
      stroke={color}
      stroke-width={stroke}
      stroke-linecap="round"
      stroke-dasharray={c}
      stroke-dashoffset={c * (1 - v)}
      transform="rotate(-90 {size / 2} {size / 2})"
      style="transition: stroke-dashoffset 0.8s ease, stroke 0.6s"
    />
  </svg>
  <div class="inside">{@render children?.()}</div>
</div>

<style>
  .ring { position: relative; flex-shrink: 0; }
  .inside {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
  }
</style>
