<script lang="ts">
  import XrayBelly from "../components/XrayBelly.svelte";
  import ProgressRing from "../components/ProgressRing.svelte";
  import PartnerCard from "../components/PartnerCard.svelte";
  import SpeciesPicker from "../components/SpeciesPicker.svelte";
  import { app, currentPregnancy, avatarPregnancy, activePregnancies, updatePregnancy } from "../lib/app.svelte";
  import { babyViews } from "../lib/look";
  import { dueReveals, nextReveal, sizeOf, TRIMESTER_COLORS } from "../lib/reveals";
  import { dueMs, humanDuration, pause, progress, resume, speedLabel, trimester, TERM_WEEKS } from "../lib/timeline";
  import { blendLabel } from "../lib/genetics";
  import { speciesName } from "../lib/species";
  import { api } from "../lib/api";
  import type { BodyKind } from "../lib/types";

  const p = $derived(currentPregnancy());
  const onAvatar = $derived(avatarPregnancy());
  let scrub = $state<number | null>(null);
  let showSpecies = $state(false);

  const live = $derived(p ? progress(p.timeline, app.now) : 0);
  const shown = $derived(scrub != null ? scrub / TERM_WEEKS : live);
  const w = $derived(shown * TERM_WEEKS);
  const tri = $derived(trimester(Math.min(shown, 0.999)));
  const views = $derived(p && app.data ? babyViews(p, app.data) : []);
  const due = $derived(p ? dueMs(p.timeline, app.now) : 0);
  const scans = $derived(p ? dueReveals(p, app.now) : []);
  const upcoming = $derived(p ? nextReveal(p, app.now) : undefined);
  const others = $derived(activePregnancies().filter((x) => x.id !== p?.id));
  const status = $derived(app.status);
  const avatarName = $derived.by(() => {
    const id = app.data?.currentAvatarId;
    return id ? (app.data?.avatars[id]?.name ?? "Current avatar") : null;
  });

  function setKind(kind: BodyKind) {
    if (!p) return;
    updatePregnancy(p.id, (x) => {
      x.kind = kind;
      if (kind === "human") x.motherSpecies = "human";
      else if (x.motherSpecies === "human") {
        x.motherSpecies = app.data!.settings.userSpecies === "human" ? "fox" : app.data!.settings.userSpecies;
        showSpecies = true;
      }
    });
  }

  function togglePause() {
    if (!p) return;
    updatePregnancy(p.id, (x) => (x.timeline = x.timeline.pausedAtMs != null ? resume(x.timeline, Date.now()) : pause(x.timeline, Date.now())));
  }

  const dateFmt = new Intl.DateTimeFormat(undefined, { weekday: "short", month: "short", day: "numeric", hour: "numeric", minute: "2-digit" });
</script>

