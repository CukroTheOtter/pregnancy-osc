<script lang="ts">
  import Modal from "./Modal.svelte";
  import SpeciesPicker from "./SpeciesPicker.svelte";
  import Toggle from "./Toggle.svelte";
  import { app, errorToast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { PALETTES } from "../lib/palettes";

  const s = $state(structuredClone($state.snapshot(app.data!.settings)));
  let step = $state(0);

  async function finish() {
    s.onboarded = true;
    try {
      await api.saveSettings(s);
      app.data!.settings = s;
    } catch (e) {
      errorToast(e);
    }
  }
</script>

<Modal title={["Welcome! 💕", "About you", "Make it yours", "Privacy"][step]} onclose={finish} wide>
  {#if step === 0}
    <div class="stack">
      <p>Preg Sys Companion tracks a pregnancy from its start time and keeps your VRChat avatar's belly in sync over OSC.</p>
      <p class="muted">When VRChat is closed it's a cozy tracker: watch your little one grow, plan their traits in the Gene Lab and collect ultrasound keepsakes.</p>
      <p class="muted small">To link with VRChat: Action Menu > Options > OSC > Enabled.</p>
    </div>
  {:else if step === 1}
    <div class="stack">
      <p>What species are you usually? Avatar profiles can override this per avatar.</p>
      <SpeciesPicker bind:value={s.userSpecies} />
    </div>
  {:else if step === 2}
    <div class="stack">
      <div class="field">
        <strong>Color theme</strong>
        <div class="row">
          {#each PALETTES as p}
            <button class="swatch" class:on={s.theme.palette === p.id} style="background:{p.accent}" onclick={() => { s.theme.palette = p.id; s.theme.accent = p.accent; app.data!.settings.theme = { ...s.theme }; }} aria-label={p.name}>
              {s.theme.palette === p.id ? "✓" : ""}
            </button>
          {/each}
        </div>
      </div>
      <div class="field">
        <strong>Light or dark</strong>
        <div class="segmented">
          {#each [["system", "Follow system"], ["light", "Light"], ["dark", "Dark"]] as [m, l]}
            <button class:on={s.theme.mode === m} onclick={() => { s.theme.mode = m as typeof s.theme.mode; app.data!.settings.theme = { ...s.theme }; }}>{l}</button>
          {/each}
        </div>
      </div>
      <div class="field">
        <strong>Default speed for new pregnancies</strong>
        <div class="segmented">
          <button class:on={s.defaultMode === "realtime"} onclick={() => { s.defaultMode = "realtime"; s.defaultSpeed = 1; }}>Real time</button>
          <button class:on={s.defaultMode === "accelerated" && s.defaultSpeed === 7} onclick={() => { s.defaultMode = "accelerated"; s.defaultSpeed = 7; }}>1 day = 1 week</button>
          <button class:on={s.defaultMode === "accelerated" && s.defaultSpeed === 168} onclick={() => { s.defaultMode = "accelerated"; s.defaultSpeed = 168; }}>1 hour = 1 week</button>
        </div>
      </div>
    </div>
  {:else}
    <div class="stack">
      <p>To guess who the other parent was, the app can read VRChat's log files <strong>on this computer only</strong>. Nothing is ever uploaded, and you can forget anyone at any time.</p>
      <Toggle bind:checked={s.readLogs} label="Read VRChat logs" hint="Used for the partner guess and avatar names" />
      <Toggle bind:checked={s.autoStartOnConception} label="Start a pregnancy when conception happens in-game" hint="Needs an avatar with a fertility system" />
    </div>
  {/if}

  <div class="row" style="justify-content:space-between;margin-top:18px">
    <span class="muted small">Step {step + 1} of 4</span>
    <div class="row">
      {#if step > 0}<button class="soft" onclick={() => step--}>Back</button>{/if}
      {#if step < 3}
        <button onclick={() => step++}>Next</button>
      {:else}
        <button onclick={finish}>Let's go! ✨</button>
      {/if}
    </div>
  </div>
</Modal>

<style>
  .swatch { width: 40px; height: 40px; border-radius: 50%; padding: 0; color: #fff; border: 3px solid transparent; }
  .swatch.on { border-color: var(--text); }
</style>
