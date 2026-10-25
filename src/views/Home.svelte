<script lang="ts">
  import ProgressRing from "../components/ProgressRing.svelte";
  import SpeedPicker, { DEFAULT_SPEED, type Speed } from "../components/SpeedPicker.svelte";
  import PartnerCard from "../components/PartnerCard.svelte";
  import CycleCard from "../components/CycleCard.svelte";
  import WaitingCard from "../components/WaitingCard.svelte";
  import RevealCard from "../components/RevealCard.svelte";
  import AllPregnancies from "../components/AllPregnancies.svelte";
  import MovementCard from "../components/MovementCard.svelte";
  import { app, currentPregnancy, currentPending, homeAvatarId, updatePregnancy, errorToast, toast, confirmAsk } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { avatarName, linkedGroup } from "../lib/scope";
  import { hasSystem } from "../lib/avatars";
  import { BABY_REVEAL_WEEK, GENDER_REVEAL_WEEK, babyWord, genderEmoji, genderText } from "../lib/fertility";
  import { dueMs, humanDuration, pause, progress, resume, retime, setProgress, TERM_WEEKS } from "../lib/timeline";
  import type { Settings } from "../lib/types";

  const p = $derived(currentPregnancy());
  const w = $derived(currentPending());
  const aid = $derived(homeAvatarId());
  const pr = $derived(p ? progress(p.timeline, app.now) : 0);
  const week = $derived(pr * TERM_WEEKS);
  const due = $derived(p ? dueMs(p.timeline, app.now) : 0);
  const paused = $derived(!!p?.timeline.pausedAtMs);
  const genders = $derived(app.data?.settings.genderMode === "random" && !!p?.genders.length);
  // The last minute counts as due, so week 40 never sits at 99.99%.
  const isDue = $derived(!!p && !paused && (pr >= 1 || due - app.now < 60_000));
  const status = $derived(app.status);
  const reveal = $derived(app.data?.lastReveal && !app.data.lastReveal.seen ? app.data.lastReveal : null);
  const viewingOther = $derived(!!app.viewAvatarId && app.viewAvatarId !== app.data?.currentAvatarId);
  // Hide Belly is on in-game (the app re-applies it on every avatar, so it's easy to forget).
  const bellyHidden = $derived(!!status?.hideBelly && (app.data?.syncValues?.[status.hideBelly] ?? 0) > 0.5);
  // The avatar shown (worn or picked) has no Pregnancy System parameters (checked; the worn one not loading).
  const noSystem = $derived.by(() => {
    const a = aid ? app.data?.avatars[aid] : undefined;
    return !!a && !(status?.avatarLoading && !viewingOther) && hasSystem(a) === false;
  });
  const shared = $derived.by(() => {
    const d = app.data;
    if (!d || d.settings.syncScope !== "perAvatar" || !aid) return [];
    return linkedGroup(d, aid).filter((id) => id !== aid).map((id) => avatarName(d, id));
  });

  // Start from the length picked last time.
  const s0 = app.data?.settings;
  let startSpeed = $state<Speed>(s0 ? { mode: s0.defaultMode, speed: s0.defaultSpeed, customMs: s0.defaultCustomMs } : { ...DEFAULT_SPEED });
  let busy = $state(false);
  let adjusting = $state(false);
  let weekInput = $state(0);

  const dateFmt = new Intl.DateTimeFormat(undefined, { weekday: "short", month: "short", day: "numeric" });

  /** Remembers the length so in-game conception (and the cycle) use it too. */
  function rememberLength(sp: Speed) {
    if (!app.data) return;
    const st = { ...($state.snapshot(app.data.settings) as Settings), defaultMode: sp.mode, defaultSpeed: sp.speed, defaultCustomMs: sp.customMs };
    app.data.settings = st;
    api.saveSettings(st).catch(errorToast);
  }

  async function start() {
    busy = true;
    try {
      await api.createPregnancy({ mode: startSpeed.mode, speed: startSpeed.speed, customMs: startSpeed.customMs, source: "manual", avatarId: aid });
      toast("A new little bean! 💕", undefined, "success");
    } catch (e) {
      errorToast(e);
    } finally {
      busy = false;
    }
  }

  function changeSpeed(s: Speed) {
    if (!p) return;
    rememberLength(s);
    // Progress stays put; only the time left changes.
    updatePregnancy(p.id, (x) => (x.timeline = retime(x.timeline, Date.now(), s.mode, s.speed, s.customMs)));
  }

  function togglePause() {
    if (!p) return;
    updatePregnancy(p.id, (x) => (x.timeline = x.timeline.pausedAtMs != null ? resume(x.timeline, Date.now()) : pause(x.timeline, Date.now())));
  }

  function openAdjust() {
    weekInput = Math.min(40, Math.round(week));
    adjusting = !adjusting;
  }

  function applyWeek() {
    if (!p) return;
    updatePregnancy(p.id, (x) => (x.timeline = setProgress(x.timeline, Date.now(), weekInput / TERM_WEEKS)));
    adjusting = false;
  }

  async function birth() {
    if (!p) return;
    const many = p.babyCount > 1;
    if (await confirmAsk({ title: many ? "Welcome your babies? 🍼" : "Welcome your baby? 🍼", text: "This ends the pregnancy with a birth and counts toward your partner stats.", ok: many ? "Welcome them!" : "Welcome baby!" })) {
      await api.giveBirth(p.id);
      toast(many ? `${babyWord(p.babyCount)} Congratulations! 🍼` : "Congratulations! 🍼", undefined, "success", 8000);
    }
  }

  async function endEarly() {
    if (!p) return;
    if (await confirmAsk({ title: "End this pregnancy?", text: "The belly goes back to flat. This doesn't count as a birth.", ok: "End pregnancy", danger: true })) {
      await api.endPregnancy(p.id);
    }
  }

  const currentSpeed = $derived<Speed>(
    p ? { mode: p.timeline.mode, speed: p.timeline.speed, customMs: p.timeline.mode === "custom" ? p.timeline.durationMs : null } : startSpeed,
  );
