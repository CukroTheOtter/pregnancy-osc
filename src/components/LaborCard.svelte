<script lang="ts">
  // Labor on Home: start it, see how it's going, or skip to the birth.
  // The logic lives in pregsys-core/src/labor.rs; this only shows it.
  import { app, errorToast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { BABY_REVEAL_WEEK } from "../lib/fertility";
  import type { Labor, Pregnancy } from "../lib/types";
  import { hipWarning } from "../lib/posture";

  let { p, week, isDue, viewingOther, onbirth }: { p: Pregnancy; week: number; isDue: boolean; viewingOther: boolean; onbirth: () => void } = $props();

  /** The avatar's Start button and the app's Start labor work from this week. */
  const LABOR_WEEK = 37;

  const status = $derived(app.status);
  const settings = $derived(app.data?.settings);
  const labor = $derived(p.labor ?? null);
  // Live from the engine when it's about the pregnancy you're wearing; the saved copy can lag.
  const paused = $derived(!!labor && (viewingOther ? labor.paused : (status?.laborPaused ?? labor.paused)));
  const partner = $derived(p.partner.name);
  // In-game on an avatar that can show labor.
  const canShow = $derived(!viewingOther && !!status?.oscActive && !!status?.laborSupported);
  const many = $derived(p.babyCount > 1 && week >= BABY_REVEAL_WEEK);
  // "Resting holds off labor": whether you're up right now (in-game only).
  const resting = $derived(!!settings?.labor.restingHolds && canShow && status?.up === false);
  const welcome = $derived(many ? "Welcome your babies 🍼" : "Welcome your baby 🍼");
  let busy = $state(false);

  /** Minutes between contractions (same as Labor::contraction_gap_ms). */
  function gapMinutes(l: Labor, minutes: number): number {
    const early = minutes * 0.5 * 60_000;
    if (l.phase === "early") return 5 - 3 * Math.min(1, l.elapsedMs / Math.max(1, early));
    return l.phase === "active" ? 1 : 0.75;
  }

  // Progress through labor: early half, water broke 30%, pushing 20% (as in labor.rs).
  const SPLIT = [0.5, 0.3, 0.2];
  const total = $derived((settings?.labor.minutes ?? 20) * 60_000);
  const done = $derived(labor ? Math.min(1, ((viewingOther ? null : status?.laborElapsedMs) ?? labor.elapsedMs) / total) : 0);
  const leftMin = $derived(Math.max(1, Math.ceil(((1 - done) * total) / 60_000)));

  const title = $derived(
    !labor ? "" : labor.phase === "early" ? "🤰 Labor has started" : labor.phase === "active" ? "💦 Your water broke" : "💪 Almost there!",
  );

  const pausedWhy = $derived.by(() => {
    if (!paused) return "";
    if (p.timeline.pausedAtMs != null) return "Resume the pregnancy to carry on.";
    if (status?.afk) return "It carries on when you're back from AFK.";
    if (status?.laborAvatar && !status.laborSupported) return `Switch back to ${status.laborAvatar} to carry on.`;
    if (!status?.oscActive) return "It carries on when you're in VRChat.";
    if (!status.laborSupported) return "This avatar can't show labor.";
    return "";
  });

  async function startLabor() {
    busy = true;
    try {
      await api.startLabor(p.id);
    } catch (e) {
      errorToast(e);
    } finally {
      busy = false;
    }
  }
</script>

{#if labor}
  <div class="labor">
    <strong>{title}</strong>
    {#if paused}
      <span class="small">⏸ Labor paused. {pausedWhy}</span>
    {:else if resting && labor.phase === "early"}
      <span class="rest small">🛋️ Resting: your water won't break until you're up</span>
    {:else if settings}
      <span class="small muted">Contractions about every {Math.round(gapMinutes(labor, settings.labor.minutes) * 10) / 10} min</span>
    {/if}
    <div class="bar" role="progressbar" aria-label="Labor progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(done * 100)}>
      <div class="fill" class:paused style="width:{done * 100}%"></div>
      <span class="mark" style="left:{SPLIT[0] * 100}%"></span>
      <span class="mark" style="left:{(SPLIT[0] + SPLIT[1]) * 100}%"></span>
    </div>
    <div class="steps small">
      <span style="width:{SPLIT[0] * 100}%" class:now={labor.phase === "early"}>🤰 Early</span>
      <span style="width:{SPLIT[1] * 100}%" class:now={labor.phase === "active"}>💦 Water</span>
      <span style="width:{SPLIT[2] * 100}%" class:now={labor.phase === "pushing"}>💪 Push</span>
    </div>
    {#if !paused}<span class="small muted">About {leftMin} min to go</span>{/if}
    {#if partner && status?.partnerHere != null && !viewingOther}
      <span class="small">{status.partnerHere ? `💞 ${partner} is here` : `💞 ${partner} left`}</span>
    {/if}
  </div>
  <button class="big" onclick={onbirth}>{welcome}</button>
{:else if week >= LABOR_WEEK && canShow}
  {#if settings?.labor.restingHolds}
    <span class="rest small" class:up={!resting}>{resting ? "🛋️ Resting: labor is holding off" : "🚶 Up and about: labor can start any time"}</span>
    {#if hipWarning(status)}<span class="small warn">⚠️ {hipWarning(status)}</span>{/if}
  {/if}
  {#if isDue && settings?.labor.waitsForPartner && partner && status?.partnerHere === false}
    <span class="small muted">Ready when {partner} is here 💞</span>
  {/if}
  <button class="big" onclick={startLabor} disabled={busy}>Start labor 🤰</button>
{:else if isDue}
  {#if status?.laborAvatar && !viewingOther}
    <span class="small muted">Ready for labor 🍼 Switch to {status.laborAvatar} to start it, or:</span>
  {/if}
  <button class="big" onclick={onbirth}>{welcome}</button>
{/if}

<style>
  .labor {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }
  /* Resting status ("Resting holds off labor"), when it decides anything. */
  .rest {
    align-self: center;
    padding: 4px 12px;
    border-radius: 999px;
    background: var(--bg-3);
    font-weight: 700;
  }
  .warn {
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--warn) 18%, transparent);
    text-align: left;
  }
  .rest.up {
    background: color-mix(in srgb, var(--ok) 20%, var(--bg-3));
  }
  .bar {
    position: relative;
    height: 12px;
    margin-top: 6px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--accent) 22%, var(--bg-2));
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: 999px;
    background: var(--accent);
    transition: width 0.9s linear;
  }
  .fill.paused {
    opacity: 0.5;
  }
  .mark {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    background: var(--bg-2);
  }
  .steps {
    display: flex;
    color: var(--muted);
  }
  .steps span {
    text-align: center;
  }
  .steps .now {
    color: var(--text);
    font-weight: 800;
  }
  .big {
    width: 100%;
    padding: 13px;
    font-size: 1.05rem;
  }
</style>
