<script lang="ts">
  import { onDestroy } from "svelte";
  import XrayBelly from "../components/XrayBelly.svelte";
  import Baby from "../components/Baby.svelte";
  import { app, currentPregnancy, updatePregnancy, errorToast, toast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { babyViews, lookFrom } from "../lib/look";
  import { computeBaby, contextFrom, blendLabel } from "../lib/genetics";
  import { dueReveals, isTraitVisible, nextReveal, REVEALS, type RevealDef } from "../lib/reveals";
  import { traitLabel, type TraitKey } from "../lib/species";
  import { progress, week } from "../lib/timeline";
  import { playChime, startHeartbeat, stopHeartbeat } from "../lib/sound";
  import { saveSvgAsPng } from "../lib/share";
  import type { Keepsake, SpoilerMode } from "../lib/types";

  const p = $derived(currentPregnancy());
  const due = $derived(p ? dueReveals(p, app.now) : []);
  const upcoming = $derived(p ? nextReveal(p, app.now) : undefined);
  const spoiler = $derived(app.data?.settings.spoilerMode ?? "surprise");

  // Scan session state
  let scanning = $state<RevealDef | null>(null);
  let clarity = $state(0);
  let phase = $state<"scan" | "resolved" | "color">("scan");
  let probe = $state({ x: 50, y: 50, on: false });
  let last: { x: number; y: number } | null = null;
  let canvas = $state<HTMLCanvasElement | null>(null);
  let raf = 0;

  function drawNoise() {
    const c = canvas;
    if (c) {
      const ctx = c.getContext("2d")!;
      const img = ctx.createImageData(c.width, c.height);
      for (let i = 0; i < img.data.length; i += 4) {
        const v = Math.random() * 255;
        img.data[i] = img.data[i + 1] = img.data[i + 2] = v;
        img.data[i + 3] = 255;
      }
      ctx.putImageData(img, 0, 0);
    }
    raf = requestAnimationFrame(drawNoise);
  }

  function start(r: RevealDef) {
    scanning = r;
    clarity = 0;
    phase = "scan";
    last = null;
    cancelAnimationFrame(raf);
    raf = requestAnimationFrame(drawNoise);
    const s = app.data!.settings;
    if (s.heartbeatSound) startHeartbeat(s.soundVolume);
  }

  function move(e: PointerEvent) {
    const el = e.currentTarget as HTMLElement;
    const rect = el.getBoundingClientRect();
    const x = ((e.clientX - rect.left) / rect.width) * 100;
    const y = ((e.clientY - rect.top) / rect.height) * 100;
    probe = { x, y, on: true };
    // Mouse scans while the button is held; touch and pen scan while touching.
    if (phase !== "scan" || (e.pointerType === "mouse" && !(e.buttons & 1))) {
      last = null;
      return;
    }
    if (last) {
      const d = Math.hypot(x - last.x, y - last.y);
      // Moving over the middle (where the baby is) clears the picture faster.
      const center = 1 - Math.min(1, Math.hypot(x - 50, y - 55) / 60);
      clarity = Math.min(1, clarity + d * (0.0028 + center * 0.004));
      if (clarity >= 1) resolve();
    }
    last = { x, y };
  }

  function resolve() {
    phase = "resolved";
    stopHeartbeat();
    setTimeout(() => (phase = "color"), 1400);
    if (app.data!.settings.heartbeatSound) playChime(app.data!.settings.soundVolume);
  }

  async function save(r: RevealDef) {
    if (!p || !app.data) return;
    const now = Date.now();
    const w = week(p.timeline, now);
    const ctx = contextFrom(app.data);
    try {
      await updatePregnancy(p.id, (x) => (x.reveals[r.key] = now));
      const lines: string[] = [];
      for (const b of p.babies) {
        const res = computeBaby(p, b, ctx);
        const name = b.name ?? `Baby ${b.index + 1}`;
        if (r.key === "species") {
          const v = blendLabel(res.blend);
          lines.push(`${name}: ${v}${res.hybrid ? " (a rare hybrid!)" : ""}`);
          await api.addKeepsake({ id: "", pregnancyId: p.id, babyIndex: b.index, traitKey: "species", value: v, week: w, timeMs: now } as Keepsake);
        }
        for (const t of r.traits) {
          const v = res.traits[t].value;
          lines.push(`${name}: ${traitLabel(t, v)}`);
          await api.addKeepsake({ id: "", pregnancyId: p.id, babyIndex: b.index, traitKey: t, value: v, week: w, timeMs: now } as Keepsake);
        }
      }
      await api.saveJournalEntry({ id: "", timeMs: now, pregnancyId: p.id, kind: "reveal", title: `Scan: ${r.label}`, text: lines.join("\n"), image: null });
      if (app.data.settings.revealPulse) await api.pulse("revealPulse", 1000);
      toast("Keepsake saved", r.label, "success");
    } catch (e) {
      errorToast(e);
    }
  }

  async function finish() {
    if (scanning) await save(scanning);
    close();
  }

  async function skipAll() {
    for (const r of due) await save(r);
  }

  function close() {
    scanning = null;
    stopHeartbeat();
    cancelAnimationFrame(raf);
  }

  onDestroy(close);

  async function setSpoiler(m: SpoilerMode) {
    if (!app.data) return;
    const s = { ...$state.snapshot(app.data.settings), spoilerMode: m };
    await api.saveSettings(s);
    app.data.settings.spoilerMode = m;
  }

  const views = $derived(p && app.data ? babyViews(p, app.data, true) : []);
  // During a scan, show what was already revealed plus this scan's traits only.
  const scanLooks = $derived.by(() => {
    if (!p || !app.data || !scanning) return [];
    const ctx = contextFrom(app.data);
    const sc = scanning;
    return p.babies.map((b) =>
      lookFrom(computeBaby(p, b, ctx), (k) => sc.traits.includes(k) || isTraitVisible(p, k, spoiler, b.locks[k] != null)),
    );
  });
  const keepsakes = $derived(p ? (app.data?.keepsakes ?? []).filter((k) => k.pregnancyId === p.id).sort((a, b) => b.timeMs - a.timeMs) : []);
  const dateFmt = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric" });

  function keepsakeLook(k: Keepsake) {
    if (!p || !app.data) return {};
    const b = p.babies.find((x) => x.index === k.babyIndex) ?? p.babies[0];
    const res = computeBaby(p, b, contextFrom(app.data));
    // Show the traits revealed by then.
    return lookFrom(res, (t: TraitKey) => keepsakes.some((x) => x.babyIndex === k.babyIndex && x.traitKey === t && x.timeMs <= k.timeMs));
  }

  function keepsakeText(k: Keepsake) {
    return k.traitKey === "species" ? k.value : traitLabel(k.traitKey as TraitKey, k.value);
  }

  let cardEls: Record<string, SVGSVGElement> = $state({});
</script>

<div class="view">
  <header class="row">
    <h1 class="grow">💫 Ultrasound scans</h1>
    <div class="segmented" role="radiogroup" aria-label="Spoilers">
      {#each [["surprise", "Surprise me"], ["hints", "Hints"], ["all", "Show everything"]] as [m, l]}
        <button class:on={spoiler === m} onclick={() => setSpoiler(m as SpoilerMode)}>{l}</button>
      {/each}
    </div>
  </header>

  {#if !p}
    <div class="card"><p class="muted">No pregnancy yet.</p></div>
  {:else if scanning}
    <div class="card session">
      <h2>{spoiler === "surprise" && phase === "scan" ? "Scanning…" : scanning.label}</h2>
      <p class="muted small">{phase === "scan" ? "Drag the probe gently over the belly until the picture clears." : phase === "resolved" ? "There they are!" : "Look at that!"}</p>
      <div
        class="screen"
        class:done={phase !== "scan"}
        role="application"
        aria-label="Ultrasound screen: drag to scan"
        onpointermove={move}
        onpointerdown={(e) => { (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId); move(e); }}
        onpointerup={() => (last = null)}
        onpointerleave={() => (probe.on = false)}
      >
        <div class="sonar" style="filter: blur({(1 - clarity) * 14}px) contrast({1 + clarity}); opacity:{phase === 'color' ? 0 : 1}">
          <XrayBelly progress={progress(p.timeline, app.now)} babies={scanLooks} egg={p.kind === "egg"} sonar />
        </div>
        <div class="color" style="opacity:{phase === 'color' ? 1 : 0}">
          <XrayBelly progress={progress(p.timeline, app.now)} babies={scanLooks} egg={p.kind === "egg"} />
        </div>
        <canvas bind:this={canvas} width="120" height="120" class="noise" style="opacity:{phase === 'scan' ? 0.75 - clarity * 0.6 : 0}"></canvas>
        {#if probe.on && phase === "scan"}
          <div class="probe" style="left:{probe.x}%;top:{probe.y}%"></div>
        {/if}
        <div class="meter"><div style="width:{clarity * 100}%"></div></div>
      </div>

      {#if phase === "color"}
        <div class="reveals">
          {#each views as v}
            <div class="reveal-card">
              <strong>{v.name}</strong>
              {#if scanning.key === "species"}
                <span>{blendLabel(v.result.blend)}{v.result.hybrid ? " ✨" : ""}</span>
              {/if}
              {#each scanning.traits as t}
                <span>
                  {#if t === "furColor" || t === "eyeColor"}<span class="sw" style="background:{v.result.traits[t].value}"></span>{/if}
                  {traitLabel(t, v.result.traits[t].value)}
                </span>
              {/each}
            </div>
          {/each}
        </div>
        <div class="row" style="justify-content:flex-end">
          <button onclick={finish}>Save keepsake 💖</button>
        </div>
      {:else}
        <div class="row" style="justify-content:flex-end">
          <button class="soft" onclick={close}>Later</button>
        </div>
      {/if}
    </div>
  {:else}
    {#if due.length}
      <div class="card ready">
        <div class="grow">
          <h2>{due.length === 1 ? "A scan is ready!" : `${due.length} scans are ready!`}</h2>
          <p class="muted">{spoiler === "surprise" ? "Something new to find out." : due.map((d) => d.label).join(", ")}</p>
        </div>
        <button onclick={() => start(due[0])}>Start scan</button>
        {#if due.length > 1}<button class="soft" onclick={skipAll}>Reveal all now</button>{/if}
      </div>
    {:else if upcoming}
      <div class="card"><p>Next scan at <strong>week {upcoming.week}</strong>{spoiler !== "surprise" ? `: ${upcoming.label}` : ""}. You're at week {Math.floor(week(p.timeline, app.now))}.</p></div>
    {:else}
      <div class="card"><p>All scans done! 🎉</p></div>
    {/if}

    <h2 style="margin-top:22px">Schedule</h2>
    <div class="schedule">
      {#each REVEALS as r}
        <div class="sched" class:done={p.reveals[r.key] != null}>
          <span class="wk">Wk {r.week}</span>
          <span>{p.reveals[r.key] != null || spoiler === "all" ? r.label : spoiler === "hints" ? r.hint : "Surprise"}</span>
          {#if p.reveals[r.key] != null}<span class="muted small">✓ {dateFmt.format(p.reveals[r.key])}</span>{/if}
        </div>
      {/each}
    </div>

    <h2 style="margin-top:22px">Keepsakes</h2>
    {#if keepsakes.length === 0}
      <p class="muted">Each reveal becomes a keepsake card here.</p>
    {:else}
      <div class="keepsakes">
        {#each keepsakes as k (k.id)}
          <div class="keepsake">
            <svg bind:this={cardEls[k.id]} viewBox="0 0 240 300" class="kcard" role="img" aria-label="Keepsake: {keepsakeText(k)}">
              <rect width="240" height="300" rx="22" fill="#fff6f9" />
              <rect x="10" y="10" width="220" height="280" rx="16" fill="none" stroke="var(--accent)" stroke-width="3" stroke-dasharray="6 6" />
              <circle cx="120" cy="120" r="78" fill="#ffe1ea" />
              <g transform="translate(120 128)"><Baby look={keepsakeLook(k)} awake scale={1.15} /></g>
              <text x="120" y="230" text-anchor="middle" font-family="Nunito, sans-serif" font-weight="800" font-size="17" fill="#4a2f3d">{keepsakeText(k)}</text>
              <text x="120" y="254" text-anchor="middle" font-family="Nunito, sans-serif" font-size="12" fill="#9a7b8a">{p.babies.find((b) => b.index === k.babyIndex)?.name ?? `Baby ${k.babyIndex + 1}`} · week {Math.floor(k.week)}</text>
              <text x="120" y="274" text-anchor="middle" font-family="Nunito, sans-serif" font-size="11" fill="#9a7b8a">{dateFmt.format(k.timeMs)}</text>
            </svg>
            <button class="small soft" onclick={() => saveSvgAsPng(cardEls[k.id], 240, 300, `keepsake-${k.traitKey}-${Math.floor(k.week)}`).catch(errorToast)}>Save image</button>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .session { max-width: 640px; margin: 10px auto; }
  .screen {
    position: relative;
    aspect-ratio: 1;
    max-width: 460px;
    margin: 10px auto;
    background: #0b0b0b;
    border-radius: 20px;
    overflow: hidden;
    touch-action: none;
    cursor: crosshair;
    /* Ultrasound fan: a wedge with a curved bottom. */
    -webkit-mask-image: conic-gradient(from 148deg at 50% -8%, #000 64deg, transparent 64deg), radial-gradient(circle at 50% -8%, #000 82%, transparent 82.5%);
    -webkit-mask-composite: source-in;
    mask-image: conic-gradient(from 148deg at 50% -8%, #000 64deg, transparent 64deg), radial-gradient(circle at 50% -8%, #000 82%, transparent 82.5%);
    mask-composite: intersect;
    transition: -webkit-mask-image 1s, mask-image 1s;
  }
  .screen.done { -webkit-mask-image: none; mask-image: none; background: var(--bg-3); }
  .sonar, .color { position: absolute; inset: 4%; transition: opacity 1.2s ease, filter 0.2s; }
  .noise { position: absolute; inset: 0; width: 100%; height: 100%; mix-blend-mode: screen; image-rendering: pixelated; pointer-events: none; transition: opacity 0.6s; }
  .probe {
    position: absolute;
    width: 46px;
    height: 46px;
    margin: -23px 0 0 -23px;
    border-radius: 50%;
    border: 3px solid rgba(255, 255, 255, 0.7);
    box-shadow: 0 0 18px rgba(255, 255, 255, 0.4);
    pointer-events: none;
  }
  .meter { position: absolute; left: 20%; right: 20%; bottom: 14px; height: 6px; background: rgba(255,255,255,.15); border-radius: 99px; }
  .meter div { height: 100%; background: var(--accent); border-radius: 99px; transition: width 0.15s; }
  .screen.done .meter { display: none; }
  .reveals { display: flex; flex-wrap: wrap; gap: 10px; justify-content: center; margin: 12px 0; }
  .reveal-card { background: var(--bg-3); border-radius: 14px; padding: 10px 14px; display: flex; flex-direction: column; gap: 4px; animation: pop 0.4s ease; }
  @keyframes pop { from { transform: scale(0.8); opacity: 0; } }
  .sw { width: 12px; height: 12px; border-radius: 50%; display: inline-block; vertical-align: middle; }
  .ready { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; border-color: var(--accent); }
  .schedule { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 8px; }
  .sched { display: flex; gap: 10px; align-items: center; background: var(--bg-2); border: 1.5px solid var(--line); border-radius: 12px; padding: 8px 12px; }
  .sched.done { border-color: var(--ok); }
  .wk { font-weight: 800; color: var(--accent); min-width: 48px; }
  .keepsakes { display: grid; grid-template-columns: repeat(auto-fill, minmax(170px, 1fr)); gap: 14px; }
  .keepsake { display: flex; flex-direction: column; gap: 6px; align-items: center; }
  .kcard { width: 100%; height: auto; filter: drop-shadow(0 4px 12px rgba(120, 60, 90, 0.15)); }
</style>
