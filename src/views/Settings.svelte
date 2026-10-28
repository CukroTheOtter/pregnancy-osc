<script lang="ts">
  import { onMount } from "svelte";
  import Toggle from "../components/Toggle.svelte";
  import Section from "../components/Section.svelte";
  import AccentPicker from "../components/AccentPicker.svelte";
  import DebugPanel from "../components/DebugPanel.svelte";
  import LinkInfo from "../components/LinkInfo.svelte";
  import { app, checkForUpdate, confirmAsk, errorToast, toast } from "../lib/app.svelte";
  import { api, inTauri, pickFolder, pickTextFile, saveTextFile } from "../lib/api";
  import { MULTIPLES_LABEL } from "../lib/fertility";
  import { HOLE_TYPES } from "../lib/sps";
  import { FEATURES, FIXED, PARAM_INFO, PARAM_ORDER, REQUIRED, SYNC_ROLES, hasParam, syncLabel } from "../lib/params";
  import { week as weekOf } from "../lib/timeline";
  import { pregnancyFor } from "../lib/scope";
  import type { LaborStart, MappingInfo, Multiples, ResetKind, Settings } from "../lib/types";

  // Chatbox events, in order, with their default text (keys and texts match pregsys-core/src/chatbox.rs).
  const CHAT_EVENTS = [
    { id: "started", label: "Pregnancy started", def: "🤰 A little one is on the way!" },
    { id: "paused", label: "Paused", def: "⏸️ Pregnancy paused (week {week})" },
    { id: "resumed", label: "Resumed", def: "▶️ Pregnancy resumed (week {week})" },
    { id: "stopped", label: "Pregnancy stopped", def: "Pregnancy stopped" },
    { id: "labor", label: "Labor started", def: "🤰 Labor has started!", off: true },
    { id: "waterBroke", label: "Water broke", def: "💦 Water broke!", off: true },
    { id: "born", label: "Baby born", def: "🍼 The baby is here!" },
    { id: "woohoo", label: "WooHoo", def: "💕 WooHoo!" },
    { id: "notPregnant", label: "Not pregnant this time", def: "Not pregnant this time" },
    { id: "achievement", label: "Achievement unlocked", def: "🏆 {achievement}" },
  ];
  // Your own accent color: Custom opens the picker (closes it again if open).
  let pickingColor = $state(false);
  function openCustom() {
    if (s.theme.customAccent && pickingColor) {
      pickingColor = false;
      return;
    }
    // Starts from the color you see now.
    if (!s.theme.customAccent) s.theme.accent = PALETTES.find((p) => p.id === s.theme.palette)?.color ?? s.theme.accent;
    s.theme.customAccent = true;
    pickingColor = true;
    commit();
  }

  // Palettes in src/app.css, with their accent color.
  const PALETTES = [
    { id: "strawberry", label: "Strawberry", color: "#f59ac0" },
    { id: "lavender", label: "Lavender", color: "#b39bff" },
    { id: "mint", label: "Mint", color: "#6fcfac" },
    { id: "peach", label: "Peach", color: "#ffad7a" },
    { id: "sky", label: "Sky", color: "#7fb8f5" },
  ];
  const LABOR_STARTS: { k: LaborStart; l: string; h: string }[] = [
    { k: "random", l: "Surprise", h: "Some day from week 37 to 42" },
    { k: "dueDate", l: "Due date", h: "Right on the due date" },
    { k: "manual", l: "When I say", h: "You start it (Home or the avatar's menu)" },
  ];
  const SHOW_FOR: [number, string][] = [[5, "5 s"], [8, "8 s"], [15, "15 s"], [30, "30 s"]];
  function withChatRows(x: Settings): Settings {
    // Empty texts show the default, typed out, so it can be edited.
    for (const ev of CHAT_EVENTS) {
      const m = (x.chatbox.messages[ev.id] ??= { on: !("off" in ev && ev.off), text: "" });
      if (!m.text.trim()) m.text = ev.def;
    }
    return x;
  }

  let s = $state<Settings>(withChatRows(structuredClone($state.snapshot(app.data!.settings)) as Settings));
  let mapInfo = $state<MappingInfo | null>(null);
  /** The avatar you're wearing (for "this avatar can't show …" hints). */
  const worn = $derived(app.data?.currentAvatarId ? app.data.avatars[app.data.currentAvatarId] : null);
  /** The worn avatar has been checked and lacks this parameter. */
  const lacks = (key: string) => hasParam(mapInfo?.mapping, worn, key) === false;

  function setSync(name: string, value: number) {
    if (!Number.isFinite(value)) return;
    app.data!.syncValues = { ...app.data!.syncValues, [name]: value };
    api.setSyncValue(name, value).catch(errorToast);
  }
  let names = $state<Record<string, string>>({});
  type SyncRow = MappingInfo["mapping"]["sync"][number];
  let syncList = $state<SyncRow[]>([]);
  let timer: ReturnType<typeof setTimeout> | null = null;

  async function loadMapping() {
    try {
      mapInfo = await api.getMapping();
      names = Object.fromEntries(Object.entries(mapInfo.mapping.parameters).map(([k, v]) => [k, v.name]));
      syncList = structuredClone($state.snapshot(mapInfo.mapping.sync ?? []));
    } catch (e) {
      errorToast(e);
    }
  }
  onMount(loadMapping);

  let updating = $state<"" | "checking" | "installing">("");
  async function checkNow() {
    updating = "checking";
    try {
      if (!(await checkForUpdate())) toast("You're up to date 💕", `Version ${app.version} is the newest.`, "success");
    } catch (e) {
      errorToast(e);
    }
    updating = "";
  }
  async function installNow() {
    const u = app.update;
    if (!u) return;
    const yes = await confirmAsk({ title: `Update to ${u.version}?`, text: "The app closes, updates, and opens again. Your data stays.", ok: "Update" });
    if (!yes) return;
    updating = "installing";
    try {
      await api.installUpdate();
    } catch (e) {
      errorToast(e);
    }
    updating = "";
  }

  let portError = $state("");
  // A blank or out-of-range port would be rejected by the app, so check it first.
  function commitPorts() {
    const ok = (n: unknown) => Number.isInteger(n) && (n as number) >= 1 && (n as number) <= 65535;
    if (!ok(s.osc.sendPort) || !ok(s.osc.recvPort)) portError = "Ports are whole numbers from 1 to 65535.";
    else if (s.osc.sendPort === s.osc.recvPort) portError = "Send and listen ports can't be the same.";
    else {
      portError = "";
      commit(true);
    }
  }

  function commit(now = false) {
    app.data!.settings = structuredClone($state.snapshot(s)) as Settings;
    if (timer) clearTimeout(timer);
    const run = async () => {
      try {
        const note = await api.saveSettings($state.snapshot(s) as Settings);
        if (note) toast("Start with SteamVR", note, "info", 12000);
      } catch (e) {
        errorToast(e);
        s = structuredClone($state.snapshot(app.data!.settings)) as Settings;
      }
    };
    if (now) run();
    else timer = setTimeout(run, 400);
  }

  let savedKicks = $state<unknown>(null);
  const kicksChanged = $derived(!!mapInfo && JSON.stringify($state.snapshot(mapInfo.mapping.kicks)) !== JSON.stringify(savedKicks));
  $effect(() => {
    if (mapInfo && savedKicks === null) savedKicks = structuredClone($state.snapshot(mapInfo.mapping.kicks));
  });

  const namesChanged = $derived(
    !!mapInfo &&
      (Object.entries(names).some(([k, v]) => (mapInfo!.mapping.parameters[k]?.name ?? "") !== v) ||
        JSON.stringify(syncList) !== JSON.stringify(mapInfo.mapping.sync ?? []) ||
        kicksChanged),
  );

  /** App parameter names (current, backups, and per-avatar ones) that Sync must not use. */
  const protectedNames = $derived.by(() => {
    const out = new Map<string, string>();
    if (!mapInfo) return out;
    for (const [k, def] of Object.entries(mapInfo.mapping.parameters)) {
      const label = PARAM_INFO[k]?.label ?? k;
      for (const n of [names[k] ?? def.name, def.name, ...(def.aliases ?? [])]) if (n?.trim()) out.set(n.trim(), label);
    }
    for (const a of Object.values(app.data?.avatars ?? {})) {
      for (const [k, n] of Object.entries(a.paramNames ?? {})) if (n?.trim()) out.set(n.trim(), `${PARAM_INFO[k]?.label ?? k} on ${a.name ?? "an avatar"}`);
    }
    return out;
  });

  function clash(row: SyncRow): string | null {
    const label = protectedNames.get(row.name.trim());
    return label ? `This is an app parameter (${label}). The app already handles it, so it can't be synced.` : null;
  }

  const anyClash = $derived(syncList.some((r) => clash(r)));

  function addSync() {
    syncList.push({ name: "Pregnancy/", type: "bool" });
  }

  function removeSync(i: number) {
    syncList.splice(i, 1);
  }

  /** What the app last saw in-game for a sync parameter. */
  function remembered(row: SyncRow): string {
    const v = app.data?.syncValues?.[row.name];
    if (v == null) return "not seen yet";
    if (row.type === "bool") return v > 0.5 ? "on" : "off";
    return row.type === "int" ? String(Math.round(v)) : v.toFixed(2);
  }

  async function saveNames() {
    if (!mapInfo) return;
    if (anyClash) {
      toast("Can't save yet", "A Sync entry uses an app parameter's name. Remove or rename it.", "error");
      return;
    }
    if (!names.stage?.trim()) {
      toast("Belly size needs a name", "It's the one parameter the app can't work without.", "error");
      return;
    }
    const m = $state.snapshot(mapInfo.mapping) as MappingInfo["mapping"];
    for (const [k, v] of Object.entries(names)) if (m.parameters[k]) m.parameters[k].name = v.trim();
    // Keep built-in rows (an empty name goes back to the default), then your own with a
    // real name, and no duplicates.
    const rows = ($state.snapshot(syncList) as SyncRow[]).map((x) => ({ ...x, name: x.name.trim() }));
    const seen = new Set(rows.filter((x) => x.role).map((x) => x.name));
    m.sync = rows.filter((x) => x.role || (x.name && x.name !== "Pregnancy/" && !seen.has(x.name) && seen.add(x.name)));
    try {
      await api.saveMapping(m);
      savedKicks = null;
      await loadMapping();
      toast("Parameters saved", "The link to VRChat restarts with the new names.", "success");
    } catch (e) {
      errorToast(e);
    }
  }

  async function resetNames() {
    if (await confirmAsk({ title: "Reset parameters?", text: "App parameter names and the sync list go back to the defaults. Per-avatar names stay.", ok: "Reset" })) {
      await api.resetMapping();
      await loadMapping();
    }
  }

  const RESETS: { kind: ResetKind; emoji: string; label: string; text: string }[] = [
    { kind: "avatars", emoji: "🎭", label: "Avatars", text: "Your avatar list and each avatar's settings, links and sockets are removed. Avatars come back when you wear them." },
    { kind: "pregnancies", emoji: "🤰", label: "Pregnancies", text: "Every pregnancy, waiting period, reveal, journal entry and keepsake is removed." },
    { kind: "partners", emoji: "💞", label: "Partners & WooHoos", text: "Your WooHoo history, who you've been with, and remembered species are removed. Pregnancies keep their partner." },
    { kind: "achievements", emoji: "🏆", label: "Achievements", text: "All achievements lock again. Ones you still qualify for unlock again in a moment." },
    { kind: "settings", emoji: "⚙️", label: "Settings", text: "Every setting goes back to its default. Your data stays." },
    { kind: "everything", emoji: "🧨", label: "Everything", text: "The whole app goes back to how it was when installed: data, settings and parameter names." },
  ];

  async function reset(r: (typeof RESETS)[number]) {
    const ok = await confirmAsk({ title: r.kind === "everything" ? "Factory reset?" : `Reset ${r.label.toLowerCase()}?`, text: r.text, ok: "Reset", danger: true });
    if (!ok) return;
    try {
      await api.resetData(r.kind);
      toast(r.kind === "everything" ? "Fresh start 🌱" : `${r.label} reset`, undefined, "success");
    } catch (e) {
      errorToast(e);
    }
  }

  async function newCycle() {
    if (await confirmAsk({ title: "Start a new cycle today?", text: "Today becomes day 1 of your cycle.", ok: "Start new cycle" })) {
      await api.restartCycle();
      toast("New cycle started 🌸", undefined, "success");
    }
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
      if (await saveTextFile(`pregnancy-osc-backup-${new Date().toISOString().slice(0, 10)}.json`, await api.exportBackup())) toast("Backup saved", undefined, "success");
    } catch (e) {
      errorToast(e);
    }
  }

  async function importBackup() {
    const json = await pickTextFile();
    if (!json) return;
    if (!(await confirmAsk({ title: "Restore this backup?", text: "Everything is replaced with the backup. A copy of what you have now is kept in the backups folder.", ok: "Restore", danger: true }))) return;
    try {
      await api.importBackup(json);
      s = structuredClone($state.snapshot(app.data!.settings)) as Settings;
      toast("Backup restored", undefined, "success");
    } catch (e) {
      errorToast(e);
    }
  }

  async function quit() {
    if (await confirmAsk({ title: "Quit Pregnancy OSC?", text: "Your avatar's belly stays where it is and its own counter carries on.", ok: "Quit" })) api.quitApp();
  }

  const PEAKS = [
    { v: 0.3, label: "Realistic", hint: "about 30%" },
    { v: 0.6, label: "Playful", hint: "about 60%" },
    { v: 0.9, label: "Very likely", hint: "about 90%" },
  ];
  const status = $derived(app.status);
  // Some Linux desktops have no tray (the browser preview pretends there is one).
  const trayOk = $derived(!inTauri || (status?.trayAvailable ?? true));

  // What the status line looks like now (or with example values when not pregnant).
  const statusPreview = $derived.by(() => {
    const p = app.data ? pregnancyFor(app.data, app.data.currentAvatarId) : undefined;
    const wk = p ? Math.floor(weekOf(p.timeline, app.now)) : 12;
    const partner = p ? p.partner.name : "Alex";
    const text = s.chatbox.statusText.trim() || "🤰 Week {week} of 40";
    let out = text.replaceAll("{week}", String(wk)).replaceAll("{partner}", partner ?? "Unknown").replaceAll("{name}", p?.name ?? "").replaceAll("{babies}", "?");
    if (s.chatbox.statusPartner && !text.includes("{partner}")) out += ` · 💕 ${partner ?? "Unknown"}`;
    return out.trim();
  });