</script>

<div class="page">
  {#if viewingOther}
    <div class="viewing">
      <span>Viewing <strong>{avatarName(app.data!, aid)}</strong></span>
      <button class="small soft" onclick={() => (app.viewAvatarId = null)}>Back to the one you're wearing</button>
    </div>
  {/if}

  {#if noSystem}
    <section class="card no-system">
      <strong>🧩 This avatar doesn't have the Pregnancy System</strong>
      <p class="muted small">The app can't show a belly on it. Add the Pregnancy System in Unity to use it here.</p>
      <div class="row">
        <button class="small" onclick={() => api.openSystemPage().catch(errorToast)}>Get the Pregnancy System</button>
        <button class="small soft" onclick={() => (app.view = "avatars")}>Avatars</button>
      </div>
    </section>
  {/if}

  {#if reveal}
    <RevealCard r={reveal} {p} />
  {/if}

  {#if p}
    <section class="card hero">
      <ProgressRing value={Math.min(1, pr)} size={190} stroke={16}>
        <span class="pct">{Math.floor(Math.min(pr, 9.99) * 100)}%</span>
        <span class="muted small">Week {Math.floor(week)} of 40</span>
      </ProgressRing>
      <div class="facts">
        <div class="fact">
          <span class="muted small">Due date</span>
          <strong>{paused ? "Paused ⏸" : dateFmt.format(due)}</strong>
          <span class="muted small">
            {#if paused}while paused{:else if isDue}{pr > 1.01 ? `${humanDuration(app.now - due)} overdue` : "any moment now"}{:else}{humanDuration(due - app.now)} to go{/if}
          </span>
        </div>
        <div class="fact">
          <span class="muted small">Babies</span>
          {#if week < BABY_REVEAL_WEEK}
            <strong>Too early to tell</strong>
            <span class="muted small">how many at week {BABY_REVEAL_WEEK}</span>
          {:else if genders && week >= GENDER_REVEAL_WEEK}
            <strong>{genderEmoji(p.genders)} {p.babyCount > 1 ? babyWord(p.babyCount) : ""}</strong>
            <span class="small">{genderText(p.genders)}</span>
          {:else}
            <strong>{"👶".repeat(Math.min(p.babyCount, 4))} {babyWord(p.babyCount)}</strong>
            {#if genders}<span class="muted small">{p.babyCount > 1 ? "genders" : "boy or girl"} at week {GENDER_REVEAL_WEEK}</span>{/if}
          {/if}
        </div>
      </div>
      {#if bellyHidden && !viewingOther}
        <div class="hidden-note">🙈 <span><strong>Your belly is hidden.</strong> Turn off Hide Belly in the Pregnancy System menu to see it.</span></div>
      {/if}
      {#if isDue}
        <button class="big" onclick={birth}>{p.babyCount > 1 && week >= BABY_REVEAL_WEEK ? "Welcome your babies 🍼" : "Welcome your baby 🍼"}</button>
      {/if}
      <div class="row">
        <button class="soft" onclick={togglePause}>{paused ? "▶ Resume" : "⏸ Pause"}</button>
        <button class="soft" onclick={openAdjust}>Adjust progress</button>
      </div>
      {#if adjusting}
        <div class="adjust">
          <label class="field">Jump to week {weekInput}
            <input type="range" min="0" max="40" step="1" bind:value={weekInput} />
          </label>
          <div class="row">
            <button class="small" onclick={applyWeek}>Set week {weekInput}</button>
            <button class="small ghost" onclick={() => (adjusting = false)}>Cancel</button>
          </div>
        </div>
      {/if}
      {#if shared.length}
        <span class="muted small">Shared with {shared.join(", ")}</span>
      {/if}
    </section>

    <MovementCard {p} />

    <PartnerCard {p} />

    <section class="card">
      <h2>⏱ Length</h2>
      <SpeedPicker value={currentSpeed} onchange={changeSpeed} />
      <p class="muted small" style="margin-top:10px">Changing this keeps the belly where it is; only the time left changes.</p>
    </section>

    <div class="row end">
      <button class="ghost small" onclick={endEarly}>End early</button>
    </div>
  {:else if w}
    <WaitingCard {w} />
  {:else}
    {#if app.data?.settings.fertility}
      <CycleCard />
    {:else}
      <section class="card hero">
        <div class="blob" aria-hidden="true">🤍</div>
        <h1>Not pregnant right now</h1>
      </section>
    {/if}
    <p class="muted small center">
      {app.data?.settings.fertility ? "After a WooHoo in-game, you'll find out after a little while if a little one is on the way." : "A pregnancy starts as soon as you WooHoo in-game."}
    </p>

    <section class="card">
      <h2>⏱ Length</h2>
      <SpeedPicker value={startSpeed} onchange={(s) => { startSpeed = s; rememberLength(s); }} />
      <p class="muted small" style="margin-top:10px">Used for the next pregnancy, and your cycle scales with it.</p>
    </section>
    <div class="row end">
      <button class="ghost small" onclick={start} disabled={busy}>Start a pregnancy now 💕</button>
    </div>
  {/if}

  <AllPregnancies />

  <p class="sync small muted">
    {#if viewingOther}
      You're not wearing this avatar, so nothing is sent for it right now.
    {:else if status?.oscActive && status.vrchatRunning && p}
      💗 Controlling your avatar's belly{status.lastStage != null ? ` (${Math.round(status.lastStage * 100)}%)` : ""}
    {:else if status?.oscActive && status.vrchatRunning && w}
      💗 Holding your avatar's belly flat until you find out
    {:else if status?.oscActive && status.vrchatRunning}
      💗 Linked to VRChat. Your avatar's own menu works until a pregnancy starts.
    {:else if status?.vrchatRunning}
      VRChat is running, waiting for OSC. Turn it on in the Action Menu > Options > OSC.
    {:else}
      VRChat isn't running. The app links up on its own when it starts.
    {/if}
  </p>
</div>

<style>
  .no-system {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .no-system p {
    margin: 0;
  }
  .no-system .row {
    gap: 8px;
    flex-wrap: wrap;
  }
  .hidden-note {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    padding: 8px 12px;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--warn) 18%, transparent);
    font-size: 0.9rem;
    text-align: left;
  }
  .page {
    max-width: 560px;
    margin: 0 auto;
    padding: 6px 16px 40px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .viewing {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
    background: var(--bg-3);
    border-radius: 14px;
    padding: 8px 12px;
  }
  .hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 14px;
    padding: 26px 18px;
  }
  .blob {
    font-size: 3.2rem;
  }
  .pct {
    font-size: 2.8rem;
    font-weight: 900;
    line-height: 1;
  }
  .facts {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    width: 100%;
  }
  .fact {
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border-radius: 16px;
    padding: 10px;
  }
  .fact strong {
    font-size: 1.05rem;
  }
  .adjust {
    width: 100%;
    max-width: 360px;
    text-align: left;
  }
  .adjust input {
    width: 100%;
  }
  .big {
    width: 100%;
    padding: 13px;
    font-size: 1.05rem;
  }
  .end {
    justify-content: center;
  }
  .center {
    text-align: center;
    margin: -6px 0 0;
  }
  .sync {
    text-align: center;
    margin: 0;
  }
</style>
