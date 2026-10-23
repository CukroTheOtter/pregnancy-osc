<script lang="ts">
  import { onMount } from "svelte";
  import Toggle from "../components/Toggle.svelte";
  import SpeciesPicker from "../components/SpeciesPicker.svelte";
  import { app, errorToast, toast } from "../lib/app.svelte";
  import { api, inTauri, pickFolder, pickTextFile, saveTextFile } from "../lib/api";
  import { PALETTES } from "../lib/palettes";
  import type { MappingInfo, Settings } from "../lib/types";

  let s = $state<Settings>(structuredClone($state.snapshot(app.data!.settings)) as Settings);
  let mapInfo = $state<MappingInfo | null>(null);
  let logDirs = $state<string[]>([]);
  let timer: ReturnType<typeof setTimeout> | null = null;

  onMount(async () => {
    try {
      mapInfo = await api.getMapping();
      logDirs = await api.defaultLogDirs();
    } catch (e) {
      errorToast(e);
    }
  });

  /** Saves after a short pause so sliders don't spam. */
  function commit(now = false) {
    app.data!.settings = structuredClone($state.snapshot(s)) as Settings;
    if (timer) clearTimeout(timer);
    const run = async () => {
      try {
        const note = await api.saveSettings($state.snapshot(s) as Settings);
        if (note) toast("Start with SteamVR", note, "info", 12000);
      } catch (e) {
        errorToast(e);
        // Put the toggle back if the change failed.
        s = structuredClone($state.snapshot(app.data!.settings)) as Settings;
      }
    };
    if (now) run();
    else timer = setTimeout(run, 400);
  }

  async function saveMapping() {
    if (!mapInfo) return;
    try {
      await api.saveMapping($state.snapshot(mapInfo.mapping) as MappingInfo["mapping"]);
      mapInfo = await api.getMapping();
      toast("Parameter names saved", "The OSC link restarts with the new names.", "success");
    } catch (e) {
      errorToast(e);
    }
  }

  async function resetMapping() {
    if (!confirm("Put all parameter names back to the defaults?")) return;
    await api.resetMapping();
    mapInfo = await api.getMapping();
  }

  async function chooseLogDir() {
    const dir = await pickFolder();
    if (dir) {
      s.logDir = dir;
      commit(true);
    }
  }

  async function exportBackup() {
    try {
      const json = await api.exportBackup();
      const d = new Date().toISOString().slice(0, 10);
      if (await saveTextFile(`pregsys-backup-${d}.json`, json)) toast("Backup saved", undefined, "success");
    } catch (e) {
      errorToast(e);
    }
  }

  async function importBackup() {
    const json = await pickTextFile();
    if (!json) return;
    if (!confirm("Replace everything with this backup? (A copy of your current data is kept in the backups folder.)")) return;
    try {
      await api.importBackup(json);
      s = structuredClone($state.snapshot(app.data!.settings)) as Settings;
      toast("Backup restored", undefined, "success");
    } catch (e) {
      errorToast(e);
    }
  }

  async function importTable() {
    const json = await pickTextFile();
    if (!json) return;
    try {
      const n = await api.importCommunityTable(json);
      toast("Species table imported", `${n} avatar names`, "success");
    } catch (e) {
      errorToast(e);
    }
  }

  const KEYS_ORDER = ["stage", "hideBelly", "overdue", "progressionSpeed", "fetalSounds", "appActive", "appHeartbeat", "babyCount", "eggMode", "species", "revealPulse", "kick"];
  const KEY_HELP: Record<string, string> = {
    stage: "Belly size, 0 to 1 (required)",
    hideBelly: "Hide belly toggle",
    overdue: "Overdue toggle (read only)",
    progressionSpeed: "Avatar clock speed (not written yet)",
    fetalSounds: "Womb sounds toggle",
    appActive: "Local: app is running",
    appHeartbeat: "Local: heartbeat",
    babyCount: "Optional: number of babies",
    eggMode: "Optional: egg clutch",
    species: "Optional: species number",
    revealPulse: "Optional: pulse on reveal",
    kick: "Optional: pulse on kick",
  };
  const status = $derived(app.status);
</script>

