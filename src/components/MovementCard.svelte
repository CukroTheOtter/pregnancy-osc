<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { humanDuration, progress, TERM_WEEKS } from "../lib/timeline";
  import type { Pregnancy } from "../lib/types";

  let { p }: { p: Pregnancy } = $props();

  const FIRST_KICK_WEEK = 16;
  const LABEL = { weak: "a little kick", strong: "a strong kick", extreme: "a big kick!" } as const;

  const week = $derived(progress(p.timeline, app.now) * TERM_WEEKS);
  const last = $derived(p.kicks?.length ? p.kicks[p.kicks.length - 1] : null);
  const today = $derived.by(() => {
    const start = new Date(app.now);
    start.setHours(0, 0, 0, 0);
    return (p.kicks ?? []).filter((k) => k.timeMs >= start.getTime()).length;
  });
  // A short wiggle when a fresh kick arrives.
  const fresh = $derived(!!last && app.now - last.timeMs < 4000);
  const enabled = $derived(!!app.data?.settings.kicks);
</script>

<section class="card movement" class:fresh>
  <div class="row">
    <h2 class="grow">💓 Movement</h2>
    {#if enabled && week >= FIRST_KICK_WEEK}<span class="pill">{today} {today === 1 ? "kick" : "kicks"} today</span>{/if}
  </div>
  {#if !enabled}
    <p class="muted small">Kicks are off (Settings > Kicks).</p>
  {:else if week < FIRST_KICK_WEEK}
    <p class="muted small">Too small to feel yet. Kicks start around week {FIRST_KICK_WEEK}.</p>
  {:else if p.timeline.pausedAtMs}
    <p class="muted small">Resting while paused.</p>
  {:else if last}
    <div class="kick">
      <span class="heart" aria-hidden="true">💓</span>
      <div>
        <strong>{fresh ? `Ooh, ${LABEL[last.strength]}` : `Last: ${LABEL[last.strength]}`}</strong>
        <div class="muted small">{fresh ? "just now" : `${humanDuration(app.now - last.timeMs)} ago`}</div>
      </div>
    </div>
  {:else}
    <p class="muted small">Waiting for the first kick… 🥺</p>
  {/if}
</section>

<style>
  .kick {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .heart {
    font-size: 1.8rem;
  }
  .fresh .heart {
    animation: thump 0.5s ease-in-out 3;
  }
  .fresh {
    animation: wiggle 0.6s ease-in-out 2;
  }
  @keyframes thump {
    50% {
      transform: scale(1.3);
    }
  }
  @keyframes wiggle {
    25% {
      transform: translateX(-2px) rotate(-0.4deg);
    }
    75% {
      transform: translateX(2px) rotate(0.4deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .fresh,
    .fresh .heart {
      animation: none;
    }
  }
</style>
