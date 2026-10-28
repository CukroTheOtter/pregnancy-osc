<script lang="ts">
  let { checked = $bindable(false), label, hint = "", disabled = false, onchange }: { checked?: boolean; label: string; hint?: string; disabled?: boolean; onchange?: (v: boolean) => void } = $props();
</script>

<label class="toggle" class:disabled>
  <span class="text">
    <span class="label">{label}</span>
    {#if hint}<span class="hint">{hint}</span>{/if}
  </span>
  <input type="checkbox" role="switch" bind:checked {disabled} onchange={() => onchange?.(checked)} />
  <span class="track" aria-hidden="true"><span class="knob"></span></span>
</label>

<style>
  /* relative: keeps the hidden checkbox inside the switch, so focusing it never scrolls the window */
  .toggle { display: flex; align-items: center; gap: 12px; cursor: pointer; padding: 6px 0; position: relative; }
  .toggle.disabled { opacity: 0.55; cursor: default; }
  .text { flex: 1; display: flex; flex-direction: column; }
  .label { font-weight: 700; }
  .hint { color: var(--muted); font-size: 0.82rem; }
  input { position: absolute; opacity: 0; width: 1px; height: 1px; right: 0; top: 50%; margin: 0; }
  .track { width: 44px; height: 26px; border-radius: 999px; background: var(--bg-3); border: 1.5px solid var(--line); position: relative; transition: background 0.15s; flex-shrink: 0; }
  .knob { position: absolute; top: 2px; left: 2px; width: 19px; height: 19px; border-radius: 50%; background: var(--bg-2); box-shadow: 0 1px 4px rgba(0,0,0,.2); transition: transform 0.15s; }
  input:checked + .track { background: var(--accent); border-color: var(--accent); }
  input:checked + .track .knob { transform: translateX(18px); }
  input:focus-visible + .track { outline: 3px solid color-mix(in srgb, var(--accent) 55%, transparent); outline-offset: 2px; }
</style>
