<script lang="ts">
  import ProgressRing from "../components/ProgressRing.svelte";
  import { app, avatarPregnancy, updatePregnancy, errorToast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { progress, retime, setProgress, speedLabel, week, TERM_WEEKS, humanDuration } from "../lib/timeline";
  import type { DurationMode, Pregnancy } from "../lib/types";

  const list = $derived([...(app.data?.pregnancies ?? [])].sort((a, b) => Number(!!a.endedMs) - Number(!!b.endedMs) || b.createdMs - a.createdMs));
  const onAvatar = $derived(avatarPregnancy());
  let editing = $state<string | null>(null);
  let weekInput = $state(0);
  let speedMode = $state<DurationMode>("accelerated");
  let speedVal = $state(7);
  let customHours = $state(48);

  function edit(p: Pregnancy) {
    editing = editing === p.id ? null : p.id;
    weekInput = Math.round(week(p.timeline, Date.now()));
    speedMode = p.timeline.mode;
    speedVal = p.timeline.speed;
    customHours = Math.round(p.timeline.durationMs / 3_600_000);
  }

  async function applyWeek(p: Pregnancy) {
    await updatePregnancy(p.id, (x) => (x.timeline = setProgress(x.timeline, Date.now(), weekInput / TERM_WEEKS)));
  }

  async function applySpeed(p: Pregnancy) {
    await updatePregnancy(p.id, (x) => (x.timeline = retime(x.timeline, Date.now(), speedMode, speedVal, customHours * 3_600_000)));
  }

  async function show(p: Pregnancy) {
    try {
      await api.setCurrentPregnancy(app.data!.currentAvatarId, p.id);
      app.selectedId = null;
      app.view = "home";
    } catch (e) {
      errorToast(e);
    }
  }

  async function end(p: Pregnancy) {
    if (confirm(`End "${p.name}"? The belly goes back to flat. It stays in your history.`)) await api.endPregnancy(p.id);
  }

  async function del(p: Pregnancy) {
    if (confirm(`Delete "${p.name}" for good? Its journal entries and keepsakes go too.`)) await api.deletePregnancy(p.id);
  }

  const dateFmt = new Intl.DateTimeFormat(undefined, { dateStyle: "medium" });
</script>

<div class="view">
  <header class="row">
    <h1 class="grow">🍼 Pregnancies</h1>
    <button onclick={() => (app.showNew = true)}>+ New</button>
  </header>

  {#if list.length === 0}
    <div class="card"><p class="muted">None yet.</p></div>
  {/if}

  <div class="list">
    {#each list as p (p.id)}
      {@const pr = progress(p.timeline, app.now)}
      <div class="card item" class:ended={!!p.endedMs}>
        <div class="row">
          <ProgressRing value={Math.min(1, pr)} size={64} stroke={7}>
            <strong>{Math.floor(pr * 40)}</strong>
          </ProgressRing>
          <div class="grow">
            <strong>{p.name}</strong>
            {#if onAvatar?.id === p.id}<span class="pill ok">On avatar</span>{/if}
            {#if p.endedMs}<span class="pill off">Ended {dateFmt.format(p.endedMs)}</span>{/if}
            {#if p.timeline.pausedAtMs}<span class="pill warn">Paused</span>{/if}
            <div class="muted small">
              {p.babyCount > 1 ? `${p.babyCount} babies · ` : ""}{p.kind} · {speedLabel(p.timeline)} · started {dateFmt.format(p.createdMs)}
              {#if p.partner.name}· with {p.partner.name}{/if}
            </div>
          </div>
          {#if !p.endedMs}
            <button class="small" onclick={() => show(p)}>{onAvatar?.id === p.id ? "Open" : "Show on avatar"}</button>
            <button class="small soft" onclick={() => edit(p)}>Adjust</button>
            <button class="small ghost" onclick={() => end(p)}>End</button>
          {/if}
          <button class="small ghost" onclick={() => del(p)} aria-label="Delete {p.name}">🗑</button>
        </div>

        {#if editing === p.id && !p.endedMs}
          <div class="adjust grid2">
            <div class="stack">
              <label class="field">Name<input value={p.name} onchange={(e) => updatePregnancy(p.id, (x) => (x.name = (e.target as HTMLInputElement).value || x.name))} /></label>
              <label class="field">Jump to week {weekInput}
                <input type="range" min="0" max="42" bind:value={weekInput} />
                <span class="hint">Moves the start time so the belly matches. Good for back-dating.</span>
              </label>
              <button class="small soft" onclick={() => applyWeek(p)}>Set week</button>
            </div>
            <div class="stack">
              <div class="field">Speed
                <div class="segmented">
                  <button class:on={speedMode === "realtime"} onclick={() => { speedMode = "realtime"; speedVal = 1; }}>Real time</button>
                  <button class:on={speedMode === "accelerated"} onclick={() => (speedMode = "accelerated")}>Faster</button>
                  <button class:on={speedMode === "custom"} onclick={() => (speedMode = "custom")}>Custom</button>
                </div>
              </div>
              {#if speedMode === "accelerated"}
                <label class="field">{speedVal}× faster ({humanDuration((280 * 86_400_000) / speedVal)} total)
                  <input type="range" min="2" max="672" step="1" bind:value={speedVal} />
                </label>
              {:else if speedMode === "custom"}
                <label class="field">Full term in hours<input type="number" min="1" bind:value={customHours} /></label>
              {/if}
              <span class="muted small">Changing speed never makes the belly jump; only the time left changes.</span>
              <button class="small soft" onclick={() => applySpeed(p)}>Apply speed</button>
            </div>
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .list { display: flex; flex-direction: column; gap: 12px; margin-top: 12px; }
  .item.ended { opacity: 0.7; }
  .adjust { margin-top: 14px; padding-top: 14px; border-top: 1.5px dashed var(--line); }
</style>