</script>

{#snippet param(k: string, feature: string | null)}
  {@const def = mapInfo!.mapping.parameters[k]}
  <div class="param">
    <div class="ptop">
      <strong>{PARAM_INFO[k]?.label ?? k}</strong>
      {#if feature}<span class="muted small">for {feature}</span>{/if}
      <span class="grow"></span>
      <span class="pill">{FIXED.includes(k) ? "fixed · " : ""}{def.type}{def.synced ? " · synced" : " · local"}</span>
    </div>
    <span class="muted small">{PARAM_INFO[k]?.text ?? ""}</span>
    {#if FIXED.includes(k)}
      <code class="fixed">{def.name}</code>
    {:else}
      <input bind:value={names[k]} placeholder="(off)" aria-label="{PARAM_INFO[k]?.label ?? k} name" />
    {/if}
  </div>
{/snippet}

{#snippet cantShow(what: string, param: string)}
  <span class="small cant">⚠️ The avatar you're wearing can't show {what} (no {param} parameter). <button class="link-inline" onclick={() => (app.view = "avatars")}>Check it</button></span>
{/snippet}

<div class="page">
  <h3 class="cat">In VRChat</h3>
  <Section id="vrchat" title="🔗 VRChat" open>
    {#snippet extra()}{#if status}<span class="pill" class:ok={status.oscActive} class:off={!status.oscActive}>{status.oscActive ? "linked" : "not linked"}</span>{/if}{/snippet}
    <Toggle bind:checked={s.osc.enabled} label="Control my avatar's belly" hint="Overrides the avatar's own counter while the app runs" onchange={() => commit(true)} />
    <div class="field">
      <strong>Connection</strong>
      <div class="segmented">
        <button class:on={s.osc.useOscquery} onclick={() => { s.osc.useOscquery = true; commit(true); }}>Find automatically</button>
        <button class:on={!s.osc.useOscquery} onclick={() => { s.osc.useOscquery = false; commit(true); }}>Fixed ports</button>
      </div>
      <span class="hint">{s.osc.useOscquery ? "Recommended. Finds VRChat by itself." : "Only if you use an OSC router."}</span>
    </div>
    {#if !s.osc.useOscquery}
      <div class="row">
        <label class="field grow">Send port<input type="number" min="1" max="65535" bind:value={s.osc.sendPort} onchange={commitPorts} /></label>
        <label class="field grow">Listen port<input type="number" min="1" max="65535" bind:value={s.osc.recvPort} onchange={commitPorts} /></label>
      </div>
      {#if portError}<span class="small err">{portError}</span>{/if}
    {/if}
    {#if status}
      <LinkInfo {status} />
      {#if status.note}<span class="muted small">{status.note}</span>{/if}
    {/if}
  </Section>

  {#if mapInfo?.mapping.sync.length}
    <Section id="follow" title="🔁 Follow you">
      <p class="muted small">Your avatar's own toggles, carried onto every avatar you wear. Change them here or in-game.</p>
      {#each mapInfo.mapping.sync as sp (sp.name)}
        {@const v = app.data?.syncValues?.[sp.name]}
        {@const missing = worn?.params.length ? !worn.params.includes(sp.name) : false}
        {#if sp.type === "bool"}
          <Toggle checked={(v ?? 0) > 0.5} label={syncLabel(sp)} hint={missing ? "Not on the avatar you're wearing" : ""} onchange={(on) => setSync(sp.name, on ? 1 : 0)} />
        {:else}
          <label class="field">{syncLabel(sp)}
            <input type="number" step={sp.type === "int" ? 1 : 0.01} value={v ?? ""} placeholder="not set yet"
              onchange={(e) => { const n = (e.target as HTMLInputElement).valueAsNumber; setSync(sp.name, sp.type === "int" ? Math.round(n) : n); }} />
            {#if missing}<span class="hint">Not on the avatar you're wearing</span>{/if}
          </label>
        {/if}
      {/each}
      <span class="hint">Add or rename these under Advanced > Follow you (Sync).</span>
    </Section>
  {/if}

  <Section id="chatbox" title="💬 Chatbox">
    {#snippet extra()}<span class="pill" class:ok={s.chatbox.enabled} class:off={!s.chatbox.enabled}>{s.chatbox.enabled ? "on" : "off"}</span>{/snippet}
    <Toggle bind:checked={s.chatbox.enabled} label="Use my chatbox" hint="Everyone near you can see it. Other chatbox apps may take turns with it." onchange={() => commit()} />
    {#if s.chatbox.enabled}
      <h3 class="sub">📊 Status</h3>
      <Toggle bind:checked={s.chatbox.status} label="Always show pregnancy status" hint="While you're pregnant" onchange={() => commit()} />
      {#if s.chatbox.status}
        <label class="field">Status text
          <input bind:value={s.chatbox.statusText} maxlength="144" placeholder="🤰 Week {'{week}'} of 40" oninput={() => commit()} />
        </label>
        <Toggle bind:checked={s.chatbox.statusPartner} label="Show partner in status" onchange={() => commit()} />
        <div class="chat-preview" aria-label="Preview">{statusPreview}</div>
      {/if}

      <h3 class="sub">🔔 Event messages</h3>
      <div class="field">
        <strong>Show each message for</strong>
        <div class="segmented">
          {#each SHOW_FOR as [v, l]}
            <button class:on={s.chatbox.showSeconds === v} onclick={() => { s.chatbox.showSeconds = v; commit(); }}>{l}</button>
          {/each}
        </div>
      </div>
      <Toggle bind:checked={s.chatbox.sound} label="Play the chatbox sound" hint="For event messages only" onchange={() => commit()} />
      <div class="chat-events">
        {#each CHAT_EVENTS as ev (ev.id)}
          {@const m = s.chatbox.messages[ev.id]}
          <label class="chat-ev" class:off={!m.on}>
            <input type="checkbox" bind:checked={m.on} onchange={() => commit()} />
            <span>{ev.label}</span>
          </label>
          <input class="chat-text" bind:value={m.text} disabled={!m.on} maxlength="144" placeholder={ev.def} aria-label="{ev.label} message" oninput={() => commit()} onblur={() => { if (!m.text.trim()) { m.text = ev.def; commit(); } }} />
        {/each}
      </div>
      <span class="muted small">Use {"{week}"}, {"{babies}"}, {"{partner}"}, {"{name}"} or {"{achievement}"} in any text. Clear one to get its default back.</span>
    {/if}
  </Section>

  <Section id="woohoos" title="💕 Detecting WooHoos">
    {#snippet extra()}<span class="pill" class:ok={s.sps.enabled} class:off={!s.sps.enabled}>{s.sps.enabled ? "on" : "off"}</span>{/snippet}
    <Toggle bind:checked={s.sps.enabled} label="Use SPS sockets" hint="Something from another player inside a socket for long enough (like OSC Goes Brrr reads)" onchange={() => commit()} />
    {#if s.sps.enabled}
      <div class="field">
        <strong>Hole types that count</strong>
        <span class="hint">For every avatar. Sockets get their type from their name.</span>
        <div class="choices two">
          {#each HOLE_TYPES as h}
            <button class="opt" class:on={s.sps.types[h.id]} onclick={() => { s.sps.types[h.id] = !s.sps.types[h.id]; commit(); }} aria-pressed={s.sps.types[h.id]}>
              <strong>{h.emoji} {h.label} {s.sps.types[h.id] ? "✓" : ""}</strong><span>{h.hint}</span>
            </button>
          {/each}
        </div>
      </div>
      <div class="soon">
        {#if !status?.spsFound?.length}
          <span class="muted small">{status?.vrchatRunning ? "No SPS sockets found on the avatar you're wearing. Sockets need VRCFury haptics turned on." : "Open VRChat to find the sockets on your avatar."}</span>
        {:else}
          <strong class="small">Sockets on the avatar you're wearing</strong>
          {#each status.spsLive as live (live.name)}
            {@const ht = HOLE_TYPES.find((h) => h.id === live.holeType)}
            <div class="socket">
              <span class="row grow small">
                <span>{live.name}</span>
                <span class="pill" class:ok={live.counts} class:off={!live.counts} title={live.counts ? "Counts" : "Doesn't count"}>{ht?.emoji} {ht?.label}</span>
              </span>
              <div class="meter" title="How deep right now"><div style="width:{Math.round(live.value * 100)}%" class:over={live.value >= s.sps.depth && live.counts}></div><span class="line" style="left:{s.sps.depth * 100}%"></span></div>
            </div>
          {/each}
          <span class="muted small">Change a socket's type, or pick sockets for one avatar, on its page under Avatars.</span>
          {#if (status.spsProgress ?? 0) > 0}
            <span class="small">💕 {status.spsProgress >= 1 ? "Counted as a WooHoo this session" : `${Math.round(status.spsProgress * 100)}% of the way to a WooHoo`}</span>
          {/if}
        {/if}
      </div>
      <label class="field">How deep counts: {Math.round(s.sps.depth * 100)}%
        <input type="range" min="0.05" max="0.95" step="0.05" bind:value={s.sps.depth} oninput={() => commit()} />
        <span class="hint">The line on each meter.</span>
      </label>
      <label class="field">How long: {s.sps.seconds} seconds in total
        <input type="range" min="5" max="180" step="5" bind:value={s.sps.seconds} oninput={() => commit()} />
        <span class="hint">Breaks count too, as long as the session keeps going. Your own touches never count.</span>
      </label>
      <label class="field">Session ends after {Math.round(s.sps.gapSeconds / 60 * 10) / 10} min of quiet
        <input type="range" min="30" max="900" step="30" bind:value={s.sps.gapSeconds} oninput={() => commit()} />
        <span class="hint">Only one WooHoo per session.</span>
      </label>
    {/if}
  </Section>

  <Section id="partners" title="🔍 Logs and pictures">
    <Toggle bind:checked={s.readLogs} label="Read VRChat logs" hint="To guess who you were with and name your avatars. On this computer only, never uploaded." onchange={() => commit(true)} />
    {#if s.readLogs}
      <div class="field">
        <strong>Log folder</strong>
        <div class="segmented">
          <button class:on={!s.logDir} onclick={() => { if (s.logDir) { s.logDir = null; commit(true); } }}>Find automatically</button>
          <button class:on={!!s.logDir} disabled={!inTauri} onclick={chooseLogDir}>Choose folder…</button>
        </div>
        {#if s.logDir}
          <span class="hint path">{s.logDir}</span>
        {:else if status?.logDir}
          <span class="hint path">Found: {status.logDir}</span>
        {:else}
          <span class="hint">Not found yet. Start VRChat once, or choose the folder.</span>
        {/if}
      </div>
    {/if}
    <Toggle bind:checked={s.showPictures} label="Show VRChat pictures" hint="Profile pictures and avatar thumbnails from public vrchat.com pages. No login needed." onchange={() => commit()} />
  </Section>

  <h3 class="cat">Pregnancy</h3>
  <Section id="fertility" title="🌸 Fertility">
    {#snippet extra()}<span class="pill" class:ok={s.fertility} class:off={!s.fertility}>{s.fertility ? "cycle on" : "off"}</span>{/snippet}
    <Toggle bind:checked={s.fertility} label="Fertility cycle and grace period" hint="Off: a pregnancy starts the moment you WooHoo" onchange={() => commit()} />
    {#if s.fertility}
      <div class="field">
        <strong>Chance on your most fertile day</strong>
        <div class="choices three">
          {#each PEAKS as pk}
            <button class="opt" class:on={Math.abs(s.fertilityPeak - pk.v) < 0.01} onclick={() => { s.fertilityPeak = pk.v; commit(); }}>
              <strong>{pk.label}</strong><span>{pk.hint}</span>
            </button>
          {/each}
        </div>
      </div>
      <button class="small soft" style="align-self:flex-start" onclick={newCycle}>Start a new cycle today</button>
    {/if}
  </Section>

  <Section id="babies" title="👶 Babies">
    <div class="field">
      <strong>Twins and more</strong>
      <div class="choices three">
        {#each Object.entries(MULTIPLES_LABEL) as [k, m]}
          <button class="opt" class:on={s.multiples === k} onclick={() => { s.multiples = k as Multiples; commit(); }}>
            <strong>{m.name}</strong><span>{m.hint}</span>
          </button>
        {/each}
      </div>
    </div>
    <div class="field">
      <strong>Genders</strong>
      <div class="choices two">
        <button class="opt" class:on={s.genderMode === "random"} onclick={() => { s.genderMode = "random"; commit(); }}>
          <strong>Random</strong><span>Each baby is a girl or a boy, revealed at week 20</span>
        </button>
        <button class="opt" class:on={s.genderMode === "off"} onclick={() => { s.genderMode = "off"; commit(); }}>
          <strong>Off</strong><span>No genders shown</span>
        </button>
      </div>
    </div>
  </Section>

  <Section id="belly" title="🫃 Belly">
    <Toggle bind:checked={s.biggerForMultiples} label="Bigger for twins and more" hint="Twins a little bigger, triplets more" onchange={() => commit()} />
    <Toggle bind:checked={s.growWhenOverdue} label="Keep growing when overdue" hint="The belly grows a little more after the due date" onchange={() => commit()} />
    <span class="muted small">Both need Bigger Belly from the Pregnancy System SDK (<code>Pregnancy/BellyExtra</code>).</span>
    {#if lacks("bellyExtra")}{@render cantShow("a bigger belly", "Extra size")}{/if}
  </Section>

  <Section id="movement" title="💓 Kicks">
    {#snippet extra()}<span class="pill" class:ok={s.kicks} class:off={!s.kicks}>{s.kicks ? s.kickFrequency : "off"}</span>{/snippet}
    <Toggle bind:checked={s.kicks} label="Kicks" hint="The app sends kicks to your avatar from week 16, even in the tray" onchange={() => commit()} />
    {#if lacks("fetalMovement")}{@render cantShow("kicks", "Fetal movement")}{/if}
    {#if s.kicks}
      <div class="field">
        <strong>How often</strong>
        <div class="choices three">
          {#each [["calm", "Calm", "every 8 to 20 min"], ["normal", "Normal", "every 3 to 10 min"], ["active", "Active", "every 1 to 4 min"]] as [k, l, h]}
            <button class="opt" class:on={s.kickFrequency === k} onclick={() => { s.kickFrequency = k as Settings["kickFrequency"]; commit(); }}>
              <strong>{l}</strong><span>{h}</span>
            </button>
          {/each}
        </div>
        <span class="hint">Twins and more kick more often.</span>
      </div>
      <Toggle bind:checked={s.kicksWithPartner} disabled={!s.readLogs} label="More active with your partner here" hint={s.readLogs ? "Kicks come twice as often (and stronger) while the partner is in the instance" : "Needs log reading (Logs and pictures)"} onchange={() => commit()} />
      <div class="field">
        <strong>Kinds of kicks</strong>
        <Toggle bind:checked={s.allowedKicks.weak} label="Little kicks" hint="From week 16" onchange={() => commit()} />
        <Toggle bind:checked={s.allowedKicks.strong} label="Strong kicks" hint="From week 24" onchange={() => commit()} />
        <Toggle bind:checked={s.allowedKicks.extreme} label="Big kicks" hint="From week 32" onchange={() => commit()} />
      </div>
    {/if}
  </Section>

  <Section id="labor" title="🍼 Labor and birth">
    {#snippet extra()}<span class="pill">{LABOR_STARTS.find((x) => x.k === s.labor.start)?.l}</span>{/snippet}
    <div class="field">
      <strong>Labor starts</strong>
      <div class="choices three">
        {#each LABOR_STARTS as o}
          <button class="opt" class:on={s.labor.start === o.k} onclick={() => { s.labor.start = o.k; commit(); }}>
            <strong>{o.l}</strong><span>{o.h}</span>
          </button>
        {/each}
      </div>
    </div>
    <div class="field">
      <strong>Induce at</strong>
      <div class="segmented">
        {#each [[null, "Off"], [41, "Week 41"], [42, "Week 42"], [43, "Week 43"]] as [w, l]}
          <button class:on={s.labor.induceWeek === w} onclick={() => { s.labor.induceWeek = w as number | null; commit(); }}>{l}</button>
        {/each}
      </div>
      <span class="hint">Starts labor by then if it hasn't started on its own.</span>
    </div>
    <div class="field">
      <strong>Labor length</strong>
      <div class="segmented">
        {#each [10, 20, 30, 60] as m}
          <button class:on={s.labor.minutes === m} onclick={() => { s.labor.minutes = m; commit(); }}>{m} min</button>
        {/each}
      </div>
      <span class="hint">Real minutes, whatever the pregnancy length. Contractions get closer, the water breaks, then the baby comes.</span>
    </div>
    <div class="field">
      <strong>Contractions start</strong>
      <div class="segmented">
        <button class:on={!s.labor.quietEarly} onclick={() => { s.labor.quietEarly = false; commit(); }}>In early labor</button>
        <button class:on={!!s.labor.quietEarly} onclick={() => { s.labor.quietEarly = true; commit(); }}>After the water breaks</button>
      </div>
      <span class="hint">{s.labor.quietEarly ? "Early labor shows nothing on your avatar (kicks carry on), so the water breaking is the first sign." : "Contractions show from the start of labor, getting closer until the water breaks."}</span>
    </div>
    <Toggle bind:checked={s.labor.restingHolds} label="Resting holds off labor" hint="Labor only starts, and the water only breaks, while you're standing. Sitting or lying down holds it off (Induce at still starts it)." onchange={() => commit()} />
    <Toggle bind:checked={s.labor.woohooStarts} label="A WooHoo can start labor" hint="From week 37: a 25% chance, rising to 75% from week 40" onchange={() => commit()} />
    <Toggle bind:checked={s.labor.waitsForPartner} disabled={!s.readLogs} label="Labor waits for your partner" hint={s.readLogs ? "Labor doesn't start until the partner is in the instance. Once it starts, it keeps going." : "Needs log reading (Logs and pictures)"} onchange={() => commit()} />
    <span class="muted small">Labor only moves while you're in VRChat on an avatar with <code>Pregnancy/Labor</code>. On other avatars, "Welcome your baby" skips straight to the birth. Holding Start in the avatar's menu (from week 37) starts labor too.</span>
    {#if lacks("labor")}{@render cantShow("labor", "Labor")}{/if}
  </Section>

  <h3 class="cat">App</h3>
  <Section id="steamvr" title="🥽 SteamVR">
    {#snippet extra()}{#if status}<span class="pill" class:ok={status.steamvrRunning} class:off={!status.steamvrRunning}>{status.steamvrRunning ? "running" : "not running"}</span>{/if}{/snippet}
    <Toggle bind:checked={s.startWithSteamvr} label="Start with SteamVR" onchange={() => commit(true)} />
    <Toggle bind:checked={s.steamvrDashboard} label="Show in the SteamVR dashboard" hint="The app gets its own tab in the dashboard" onchange={() => commit()} />
    {#if s.steamvrDashboard}
      <Toggle bind:checked={s.hideWindowInVr} label="Hide the window while in VR" hint="Open it from the dashboard tab or the tray" onchange={() => commit()} />
    {/if}
    <div class="field">
      <strong>When SteamVR closes</strong>
      <div class="segmented">
        <button class:on={!s.exitWithSteamvr} onclick={() => { s.exitWithSteamvr = false; commit(); }}>Keep running</button>
        <button class:on={s.exitWithSteamvr} onclick={() => { s.exitWithSteamvr = true; commit(); }}>Quit the app</button>
      </div>
    </div>
    {#if status}
      <span class="hint">
        {#if !status.steamvrRunning}SteamVR isn't running.{:else if s.steamvrDashboard}SteamVR is running · {status.steamvrDashboard ? "In the dashboard ✓" : "Joining the dashboard…"}{:else}SteamVR is running.{/if}
      </span>
    {/if}
  </Section>

  <Section id="startup" title="🚀 Starting and closing">
    <Toggle bind:checked={s.startWithOs} label="Start when I log in" onchange={() => commit(true)} />
    <div class="field">
      <strong>When it starts by itself</strong>
      <div class="segmented">
        <button class:on={!s.startInTray} onclick={() => { s.startInTray = false; commit(); }}>Open the window</button>
        <button class:on={s.startInTray} disabled={!trayOk} onclick={() => { s.startInTray = true; commit(); }}>Stay in the tray</button>
      </div>
      <span class="hint">At log-in or with SteamVR. Opening it yourself always shows the window.</span>
    </div>
    <div class="field">
      <strong>Closing the window</strong>
      <div class="segmented">
        <button class:on={s.closeToTray} disabled={!trayOk} onclick={() => { s.closeToTray = true; commit(); }}>Keep running in the tray</button>
        <button class:on={!s.closeToTray} onclick={() => { s.closeToTray = false; commit(); }}>Quit the app</button>
      </div>
      {#if !trayOk}<span class="hint">There's no tray on this desktop, so closing quits.</span>{/if}
    </div>
  </Section>

  <Section id="look" title="🎨 Look">
    <div class="field">
      <strong>Theme</strong>
      <div class="segmented">
        {#each [["system", "Follow system"], ["light", "Light"], ["dark", "Dark"], ["black", "Black"]] as [m, l]}
          <button class:on={s.theme.mode === m} onclick={() => { s.theme.mode = m as Settings["theme"]["mode"]; commit(); }}>{l}</button>
        {/each}
      </div>
      {#if s.theme.mode === "black"}<span class="hint">True black, for OLED screens.</span>{/if}
    </div>
    <div class="field">
      <strong>Color</strong>
      <div class="swatches">
        {#each PALETTES as pl}
          {@const on = !s.theme.customAccent && s.theme.palette === pl.id}
          <button class="swatch" class:on style="--sw:{pl.color}" aria-pressed={on} onclick={() => { s.theme.palette = pl.id; s.theme.customAccent = false; commit(); }}>
            <span class="dot">{#if on}✓{/if}</span>{pl.label}
          </button>
        {/each}
        <button class="swatch custom" class:on={s.theme.customAccent} style="--sw:{s.theme.customAccent ? s.theme.accent : 'transparent'}" aria-expanded={pickingColor} onclick={openCustom}>
          <span class="dot">{#if s.theme.customAccent}✓{/if}</span>Custom
        </button>
      </div>
      {#if pickingColor && s.theme.customAccent}
        <AccentPicker value={s.theme.accent} onchange={(c) => { s.theme.accent = c; commit(); }} />
      {/if}
      <span class="hint">Buttons, toggles and the progress ring.</span>
    </div>
  </Section>

  <Section id="updates" title="✨ Updates">
    {#snippet extra()}{#if app.update}<span class="pill">New!</span>{/if}{/snippet}
    <span class="muted small">You have version {app.version || "?"}.</span>
    {#if app.update}
      <strong class="small">Version {app.update.version} is ready!</strong>
      {#if app.update.notes}<p class="muted small notes">{app.update.notes}</p>{/if}
      <div class="row">
        <button class="small" disabled={!!updating} onclick={installNow}>{updating === "installing" ? "Updating…" : "Update now"}</button>
      </div>
    {:else}
      <div class="row">
        <button class="soft small" disabled={!!updating} onclick={checkNow}>{updating === "checking" ? "Checking…" : "Check for updates"}</button>
      </div>
    {/if}
  </Section>

  <Section id="backup" title="🗂 Backup and reset">
    <strong class="small">Backup</strong>
    <div class="row">
      <button class="soft small" onclick={exportBackup}>Export</button>
      <button class="soft small" onclick={importBackup}>Import</button>
    </div>
    <strong class="small">Reset</strong>
    <span class="hint">A copy of your data is saved first, just in case.</span>
    <div class="resets">
      {#each RESETS as r}
        <button class="soft small" class:danger={r.kind === "everything"} onclick={() => reset(r)}>{r.emoji} {r.label}</button>
      {/each}
    </div>
  </Section>

  <Section id="advanced" title="🛠 Advanced">
    {#snippet extra()}{#if mapInfo}<span class="pill">{mapInfo.syncedBitsAll} synced bits</span>{/if}{/snippet}
    <p class="muted small">Only needed if your avatar uses its own parameter names, or you want other parameters to follow you between avatars.</p>


    <p class="muted small">What the app drives or reads on your avatar. Leave a name empty to turn one off. If one avatar uses different names, set them on its page under Avatars.</p>
    {#if mapInfo}
      <h3 class="sub">✅ Required</h3>
      <p class="muted small">The app can't work on an avatar without these.</p>
      <div class="params">
        {#each REQUIRED.filter((k) => mapInfo!.mapping.parameters[k]) as k}{@render param(k, null)}{/each}
      </div>
      <h3 class="sub">✨ Features</h3>
      <p class="muted small">Optional. Each one turns something on; without it, that feature just doesn't show. The menu buttons are made for this app, so their names are fixed.</p>
      <div class="params">
        {#each FEATURES as f}
          {#each f.keys.filter((k) => mapInfo!.mapping.parameters[k]) as k}{@render param(k, PARAM_INFO[k]?.label === f.label ? null : f.label)}{/each}
        {/each}
      </div>
    {/if}

    <h3 class="sub">🔁 Follow you (Sync)</h3>
    <p class="muted small">Your own avatar's parameters that follow you onto every avatar. The app remembers the last value it sees in-game (or that you set in Settings > Follow you) and sends it again whenever you switch avatars. Hide belly and Fetal sounds are built in: rename them if your avatar uses other names, but they can't be removed. Add anything else you like.</p>
    <div class="params">
      {#each syncList as row, i}
        {@const why = clash(row)}
        <div class="syncrow" class:bad={!!why}>
          {#if row.role}
            <span class="builtin"><strong>{SYNC_ROLES[row.role]}</strong> <span class="pill">built-in · bool</span></span>
            <input class="grow" bind:value={row.name} placeholder={row.role === "hideBelly" ? "Pregnancy/HideBelly" : "Pregnancy/FetalSounds"} aria-label="{SYNC_ROLES[row.role]} parameter name" />
          {:else}
            <input class="grow" bind:value={row.name} placeholder="Pregnancy/Something" aria-label="Parameter name" />
            <select bind:value={row.type} aria-label="Type">
              <option value="bool">bool</option>
              <option value="int">int</option>
              <option value="float">float</option>
            </select>
            <button class="icon soft" onclick={() => removeSync(i)} aria-label="Remove {row.name}">✕</button>
          {/if}
          {#if why}
            <span class="small remembered warn-text">⛔ {why}</span>
          {:else}
            <span class="muted small remembered">Remembered: {remembered(row)}</span>
          {/if}
        </div>
      {:else}
        <p class="muted small">Nothing synced yet.</p>
      {/each}
    </div>
    <button class="small soft" style="align-self:flex-start" onclick={addSync}>+ Add a parameter</button>
    <div class="field">
      <strong>Wait before remembering a change</strong>
      <span class="hint">Switching avatars in this time drops the change (it's the new avatar's own value). With VRChat log reading on, switches are also caught from the log.</span>
      <div class="segmented">
        {#each [[1000, "1 s"], [2000, "2 s"], [3000, "3 s"], [5000, "5 s"]] as [ms, l]}
          <button class:on={s.syncWaitMs === ms} onclick={() => { s.syncWaitMs = ms as number; commit(); }}>{l}</button>
        {/each}
      </div>
    </div>

    {#if mapInfo}
      <h3 class="sub">💓 Kick values</h3>
      <p class="muted small">Sent to Fetal movement. The SDK plays a different kick for 1, 2 and 3, and goes back to 0 after the hold time. During labor, the contraction value is held for 30 s instead. If your avatar also makes its own random kicks, have its kick layer pause while <code>Pregnancy/AppActive</code> is on.</p>
      <div class="kickvals">
        <label class="field">Little<input type="number" bind:value={mapInfo.mapping.kicks.weak} /></label>
        <label class="field">Strong<input type="number" bind:value={mapInfo.mapping.kicks.strong} /></label>
        <label class="field">Big<input type="number" bind:value={mapInfo.mapping.kicks.extreme} /></label>
        <label class="field">Hold (ms)<input type="number" min="100" step="100" bind:value={mapInfo.mapping.kicks.holdMs} /></label>
        <label class="field">Contraction<input type="number" bind:value={mapInfo.mapping.kicks.contraction} /></label>
      </div>

      <div class="row">
        {#if namesChanged}<button onclick={saveNames} disabled={anyClash}>Save changes</button>{/if}
        <span class="grow"></span>
        <button class="soft small" onclick={resetNames}>Reset to defaults</button>
        {#if inTauri}<button class="ghost small" onclick={() => api.openPath(mapInfo!.path)}>Open the file</button>{/if}
      </div>
    {/if}
  </Section>

  {#if inTauri}
    <div class="row quit">
      <button class="danger small" onclick={quit}>Quit app</button>
    </div>
  {:else if !s.debug}
    <p class="muted small center">Preview tip: tap the title 5 times for the debug tools.</p>
  {/if}

  {#if app.data?.settings.debug}
    <DebugPanel />
  {/if}
</div>

<style>
  .chat-preview {
    padding: 8px 12px;
    border-radius: 12px;
    background: var(--bg-3);
    font-weight: 600;
    font-size: 0.9rem;
    align-self: flex-start;
    max-width: 100%;
    overflow-wrap: anywhere;
  }
  .chat-events {
    display: grid;
    grid-template-columns: minmax(130px, auto) 1fr;
    gap: 6px 10px;
    align-items: center;
  }
  .chat-ev {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 700;
    font-size: 0.9rem;
    cursor: pointer;
  }
  .chat-ev input {
    accent-color: var(--accent);
    width: 16px;
    height: 16px;
    margin: 0;
  }
  .chat-ev.off {
    color: var(--muted);
  }
  .chat-text:disabled {
    opacity: 0.45;
  }
  @media (max-width: 420px) {
    .chat-events {
      grid-template-columns: 1fr;
    }
  }
  .err {
    color: var(--danger);
    font-weight: 700;
  }
  .page {
    max-width: 560px;
    margin: 0 auto;
    padding: 6px 16px 40px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .soon {
    display: flex;
    flex-direction: column;
    background: var(--bg-3);
    border-radius: 14px;
    padding: 10px 12px;
  }
  .socket {
    display: grid;
    grid-template-columns: 1fr 1.4fr;
    gap: 10px;
    align-items: center;
    padding: 4px 0;
  }
  .meter {
    position: relative;
    height: 10px;
    background: var(--bg-2);
    border-radius: 99px;
    overflow: hidden;
  }
  .meter div {
    height: 100%;
    background: var(--muted);
    transition: width 0.2s;
  }
  .meter div.over {
    background: var(--accent);
  }
  .meter .line {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    background: var(--text);
    opacity: 0.6;
  }
  .choices {
    display: grid;
    gap: 8px;
  }
  .choices.three {
    grid-template-columns: repeat(3, 1fr);
  }
  .choices.two {
    grid-template-columns: repeat(2, 1fr);
  }
  .opt {
    flex-direction: column;
    align-items: flex-start;
    text-align: left;
    white-space: normal;
    gap: 1px;
    background: var(--bg);
    color: var(--text);
    border: 2px solid var(--line);
    border-radius: 14px;
    padding: 9px 10px;
  }
  .opt span {
    font-weight: 600;
    font-size: 0.75rem;
    color: var(--muted);
  }
  .opt.on {
    border-color: var(--accent);
    background: var(--bg-3);
  }
  .params {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 4px 0 8px;
  }
  .param {
    display: flex;
    flex-direction: column;
    gap: 3px;
    background: var(--bg);
    border-radius: 14px;
    padding: 10px 12px;
  }
  .ptop {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }
  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .swatch {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 5px 12px 5px 5px;
    border-radius: 999px;
    border: 1.5px solid transparent;
    background: var(--bg-3);
    color: var(--text);
    font: inherit;
    font-weight: 700;
    font-size: 0.9rem;
    box-shadow: none;
    cursor: pointer;
  }
  .swatch.on {
    border-color: var(--sw);
    background: color-mix(in srgb, var(--sw) 16%, var(--bg-3));
  }
  .dot {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--sw);
    color: #fff;
    font-size: 0.75rem;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.35);
  }
  /* Custom before one is picked: a little rainbow. */
  .custom:not(.on) .dot {
    background: conic-gradient(#f59ac0, #ffad7a, #f7d774, #6fcfac, #7fb8f5, #b39bff, #f59ac0);
  }
  .custom.on {
    border-color: var(--accent);
  }
  .kickvals {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
    gap: 8px;
    margin-bottom: 8px;
  }
  .kickvals input {
    width: 100%;
  }
  code {
    background: var(--bg-3);
    padding: 1px 6px;
    border-radius: 6px;
  }
  .syncrow {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    background: var(--bg);
    border-radius: 14px;
    padding: 8px 10px;
  }
  .syncrow select {
    width: 80px;
  }
  .fixed {
    padding: 8px 10px;
    border-radius: var(--radius);
    background: var(--bg-3);
    font-size: 0.85rem;
  }
  .remembered,
  .builtin {
    flex-basis: 100%;
  }
  .syncrow.bad {
    box-shadow: inset 0 0 0 2px var(--danger);
  }
  .cant {
    padding: 6px 10px;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--warn) 18%, transparent);
  }
  .link-inline {
    background: none;
    padding: 0;
    color: var(--accent);
    font-size: inherit;
    text-decoration: underline;
  }
  .warn-text {
    color: var(--danger);
    font-weight: 700;
  }
  .resets {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 6px;
  }
  .cat {
    margin: 8px 4px -6px;
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
  }
  .cat:first-child {
    margin-top: 0;
  }
  .sub {
    margin: 10px 0 0;
    font-size: 1rem;
  }
  .quit {
    justify-content: center;
  }
  .center {
    text-align: center;
  }
  @media (max-width: 440px) {
    .choices.three,
    .choices.two {
      grid-template-columns: 1fr;
    }
  }
  .notes {
    margin: 0;
    white-space: pre-wrap;
  }
  .path {
    word-break: break-all;
  }
</style>