<div class="view">
  <h1>⚙️ Settings</h1>
  <div class="cols">
    <section class="card stack">
      <h2>Look and feel</h2>
      <div class="row">
        {#each PALETTES as p}
          <button class="swatch" class:on={s.theme.palette === p.id} style="background:{p.accent}" title={p.name} aria-label={p.name} onclick={() => { s.theme.palette = p.id; s.theme.accent = p.accent; commit(); }}>{s.theme.palette === p.id ? "✓" : ""}</button>
        {/each}
        <label class="row small">Accent <input type="color" bind:value={s.theme.accent} onchange={() => commit()} /></label>
      </div>
      <div class="segmented">
        {#each [["system", "Follow system"], ["light", "Light"], ["dark", "Dark"]] as [m, l]}
          <button class:on={s.theme.mode === m} onclick={() => { s.theme.mode = m as Settings["theme"]["mode"]; commit(); }}>{l}</button>
        {/each}
      </div>
    </section>

    <section class="card stack">
      <h2>You</h2>
      <div class="field"><strong>Your usual species</strong><span class="muted small">Avatar profiles can override this.</span></div>
      <SpeciesPicker bind:value={s.userSpecies} onpick={() => commit()} />
    </section>

    <section class="card stack">
      <h2>Babies and surprises</h2>
      <label class="field">Hybrid chance: {Math.round(s.hybridChance * 100)}%
        <input type="range" min="0" max="1" step="0.01" bind:value={s.hybridChance} oninput={() => commit()} />
        <span class="hint">When the parents are different species. Rare by default.</span>
      </label>
      <div class="field"><strong>Spoilers</strong>
        <div class="segmented">
          {#each [["surprise", "Surprise me"], ["hints", "Hints"], ["all", "Show everything"]] as [m, l]}
            <button class:on={s.spoilerMode === m} onclick={() => { s.spoilerMode = m as Settings["spoilerMode"]; commit(); }}>{l}</button>
          {/each}
        </div>
      </div>
      <div class="field"><strong>Default speed for new pregnancies</strong>
        <div class="segmented">
          <button class:on={s.defaultMode === "realtime"} onclick={() => { s.defaultMode = "realtime"; s.defaultSpeed = 1; commit(); }}>Real time</button>
          <button class:on={s.defaultMode === "accelerated" && s.defaultSpeed === 7} onclick={() => { s.defaultMode = "accelerated"; s.defaultSpeed = 7; commit(); }}>1 day = 1 week</button>
          <button class:on={s.defaultMode === "accelerated" && s.defaultSpeed === 168} onclick={() => { s.defaultMode = "accelerated"; s.defaultSpeed = 168; commit(); }}>1 hour = 1 week</button>
        </div>
      </div>
    </section>

    <section class="card stack">
      <h2>Sounds and events</h2>
      <Toggle bind:checked={s.heartbeatSound} label="Heartbeat and kick sounds" hint="During scans and kicks" onchange={() => commit()} />
      <label class="field">Volume<input type="range" min="0" max="1" step="0.05" bind:value={s.soundVolume} oninput={() => commit()} /></label>
      <Toggle bind:checked={s.kickEvents} label="Send kicks to the avatar" hint="Pulses the optional kick parameter" onchange={() => commit()} />
      <Toggle bind:checked={s.revealPulse} label="Send a pulse on scan reveals" hint="For an in-world effect, if your avatar has one" onchange={() => commit()} />
      <Toggle bind:checked={s.notifications} label="Notifications" hint="Milestones and in-game conception" onchange={() => commit()} />
    </section>

    <section class="card stack">
      <h2>Starting up</h2>
      {#if !inTauri}<p class="muted small">These work in the installed app.</p>{/if}
      <Toggle bind:checked={s.startWithSteamvr} label="Start with SteamVR" onchange={() => commit(true)} />
      <Toggle bind:checked={s.startWithOs} label="Start when I log in" onchange={() => commit(true)} />
      <Toggle bind:checked={s.startInTray} label="Start in the tray" hint="When started automatically, stay hidden" onchange={() => commit()} />
      <Toggle bind:checked={s.exitWithSteamvr} label="Quit when SteamVR closes" onchange={() => commit()} />
      {#if status && !status.trayAvailable && inTauri}
        <p class="muted small">No system tray was found, so closing the window quits the app, and "start in tray" starts minimized instead.</p>
      {/if}
    </section>

    <section class="card stack">
      <h2>VRChat link (OSC)</h2>
      <Toggle bind:checked={s.osc.enabled} label="Send to VRChat" onchange={() => commit(true)} />
      <Toggle bind:checked={s.osc.useOscquery} label="Find VRChat automatically (OSCQuery)" hint="Recommended: no ports to set, works next to other OSC apps" onchange={() => commit(true)} />
      {#if !s.osc.useOscquery}
        <div class="grid2">
          <label class="field">Send to port<input type="number" bind:value={s.osc.sendPort} onchange={() => commit(true)} /></label>
          <label class="field">Listen on port<input type="number" bind:value={s.osc.recvPort} onchange={() => commit(true)} /></label>
        </div>
      {/if}
      {#if status}
        <div class="small muted">
          VRChat: {status.vrchatRunning ? "running" : "not running"} ·
          {status.oscActive ? `linked${status.oscquery ? " via OSCQuery" : ""}, sending to ${status.sendTarget}, listening on ${status.recvPort}` : "not linked"}
          {#if status.note}<br />{status.note}{/if}
        </div>
      {/if}
      <h3>Belly defaults</h3>
      <Toggle bind:checked={s.hideBelly} label="Hide belly" onchange={() => commit()} />
      <label class="field">Belly size {Math.round(s.bellyScale * 100)}%<input type="range" min="0.3" max="1.5" step="0.05" bind:value={s.bellyScale} oninput={() => commit()} /></label>
    </section>

    <section class="card stack wide">
      <div class="row">
        <h2 class="grow">OSC parameter names</h2>
        {#if mapInfo}<span class="pill">{mapInfo.syncedBits} synced bits ({mapInfo.syncedBitsAll} with extras)</span>{/if}
      </div>
      <p class="muted small">Match these to your avatar's parameter names. Leave a name empty to turn it off. Saved to <code>{mapInfo?.path}</code>.</p>
      {#if mapInfo}
        <div class="maptable">
          {#each KEYS_ORDER.filter((k) => mapInfo!.mapping.parameters[k]) as key}
            {@const d = mapInfo.mapping.parameters[key]}
            <label class="maprow">
              <span><strong>{key}</strong><br /><span class="muted small">{KEY_HELP[key] ?? ""} · {d.type}{d.synced ? ", synced" : ", local"}</span></span>
              <input bind:value={d.name} placeholder="(off)" />
            </label>
          {/each}
        </div>
        <div class="row">
          <button onclick={saveMapping}>Save names</button>
          <button class="soft" onclick={resetMapping}>Reset to defaults</button>
          {#if inTauri}<button class="ghost" onclick={() => api.openPath(mapInfo!.path)}>Open the file</button>{/if}
        </div>
      {/if}
    </section>

    <section class="card stack">
      <h2>VRChat logs</h2>
      <p class="muted small">Read on this computer only, never uploaded. Used to guess the other parent and name your avatars.</p>
      <Toggle bind:checked={s.readLogs} label="Read VRChat logs" onchange={() => commit(true)} />
      <div class="small">Folder: <code>{s.logDir ?? status?.logDir ?? "auto-detect"}</code></div>
      {#if status?.logFile}<div class="small muted">Reading {status.logFile.split(/[\\/]/).pop()}</div>{/if}
      <div class="row">
        {#if inTauri}<button class="small soft" onclick={chooseLogDir}>Choose folder…</button>{/if}
        {#if s.logDir}<button class="small ghost" onclick={() => { s.logDir = null; commit(true); }}>Auto-detect</button>{/if}
      </div>
      <details class="small"><summary>Where to look</summary>{#each logDirs as d}<div><code>{d}</code></div>{/each}</details>
      <label class="field">Partner guess: quiet minutes {s.settleMinutes}
        <input type="range" min="1" max="60" bind:value={s.settleMinutes} oninput={() => commit()} />
        <span class="hint">Nobody may join or leave for this long for a lone person to count as the partner.</span>
      </label>
      <Toggle bind:checked={s.autoStartOnConception} label="Start a pregnancy on in-game conception" onchange={() => commit()} />
      <button class="small soft" onclick={importTable}>Import a species table…</button>
      <span class="muted small">A JSON file of <code>{'{"avatar name": "fox"}'}</code> pairs, stored locally.</span>
    </section>

    <section class="card stack">
      <h2>Backup</h2>
      <div class="row">
        <button class="soft" onclick={exportBackup}>Export backup</button>
        <button class="soft" onclick={importBackup}>Import backup</button>
      </div>
      <p class="muted small">The app also keeps a daily backup for a week{status?.dataDir ? ` in ${status.dataDir}` : ""}.</p>
      {#if inTauri && status?.dataDir}<button class="small ghost" onclick={() => api.openPath(status!.dataDir)}>Open data folder</button>{/if}
    </section>

    <section class="card stack">
      <h2>About</h2>
      <p class="small">Preg Sys Companion 0.1. Nothing you do here is sent anywhere except OSC to VRChat on this computer.</p>
      {#if !inTauri}
        <button class="soft small" onclick={() => api.simulateConception()}>Preview: simulate in-game conception</button>
      {/if}
      {#if inTauri}<button class="danger" onclick={() => api.quitApp()}>Quit app</button>{/if}
    </section>
  </div>
</div>

<style>
  .cols { display: grid; grid-template-columns: repeat(auto-fill, minmax(340px, 1fr)); gap: 16px; align-items: start; }
  .wide { grid-column: 1 / -1; }
  .swatch { width: 36px; height: 36px; border-radius: 50%; padding: 0; color: #fff; border: 3px solid transparent; }
  .swatch.on { border-color: var(--text); }
  .maptable { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 8px 16px; }
  .maprow { display: grid; grid-template-columns: 1fr 1.2fr; gap: 10px; align-items: center; }
  code { background: var(--bg-3); padding: 1px 6px; border-radius: 6px; font-size: 0.85em; word-break: break-all; }
  details summary { cursor: pointer; }
  @media (max-width: 420px) { .cols { grid-template-columns: 1fr; } .maprow { grid-template-columns: 1fr; } }
</style>
