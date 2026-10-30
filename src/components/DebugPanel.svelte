<script lang="ts">
  import { app, currentPregnancy, currentPending, updatePregnancy, errorToast, toast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { cycleDay, cycleMs } from "../lib/fertility";
  import { setProgress, progress, TERM_WEEKS } from "../lib/timeline";
  import type { Settings } from "../lib/types";
  import { hipWarning } from "../lib/posture";

  const d = $derived(app.data!);
  const st = $derived(app.status);
  const p = $derived(currentPregnancy());
  const w = $derived(currentPending());
  let day = $state(13);
  let tName = $state("");
  let tId = $state("");
  let pic = $state<{ loading: boolean; src: string | null; error: string | null } | null>(null);
  const tIdOk = $derived(!tId.trim() || /^usr_[0-9a-f-]{36}$/i.test(tId.trim()));
  // People "in" the instance right now (from the log, or added here).
  const here = $derived(st?.others ?? []);

  async function checkPicture() {
    if (!tId.trim() || !tIdOk) return;
    pic = { loading: true, src: null, error: null };
    try {
      const src = await api.debugPicture(tId.trim());
      pic = { loading: false, src, error: src ? null : "No picture came back." };
    } catch (e) {
      pic = { loading: false, src: null, error: e instanceof Error ? e.message : String(e) };
    }
  }

  async function addToInstance() {
    await run(`${tName.trim()} joined the instance`, () => api.debugAddPerson(tName.trim(), tId.trim() || null));
  }

  async function setAsPartner() {
    if (!p) return;
    await run(`${tName.trim()} is the partner`, () =>
      api.setPartner(p!.id, { name: tName.trim(), userId: tId.trim() || null, avatarName: null, lastSeenMs: Date.now() }, null),
    );
  }
  let week = $state(20);

  const nowDay = $derived(d.settings.cycleAnchorMs ? Math.floor(cycleDay(app.now, d.settings.cycleAnchorMs, cycleMs(d.settings))) : null);

  async function run(label: string, f: () => Promise<unknown>) {
    try {
      await f();
      toast("🛠 " + label, undefined, "info", 2500);
    } catch (e) {
      errorToast(e);
    }
  }

  // Late on, a WooHoo while pregnant can start labor (Labor and birth setting).
  const laborWoohoo = $derived(!!p && !p.labor && !!d.settings.labor.woohooStarts && progress(p.timeline, app.now) * TERM_WEEKS >= 37);

  async function woohoo(outcome: boolean | null) {
    if (laborWoohoo) {
      await run(outcome === false ? "WooHoo (no labor)" : outcome ? "WooHoo (labor)" : "WooHoo (maybe labor)", () => api.debugWoohoo(outcome));
      return;
    }
    if (p) {
      toast("Already pregnant", "End or finish this pregnancy first (a WooHoo is ignored while pregnant).", "info");
      return;
    }
    await run(outcome === null ? "WooHoo (random)" : outcome ? "WooHoo (pregnant)" : "WooHoo (not this time)", () => api.debugWoohoo(outcome));
  }

  async function jumpWeek() {
    if (!p) return;
    await updatePregnancy(p.id, (x) => (x.timeline = setProgress(x.timeline, Date.now(), week / TERM_WEEKS)));
    toast(`🛠 Jumped to week ${week}`, undefined, "info", 2500);
  }

  // Testing notes in the VRChat chatbox (keys match DEBUG_EVENTS in pregsys-core/src/chatbox.rs).
  const CHAT_NOTES = [
    { id: "avatarChange", label: "Avatar change detected" },
    { id: "avatarLoaded", label: "Avatar loaded (params found, belly ✓)" },
    { id: "sync", label: "Sync values saved, or ignored while loading" },
    { id: "menuButtons", label: "Menu button presses received" },
    { id: "bellySnap", label: "Belly put back after an in-game change" },
    { id: "heard", label: "Started hearing VRChat" },
    { id: "kicks", label: "Kicks sent" },
  ];

  async function toggleNote(id: string, on: boolean) {
    const s = $state.snapshot(d.settings) as Settings;
    s.chatbox = { ...s.chatbox, debug: { ...s.chatbox.debug, [id]: on } };
    try {
      await api.saveSettings(s);
      app.data!.settings = s;
    } catch (e) {
      errorToast(e);
    }
  }

  async function off() {
    const s: Settings = { ...($state.snapshot(d.settings) as Settings), debug: false };
    await api.saveSettings(s);
    app.data!.settings.debug = false;
    toast("Debug mode off", undefined, "info", 2500);
  }
</script>

<section class="card debug">
  <div class="row">
    <h2 class="grow">🛠 Debug</h2>
    <button class="small ghost" onclick={off}>Turn off</button>
  </div>
  <p class="muted small">Pretend events to see how the system reacts in VRChat. Everything goes through the real system: partner guess, belly, notifications.</p>

  <div class="group">
    <strong>💕 Pretend a WooHoo</strong>
    <span class="muted small">{laborWoohoo ? "Week 37 or later: a WooHoo can start labor here (💕 starts it, 🤍 doesn't)." : p ? "You're pregnant on this avatar, so WooHoos are ignored." : w ? "Waiting already: another WooHoo is a second chance if the first didn't take." : "Uses who's in your instance right now for the partner guess."}</span>
    <div class="row">
      <button class="small" onclick={() => woohoo(null)} disabled={!!p && !laborWoohoo}>🎲 Random</button>
      <button class="small" onclick={() => woohoo(true)} disabled={!!p && !laborWoohoo}>💕 {laborWoohoo ? "Labor" : "Pregnant"}</button>
      <button class="small soft" onclick={() => woohoo(false)} disabled={!!p && !laborWoohoo}>🤍 Not this time</button>
    </div>
  </div>

  <div class="group">
    <strong>🐰 Test Buddy</strong>
    <span class="muted small">A pretend partner in your instance, with a made-up picture, a few past WooHoos and a past baby. Test with them so your real partner stats stay clean.</span>
    <label class="note">
      <input type="checkbox" checked={!!d.testBuddySince} onchange={(e) => run(e.currentTarget.checked ? "Test Buddy is here" : "Test Buddy removed", () => api.debugTestBuddy(e.currentTarget.checked))} />
      <span>Test Buddy is here</span>
    </label>
    {#if d.testBuddySince}
      <span class="muted small">Turning this off (or debug mode) removes Test Buddy and everything with them: WooHoos, pregnancies, grace periods, and achievements they unlocked.</span>
      <button class="small soft" style="align-self:flex-start" onclick={() => run("Test Buddy's stats cleared", async () => { await api.debugTestBuddy(false); await api.debugTestBuddy(true); })}>🧹 Reset their stats</button>
    {/if}
  </div>

  <div class="group">
    <strong>🧪 Test partner</strong>
    <span class="muted small">Copy a user ID from someone's VRChat profile link (vrchat.com/home/user/<b>usr_…</b>).</span>
    <input bind:value={tName} placeholder="Name" aria-label="Test partner name" />
    <input bind:value={tId} placeholder="usr_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx (optional)" aria-label="Test partner user ID" />
    {#if !tIdOk}<span class="small warn-text">That doesn't look like a user ID yet.</span>{/if}
    <div class="row">
      <button class="small soft" onclick={checkPicture} disabled={!tId.trim() || !tIdOk}>📷 Check picture</button>
      <button class="small" onclick={addToInstance} disabled={!tName.trim() || !tIdOk}>➕ Add to instance</button>
      {#if p}<button class="small soft" onclick={setAsPartner} disabled={!tName.trim() || !tIdOk}>💞 Set as partner</button>{/if}
    </div>
    {#if pic}
      <div class="pic-result">
        {#if pic.loading}
          <span class="muted small">Fetching from vrchat.com…</span>
        {:else if pic.src}
          <img src={pic.src} alt="Profile" />
          <span class="small">✅ Picture fetched and cached. {d.settings.showPictures ? "" : "Turn on \"Show VRChat pictures\" to see it in the app."}</span>
        {:else}
          <span class="small warn-text">❌ {pic.error}</span>
        {/if}
      </div>
    {/if}
    <span class="muted small">In the instance now: {here.length ? "" : "nobody"}</span>
    {#each here as o}
      <div class="row person">
        <span class="grow small">{o.name} <span class="muted">{o.userId ?? "(no ID)"}</span></span>
        <button class="small ghost" onclick={() => run(`${o.name} left`, () => api.debugRemovePerson(o.name, o.userId))}>Leave</button>
      </div>
    {/each}
    <span class="muted small">Tip: with exactly one person here, a debug WooHoo guesses them as the partner.</span>
  </div>

  <div class="group">
    <strong>💬 Chatbox notes</strong>
    <span class="muted small">Shown in your VRChat chatbox (starting with 🔧) while debug mode is on, even if the chatbox is off in Settings.</span>
    {#each CHAT_NOTES as n (n.id)}
      <label class="note">
        <input type="checkbox" checked={!!d.settings.chatbox.debug?.[n.id]} onchange={(e) => toggleNote(n.id, e.currentTarget.checked)} />
        <span>{n.label}</span>
      </label>
    {/each}
  </div>

  <div class="group">
    <strong>⏩ Grace period</strong>
    <span class="muted small">{d.waiting.length ? `${d.waiting.length} waiting.` : "Nothing waiting right now."}</span>
    <button class="small soft" onclick={() => run("Reveal now", api.debugRevealNow)} disabled={!d.waiting.length}>Find out now</button>
  </div>

  <div class="group">
    <strong>👶 How many babies</strong>
    <span class="muted small">
      {#if p}This pregnancy has {p.babyCount}. The Home card shows them from week 12.{:else if w}Waiting to find out: sets how many there will be.{:else}Not pregnant or waiting on this avatar.{/if}
    </span>
    <div class="segmented" style="align-self:flex-start">
      {#each [1, 2, 3, 4] as n}
        <button class:on={(p?.babyCount ?? w?.babyCount) === n} disabled={!p && !w} onclick={() => run(n === 1 ? "One baby" : `${n} babies`, () => api.debugSetBabies(n))}>{n}</button>
      {/each}
    </div>
  </div>

  <div class="group">
    <strong>⏩ Labor</strong>
    <span class="muted small">
      {#if !p}Not pregnant on this avatar.{:else if p.labor}In labor: {p.labor.phase === "early" ? "early" : p.labor.phase === "active" ? "water broke" : "pushing"}. Skips play out within a second, with the real events.{:else}Not in labor yet. Starting it here skips the week and avatar checks.{/if}
    </span>
    <div class="row">
      <button class="small" onclick={() => run("Labor started", () => api.debugLabor(null))} disabled={!p || !!p.labor}>🤰 Start labor</button>
      <button class="small soft" onclick={() => run("+1 minute", () => api.debugLabor("minute"))} disabled={!p?.labor}>+1 min</button>
      <button class="small soft" onclick={() => run("Next phase", () => api.debugLabor("nextPhase"))} disabled={!p?.labor}>⏭ Next phase</button>
      <button class="small soft" onclick={() => run("Born", () => api.debugLabor("birth"))} disabled={!p?.labor}>🍼 Born now</button>
    </div>
    {#if p?.labor && st?.laborPaused}<span class="small warn-text">Labor is paused right now, so skips wait until it runs again.</span>{/if}
  </div>

  <div class="group">
    <strong>🧍 Posture</strong>
    <span class="muted small">For "Resting holds off labor". In VR with SteamVR: resting when your headset is 0.45 m below your normal height (the highest it's spent 20 s at), tipped about 70° over, or your waist tracker says you're lying down. Otherwise: VRChat's Upright 0.7 or more, not in a seat.</span>
    <span class="small">{st?.up === false ? "🛋️ resting" : "🚶 up"} · by {st?.postureSource === "steamvr" ? "SteamVR" : "VRChat"}{st?.afk ? " · 💤 AFK" : ""}</span>
    <span class="small muted">Upright {st?.upright != null ? st.upright.toFixed(2) : "not heard yet"}{#if st?.vrDropM != null} · headset {st.vrDropM.toFixed(2)} m below standing{/if}{#if st?.vrStandingM} (saved normal height {st.vrStandingM.toFixed(2)} m){/if}{#if st?.postureSource === "steamvr"} · {st.vrProven ? "height proven this session" : "height not proven yet this session"}{/if}{#if st?.vrHeightDoubt} · 🤔 avatar standing, SteamVR says sitting{/if}{#if st?.vrTilted} · headset tipped over{/if}{#if st?.vrLying != null} · hip tracker: {st.vrLying ? "lying down" : "upright"}{/if}{#if st?.vrTrackers} · {st.vrTrackers} trackers, waist {st.vrWaist ? (st.vrHipTracked ? "✓" : "lost") : "not set"}{/if}</span>
    {#if hipWarning(st)}<span class="small warn-text">⚠️ {hipWarning(st)}</span>{/if}
  </div>

  <div class="group">
    <strong>💓 Kick now</strong>
    <span class="muted small">Sends a kick to your avatar straight away (and logs it on Home if you're pregnant).</span>
    <div class="row">
      <button class="small soft" onclick={() => run("Little kick", () => api.debugKick("weak"))}>Little</button>
      <button class="small soft" onclick={() => run("Strong kick", () => api.debugKick("strong"))}>Strong</button>
      <button class="small soft" onclick={() => run("Big kick", () => api.debugKick("extreme"))}>Big</button>
    </div>
  </div>

  <div class="group">
    <strong>🌸 Cycle</strong>
    <span class="muted small">Today is day {nowDay ?? "-"}. Days 11 to 14 are high/peak.</span>
    <div class="row">
      <input type="range" min="1" max="28" bind:value={day} class="grow" aria-label="Cycle day" />
      <button class="small soft" onclick={() => run(`Cycle day ${day}`, () => api.debugSetCycleDay(day))}>Set day {day}</button>
    </div>
  </div>

  {#if p}
    <div class="group">
      <strong>🤰 Pregnancy</strong>
      <span class="muted small">Now at week {Math.floor(progress(p.timeline, app.now) * TERM_WEEKS)}. Week 12 shows how many, week 20 shows genders, week 40 is due.</span>
      <div class="row">
        <input type="range" min="0" max="42" bind:value={week} class="grow" aria-label="Week" />
        <button class="small soft" onclick={jumpWeek}>Jump to week {week}</button>
      </div>
    </div>
  {/if}

  <div class="group readout">
    <strong>📡 Live</strong>
    <code>
      VRChat: {st?.vrchatRunning ? "running" : "not running"} · OSC: {st?.oscActive ? `linked${st.oscquery ? " (OSCQuery)" : ""}` : "off"}<br />
      Sending to: {st?.sendTarget ?? "-"} · listening on {st?.recvPort ?? "-"}<br />
      Belly sent: {st?.lastStage != null ? st.lastStage.toFixed(4) : "-"} · extra size: {st?.lastExtra != null ? st.lastExtra.toFixed(2) : "-"}<br />
      Avatar: {st?.avatarId ?? "-"}<br />
      Avatar params: {st?.avatarParams ? st.avatarParams.filter((n) => n.startsWith("Pregnancy")).join(", ") || "(none with Pregnancy/)" : "unknown"}<br />
      Instance: {st?.worldName ?? "-"} · others: {st?.others.map((o) => o.name).join(", ") || "nobody"}<br />
      Log: {st?.logFile?.split(/[\\/]/).pop() ?? "-"}
      {#if st?.note}<br />Note: {st.note}{/if}
    </code>
  </div>
</section>

<style>
  .note {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.88rem;
    cursor: pointer;
  }
  .note input {
    accent-color: var(--accent);
    margin: 0;
  }
  .debug {
    display: flex;
    flex-direction: column;
    gap: 10px;
    border-style: dashed;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    background: var(--bg);
    border-radius: 14px;
    padding: 10px 12px;
  }
  .pic-result {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--bg-3);
    border-radius: 12px;
    padding: 8px;
  }
  .pic-result img {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    object-fit: cover;
  }
  .person {
    padding: 2px 0;
  }
  .warn-text {
    color: var(--danger);
    font-weight: 700;
  }
  .readout code {
    font-size: 0.75rem;
    line-height: 1.6;
    word-break: break-all;
    background: none;
    padding: 0;
  }
</style>
