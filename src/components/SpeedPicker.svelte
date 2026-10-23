<script lang="ts" module>
  import type { DurationMode } from "../lib/types";

  export interface Speed {
    mode: DurationMode;
    speed: number;
    customMs: number | null;
  }

  const DAY = 86_400_000;

  /** Choices by total length of the whole pregnancy. */
  export const LENGTHS: { label: string; ms: number }[] = [
    { label: "9 months", ms: 280 * DAY },
    { label: "1 month", ms: 30 * DAY },
    { label: "1 week", ms: 7 * DAY },
    { label: "1 day", ms: DAY },
  ];

  export function speedForLength(ms: number): Speed {
    // 9 months is the real thing; everything else is a custom length.
    return ms === 280 * DAY ? { mode: "realtime", speed: 1, customMs: null } : { mode: "custom", speed: 1, customMs: ms };
  }

  export const DEFAULT_SPEED: Speed = speedForLength(30 * DAY);
</script>

<script lang="ts">
  import { durationFor, humanDuration } from "../lib/timeline";

  let { value, onchange }: { value: Speed; onchange: (s: Speed) => void } = $props();

  const UNITS = { hours: 3_600_000, days: DAY, weeks: 7 * DAY, months: 30 * DAY } as const;
  type Unit = keyof typeof UNITS;

  const total = $derived(durationFor(value.mode, value.speed, value.customMs));
  // Within a minute counts as the same choice.
  const picked = $derived(LENGTHS.find((l) => Math.abs(l.ms - total) < 60_000));

  let showCustom = $state(false);
  let amount = $state(3);
  let unit = $state<Unit>("days");

  function openCustom() {
    // Start the custom box from the current length, in a sensible unit.
    const wholeWeeks = total >= 14 * DAY && Math.abs(total / (7 * DAY) - Math.round(total / (7 * DAY))) < 0.01;
    const u: Unit = total >= 60 * DAY ? "months" : wholeWeeks ? "weeks" : total >= 2 * DAY ? "days" : "hours";
    unit = u;
    amount = Math.round((total / UNITS[u]) * 10) / 10;
    showCustom = true;
  }

  function applyCustom() {
    if (!(amount > 0)) return;
    onchange(speedForLength(Math.round(amount * UNITS[unit])));
  }
</script>

<p class="q">How long should the whole pregnancy last?</p>
<div class="lengths" role="radiogroup" aria-label="Pregnancy length">
  {#each LENGTHS as l}
    <button
      class="opt"
      class:on={!showCustom && picked?.ms === l.ms}
      role="radio"
      aria-checked={!showCustom && picked?.ms === l.ms}
      onclick={() => {
        showCustom = false;
        onchange(speedForLength(l.ms));
      }}>{l.label}</button
    >
  {/each}
  <button class="opt wide" class:on={showCustom || !picked} role="radio" aria-checked={showCustom || !picked} onclick={openCustom}>
    Custom{!showCustom && !picked ? `: ${humanDuration(total)}` : ""}
  </button>
</div>

{#if showCustom}
  <div class="row custom">
    <input type="number" min="0.1" step="any" bind:value={amount} aria-label="Length" />
    <select bind:value={unit} aria-label="Unit">
      {#each Object.keys(UNITS) as u}<option value={u}>{u}</option>{/each}
    </select>
    <button class="small" onclick={applyCustom} disabled={!(amount > 0)}>Use</button>
  </div>
{/if}

<style>
  .q {
    font-weight: 700;
    margin: 0 0 10px;
  }
  .lengths {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 8px;
  }
  .opt {
    background: var(--bg);
    color: var(--text);
    border: 2px solid var(--line);
    border-radius: 16px;
    padding: 12px;
    font-size: 1rem;
  }
  .opt.wide {
    grid-column: 1 / -1;
  }
  .opt.on {
    border-color: var(--accent);
    background: var(--bg-3);
  }
  .custom {
    margin-top: 10px;
  }
  .custom input {
    width: 90px;
  }
</style>
