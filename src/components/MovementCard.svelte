<script lang="ts">
  import KickChart from "./KickChart.svelte";
  import { app } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { errorToast } from "../lib/app.svelte";
  import { isEggs } from "../lib/fertility";
  import { KICK_MOODS, kickMood, moodInfo } from "../lib/kickMood";
  import { humanDuration, progress, TERM_WEEKS } from "../lib/timeline";
  import type { KickMood, Pregnancy } from "../lib/types";

  // worn: you wear this avatar now (not just viewing it on Home).
  let { p, worn = true }: { p: Pregnancy; worn?: boolean } = $props();

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
  // The chart shows the current (or last) VRChat session, or with no session
  // known yet, once there's been a kick in the last day.
  const recent = $derived(!!app.status?.vrchatSinceMs || (!!last && app.now - last.timeMs < 24 * 3_600_000));
  // The baby's personality, and whether a WooHoo just woke it up.
  const moods = $derived(!!app.data?.settings.kickMoods);
  const mood = $derived(moodInfo(kickMood(p)));
  const awakeFor = $derived(Math.max(0, (app.status?.awakeUntilMs ?? 0) - app.now));
  let picking = $state(false);

  async function pickMood(m: KickMood | null) {
    try {
      await api.setKickMood(p.id, m);
      picking = false;
    } catch (e) {
      errorToast(e);
    }
  }
</script>

<section class="card movement" class:fresh>
  <div class="row">
    <h2 class="grow">💓 Movement</h2>
    {#if enabled && week >= FIRST_KICK_WEEK && awakeFor > 0}<span class="pill awake" title="Woken up by a WooHoo">👀 Awake</span>{/if}
    {#if enabled && week >= FIRST_KICK_WEEK}<span class="pill">{today} {today === 1 ? "kick" : "kicks"} today</span>{/if}
  </div>
  {#if enabled && moods && week >= FIRST_KICK_WEEK}
    <div class="mood">
      <span class="small"><strong>{mood.emoji} {mood.name}</strong> <span class="muted">· {mood.hint}</span></span>
      <button class="link small" onclick={() => (picking = !picking)} aria-expanded={picking}>{picking ? "Done" : "Change"}</button>
    </div>
    {#if picking}
      <div class="moods">
        {#each KICK_MOODS as m (m.k)}
          <button class="soft small" class:on={mood.k === m.k} onclick={() => pickMood(m.k)}>{m.emoji} {m.name}</button>
        {/each}
        {#if p.kickMood}<button class="ghost small" onclick={() => pickMood(null)}>As it was</button>{/if}
      </div>
    {/if}
  {/if}
  {#if enabled && week >= FIRST_KICK_WEEK && awakeFor > 0}
    <p class="muted small">A WooHoo woke {isEggs(p) ? (p.babyCount > 1 ? "your eggs" : "your egg") : p.babyCount > 1 ? "your babies" : "your baby"} up: busy for {humanDuration(awakeFor)} more.</p>
  {/if}
  {#if !enabled}
    <p class="muted small">Kicks are off (Settings > Kicks).</p>
  {:else if week < FIRST_KICK_WEEK}
    <p class="muted small">Too small to feel yet. Kicks start around week {FIRST_KICK_WEEK}.</p>
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
  {#if enabled && week >= FIRST_KICK_WEEK && recent}
    <KickChart {p} {worn} />
  {/if}
</section>

<style>
  .mood {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 2px 0 8px;
  }
  .mood .link {
    background: none;
    color: var(--accent);
    padding: 0;
  }
  .moods {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 10px;
  }
  .moods .on {
    background: color-mix(in srgb, var(--accent) 35%, var(--bg-2));
  }
  .awake {
    background: color-mix(in srgb, var(--accent) 30%, var(--bg-2));
  }
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