<div class="view">
  {#if !p}
    <div class="empty card">
      <div class="art"><XrayBelly progress={0} hideBaby /></div>
      <div class="stack">
        <h1>No little bean yet</h1>
        <p class="muted">Start a pregnancy here, or let it begin on its own when conception happens in-game.</p>
        <div class="row">
          <button onclick={() => (app.showNew = true)}>Start a pregnancy 💕</button>
          {#if others.length}
            <button class="soft" onclick={() => (app.view = "pregnancies")}>Pick an existing one</button>
          {/if}
        </div>
        {#if status && !status.vrchatRunning}
          <p class="muted small">VRChat isn't running, so the app is resting quietly. It links up automatically when VRChat starts.</p>
        {/if}
      </div>
    </div>
  {:else}
    <header class="row top">
      <div class="grow">
        <h1>{p.name}</h1>
        <div class="row small">
          {#if onAvatar?.id === p.id}
            <span class="pill ok">On {avatarName ?? "your avatar"}</span>
          {:else}
            <span class="pill off">Not on this avatar</span>
            <button class="small soft" onclick={() => api.setCurrentPregnancy(app.data!.currentAvatarId, p!.id)}>Show on avatar</button>
          {/if}
          {#if p.timeline.pausedAtMs}<span class="pill warn">Paused</span>{/if}
          {#if p.source === "conception"}<span class="pill">Started in-game</span>{/if}
        </div>
      </div>
      {#if others.length}
        <select aria-label="Switch pregnancy" value={p.id} onchange={(e) => (app.selectedId = (e.target as HTMLSelectElement).value)}>
          <option value={p.id}>{p.name}</option>
          {#each others as o}<option value={o.id}>{o.name}</option>{/each}
        </select>
      {/if}
      <button class="soft" onclick={togglePause}>{p.timeline.pausedAtMs ? "▶ Resume" : "⏸ Pause"}</button>
    </header>

    {#if scans.length}
      <button class="scanbar" onclick={() => (app.view = "scans")}>
        💫 {scans.length === 1 ? "A scan is ready!" : `${scans.length} scans are ready!`} Tap to see your little one.
      </button>
    {/if}

    <div class="layout">
      <section class="card xray-card">
        <div class="xray-wrap">
          <XrayBelly progress={shown} babies={views.map((v) => v.look)} egg={p.kind === "egg"} kick={scrub == null ? app.kick : 0} label="X-ray belly at week {Math.floor(w)}" />
        </div>
        <div class="scrub">
          <input
            type="range"
            min="0"
            max="42"
            step="0.25"
            value={scrub ?? Math.min(42, live * TERM_WEEKS)}
            oninput={(e) => (scrub = +(e.target as HTMLInputElement).value)}
            aria-label="Preview any week"
          />
          <div class="row small">
            <span class="grow muted">{scrub != null ? `Previewing week ${Math.floor(scrub)}` : "Drag to preview any week"}</span>
            {#if scrub != null}<button class="small soft" onclick={() => (scrub = null)}>Back to now</button>{/if}
          </div>
        </div>
        <div class="row babies">
          {#each views as v}
            <span class="pill">{v.name}{#if app.data?.settings.spoilerMode === "all" || p.reveals.species}: {blendLabel(v.result.blend)}{/if}</span>
          {/each}
        </div>
      </section>

      <section class="side">
        <div class="card stage">
          <ProgressRing value={Math.min(1, shown)} color={TRIMESTER_COLORS[tri]} size={170}>
            <span class="muted small">Week</span>
            <span class="big">{Math.floor(w)}</span>
            <span class="muted small">of 40</span>
          </ProgressRing>
          <div class="stack info">
            <span class="pill" style="background:{TRIMESTER_COLORS[tri]};color:#4a2f3d">Trimester {tri}</span>
            <div>
              <div class="muted small">{p.kind === "egg" ? "Each egg is about the size of" : p.babyCount > 1 ? "Each baby is about the size of" : "Baby is about the size of"}</div>
              <strong class="size">{sizeOf(w, p.kind === "egg")}</strong>
            </div>
            {#if scrub == null}
              <div class="small">
                {#if live >= 1}
                  <strong>Due!</strong> {live > 1.02 ? `${humanDuration(app.now - due)} overdue` : "Any moment now"}
                {:else}
                  Due {dateFmt.format(due)} <span class="muted">({humanDuration(due - app.now)} left)</span>
                {/if}
              </div>
            {/if}
            <div class="muted small">{speedLabel(p.timeline)}</div>
          </div>
        </div>

        <div class="card">
          <div class="row">
            <h3 class="grow">Mode</h3>
            <div class="segmented" role="radiogroup" aria-label="Mode">
              {#each [["human", "👶 Human"], ["furry", "🦊 Furry"], ["egg", "🥚 Egg"]] as [k, l]}
                <button class:on={p.kind === k} role="radio" aria-checked={p.kind === k} onclick={() => setKind(k as BodyKind)}>{l}</button>
              {/each}
            </div>
          </div>
          {#if p.kind !== "human"}
            <button class="soft small" style="margin-top:8px" onclick={() => (showSpecies = !showSpecies)}>Your species: {speciesName(p.motherSpecies)} {showSpecies ? "▴" : "▾"}</button>
            {#if showSpecies}
              <div style="margin-top:10px">
                <SpeciesPicker value={p.motherSpecies} exclude={["human"]} onpick={(id) => updatePregnancy(p!.id, (x) => (x.motherSpecies = id))} />
              </div>
            {/if}
          {/if}
        </div>

        <PartnerCard {p} />

        {#if upcoming && !scans.length}
          <div class="card small">
            <strong>Next scan:</strong> {app.data?.settings.spoilerMode === "surprise" ? "a surprise" : upcoming.label} at week {upcoming.week}
          </div>
        {/if}

        {#if status?.vrchatRunning}
          <div class="card small sync">
            <strong>Avatar sync</strong>
            <div class="muted">
              {status.oscActive ? `Belly value ${status.lastStage != null ? status.lastStage.toFixed(3) : "-"} sent` : "Waiting for OSC"}
              {#if status.avatarOverdue}· avatar in overdue mode{/if}
            </div>
          </div>
        {/if}
      </section>
    </div>
  {/if}
</div>

<style>
  .top { margin-bottom: 14px; align-items: flex-start; }
  .layout { display: grid; grid-template-columns: minmax(0, 1.25fr) minmax(280px, 1fr); gap: 18px; }
  @media (max-width: 900px) { .layout { grid-template-columns: 1fr; } }
  .xray-card { display: flex; flex-direction: column; gap: 10px; }
  .xray-wrap { aspect-ratio: 400 / 480; max-height: 520px; margin: 0 auto; width: 100%; }
  .scrub input { width: 100%; }
  .babies { justify-content: center; }
  .side { display: flex; flex-direction: column; gap: 14px; }
  .stage { display: flex; gap: 18px; align-items: center; flex-wrap: wrap; }
  .info { flex: 1; min-width: 160px; gap: 8px; }
  .big { font-size: 3rem; font-weight: 900; line-height: 1; }
  .size { font-size: 1.2rem; text-transform: capitalize; }
  .scanbar {
    width: 100%;
    margin-bottom: 14px;
    justify-content: flex-start;
    white-space: normal;
    text-align: left;
    border-radius: 18px;
    background: linear-gradient(90deg, var(--accent), var(--accent-2));
    animation: glow 2s ease-in-out infinite alternate;
  }
  @keyframes glow { to { filter: brightness(1.08); } }
  .empty { display: grid; grid-template-columns: minmax(0, 260px) 1fr; gap: 24px; align-items: center; max-width: 760px; margin: 40px auto; }
  @media (max-width: 600px) { .empty { grid-template-columns: 1fr; } }
  .art { max-width: 260px; }
  select { max-width: 200px; }
</style>
