<script lang="ts">
  import Modal from "./Modal.svelte";
  import SpeciesPicker from "./SpeciesPicker.svelte";
  import { app, errorToast, toast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import type { BodyKind, DurationMode } from "../lib/types";
  import { durationFor, humanDuration, TERM_WEEKS } from "../lib/timeline";

  let { onclose }: { onclose: () => void } = $props();

  const s = app.data!.settings;
  const aid = app.data!.currentAvatarId;
  const prof = aid ? app.data!.avatars[aid] : undefined;

  let name = $state("");
  let mode = $state<DurationMode>(s.defaultMode);
  let speed = $state(s.defaultSpeed);
  let customHours = $state(48);
  let babyCount = $state(1);
  let kind = $state<BodyKind>(prof?.kind ?? (s.userSpecies === "human" ? "human" : "furry"));
  let motherSpecies = $state(prof?.species ?? s.userSpecies);
  let startWeeks = $state(0);
  let busy = $state(false);

  const duration = $derived(durationFor(mode, speed, customHours * 3_600_000));
  const startMs = $derived(Date.now() - (startWeeks / TERM_WEEKS) * duration);

  const SPEEDS = [
    { v: 1, label: "Real time" },
    { v: 7, label: "1 day = 1 week" },
    { v: 28, label: "1 day = 4 weeks" },
    { v: 168, label: "1 hour = 1 week" },
  ];

  async function create() {
    busy = true;
    try {
      await api.createPregnancy({
        name,
        startMs: startWeeks > 0 ? startMs : null,
        mode,
        speed,
        customMs: mode === "custom" ? customHours * 3_600_000 : null,
        babyCount,
        kind,
        motherSpecies: kind === "human" ? "human" : motherSpecies,
        source: "manual",
      });
      app.selectedId = null;
      app.view = "home";
      toast("A new little bean!", `${name || "Your pregnancy"} has started.`, "success");
      onclose();
    } catch (e) {
      errorToast(e);
    } finally {
      busy = false;
    }
  }
</script>

<Modal title="New pregnancy" {onclose} wide>
  <div class="stack">
    <label class="field">Name <span class="hint">Optional, like "Little Bean"</span>
      <input bind:value={name} placeholder="Little Bean" maxlength="40" />
    </label>

    <div class="field">
      <strong>Type</strong>
      <div class="segmented" role="radiogroup" aria-label="Type">
        {#each [["human", "👶 Human"], ["furry", "🦊 Furry"], ["egg", "🥚 Egg"]] as [k, l]}
          <button class:on={kind === k} role="radio" aria-checked={kind === k} onclick={() => (kind = k as BodyKind)}>{l}</button>
        {/each}
      </div>
    </div>

    {#if kind !== "human"}
      <div class="field">
        <strong>Your species</strong>
        <SpeciesPicker bind:value={motherSpecies} exclude={["human"]} />
      </div>
    {/if}

    <div class="field">
      <strong>Speed</strong>
      <div class="segmented" role="radiogroup" aria-label="Speed">
        {#each SPEEDS as sp}
          <button
            class:on={(sp.v === 1 && mode === "realtime") || (mode === "accelerated" && speed === sp.v)}
            onclick={() => {
              mode = sp.v === 1 ? "realtime" : "accelerated";
              speed = sp.v;
            }}>{sp.label}</button
          >
        {/each}
        <button class:on={mode === "custom"} onclick={() => (mode = "custom")}>Custom</button>
      </div>
      {#if mode === "custom"}
        <label class="field">Full term takes (hours)
          <input type="number" min="1" max="20000" bind:value={customHours} />
        </label>
      {/if}
      <span class="muted small">Full term takes {humanDuration(duration)}.</span>
    </div>

    <div class="grid2">
      <label class="field">Babies
        <input type="range" min="1" max="8" bind:value={babyCount} />
        <span class="hint">{babyCount === 1 ? "One" : babyCount === 2 ? "Twins!" : babyCount === 3 ? "Triplets!" : `${babyCount} babies`}</span>
      </label>
      <label class="field">Already along? (weeks)
        <input type="range" min="0" max="40" step="1" bind:value={startWeeks} />
        <span class="hint">{startWeeks === 0 ? "Starting now" : `Back-dated to week ${startWeeks} (conceived ${humanDuration(Date.now() - startMs)} ago)`}</span>
      </label>
    </div>

    <div class="row" style="justify-content:flex-end">
      <button class="soft" onclick={onclose}>Cancel</button>
      <button onclick={create} disabled={busy}>Start pregnancy 💕</button>
    </div>
  </div>
</Modal>
