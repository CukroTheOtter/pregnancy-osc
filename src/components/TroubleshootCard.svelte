<script lang="ts">
  // Settings: a checkup (live status, and problems in the save or settings with
  // fixes, from pregsys-core/src/check.rs), then the common questions, short.
  // The full guide is docs/troubleshooting.md (keep them in step).
  import { app, checkProblems, errorToast, toast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { hasSystem } from "../lib/avatars";
  import type { Problem } from "../lib/types";
  import Section from "./Section.svelte";

  const RESET = "In VRChat: Action Menu > Options > OSC > Reset Config, then switch avatars and back.";
  const FIXES: { emoji: string; title: string; steps: string[] }[] = [
    { emoji: "🫃", title: "The belly isn't changing", steps: ["Is Hide Belly on, or is it hidden in a public instance? Home has a Show button.", "Paused, or waiting to find out? The app leaves the belly alone, or keeps it flat.", "Your avatar uses its own belly name? Set it in Avatars > Edit."] },
    { emoji: "💕", title: "WooHoos aren't noticed", steps: ["Settings > Detecting WooHoos: is it on, and does your socket's type count?", "No sockets listed? They need VRCFury haptics on your avatar.", "While you're pregnant they're kept as “Already pregnant”, not tries.", "With Only my partners on, they only count with your 💞 partners."] },
    { emoji: "🧩", title: "An avatar “doesn't have the system”", steps: ["Wear it and wait about 30 seconds.", RESET, "Check the system is on it in Unity, then upload it again."] },
    { emoji: "📦", title: "Something's missing after an upload", steps: [`VRChat can keep the old list. ${RESET}`, "Check that parameter is on the uploaded avatar."] },
    { emoji: "📡", title: "It says No OSC", steps: ["In VRChat: Action Menu > Options > OSC > Enabled.", "Using an OSC router? Set your ports under VRChat."] },
    { emoji: "🍼", title: "Labor won't start, or is paused", steps: ["It needs VRChat open and an avatar with Pregnancy/Labor.", "It waits in public instances by default (Safety).", "Being AFK, Resting holds off labor and Labor waits for the other parent can hold it too."] },
    { emoji: "❔", title: "The other parent is unknown", steps: ["That's normal. VRChat doesn't say who it was, so the app only names someone when there's no doubt. Pick them on the Other parent card."] },
    { emoji: "⏳", title: "The weeks and the wait don't match", steps: ["Only the weeks are scaled to your pregnancy length. Hours, days and dates are always real time."] },
  ];

  const status = $derived(app.status);
  const worn = $derived(status?.avatarId ? app.data?.avatars[status.avatarId] : undefined);
  /** Right now: ok, or what's wrong (a line each). */
  const live = $derived.by(() => {
    const s = app.data?.settings;
    const out: { ok: boolean; text: string }[] = [
      { ok: !!status?.vrchatRunning, text: status?.vrchatRunning ? "VRChat is running" : "VRChat isn't running (the app links up when it starts)" },
    ];
    if (status?.vrchatRunning) {
      out.push({ ok: !!status.oscActive, text: status.oscActive ? "Linked over OSC" : "No OSC: turn it on in VRChat's Action Menu > Options > OSC" });
      if (status.oscActive && worn && !status.avatarLoading && hasSystem(worn) === false) out.push({ ok: false, text: "The avatar you're wearing doesn't show the Pregnancy System" });
      else if (status.oscActive && worn && hasSystem(worn)) out.push({ ok: true, text: "Your avatar has the Pregnancy System" });
    }
    if (s?.readLogs) out.push({ ok: !!status?.logDir, text: status?.logDir ? "Reading VRChat's logs" : "VRChat's log folder wasn't found (Settings > Logs and pictures)" });
    return out;
  });

  // Checked again whenever the save changes (app.svelte.ts refresh).
  const problems = $derived(app.problems);
  let fixing = $state<string | null>(null);

  async function fix(p: Problem, i: number) {
    fixing = p.id;
    try {
      if (await api.fixProblem(p.id, i)) toast("Fixed", p.fixes[i], "success", 3000);
      await checkProblems();
    } catch (e) {
      errorToast(e);
    } finally {
      fixing = null;
    }
  }

  const dateFmt = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" });

  // Shown first, so you can see what you're sharing.
  let details = $state<string | null>(null);
  async function showDetails() {
    try {
      details = await api.copyDetails();
    } catch (e) {
      errorToast(e);
    }
  }
  async function copy() {
    try {
      await navigator.clipboard.writeText(details ?? "");
      toast("Copied", "Paste it wherever you're asking for help.", "success", 3000);
    } catch {
      toast("Couldn't copy", "Select the text and copy it yourself.", "info");
    }
  }
</script>

<Section id="troubleshooting" title="🩹 Troubleshooting">
  {#snippet extra()}
    {#if problems.length}<span class="pill warn">{problems.length} to fix</span>{:else}<span class="pill ok">all good</span>{/if}
  {/snippet}

  <h4>Checkup</h4>
  <ul class="live">
    {#each live as l (l.text)}
      <li class:bad={!l.ok}><span class="mark" aria-hidden="true">{l.ok ? "✓" : "!"}</span>{l.text}</li>
    {/each}
  </ul>

  {#if problems.length}
    <div class="problems">
      {#each problems as p (p.id)}
        <div class="problem">
          <span class="ptext">⚠️ {p.text}</span>
          {#if p.atMs}<span class="muted small">{dateFmt.format(p.atMs)}</span>{/if}
          <div class="pfixes">
            {#each p.fixes as label, i}
              <button class="small" class:soft={i > 0} disabled={fixing === p.id} onclick={() => fix(p, i)}>{label}</button>
            {/each}
            {#if p.go === "partners"}<button class="small ghost" onclick={() => (app.view = "partners")}>Open Partners</button>{/if}
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <p class="fine small">✓ Nothing wrong in your save or settings.</p>
  {/if}

  <h4>Common questions</h4>
  <div class="faq">
    {#each FIXES as f (f.title)}
      <details name="troubleshoot-faq">
        <summary><span aria-hidden="true">{f.emoji}</span> {f.title}</summary>
        <ul>
          {#each f.steps as step}<li class="small">{step}</li>{/each}
        </ul>
      </details>
    {/each}
  </div>

  <h4>Still stuck?</h4>
  <div class="row actions">
    <button class="small soft" onclick={() => api.openTroubleshooting().catch(errorToast)}>📖 Full guide</button>
    <button class="small soft" onclick={showDetails}>📋 Details to share</button>
    <button class="small soft" onclick={() => api.openFeedbackPage(true).catch(errorToast)}>🐛 Report a bug</button>
  </div>
  {#if details != null}
    <pre class="details small">{details}</pre>
    <span class="muted small">To paste when asking for help. It has this avatar's name and ID, but nothing about other people, your worlds or your WooHoos.</span>
    <div class="row actions">
      <button class="small" onclick={copy}>Copy</button>
      <button class="small ghost" onclick={showDetails}>Refresh</button>
      <button class="small ghost" onclick={() => (details = null)}>Close</button>
    </div>
  {/if}
</Section>

<style>
  h4 {
    margin: 10px 0 2px;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  h4:first-of-type {
    margin-top: 0;
  }
  .live {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .live li {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 0.9rem;
  }
  .mark {
    flex: none;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: 0.75rem;
    font-weight: 900;
    background: color-mix(in srgb, var(--ok) 20%, transparent);
    color: var(--ok);
  }
  .bad .mark {
    background: color-mix(in srgb, var(--warn) 25%, transparent);
    color: var(--warn);
  }
  .problems {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-top: 6px;
  }
  .problem {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 14px;
    border-radius: 14px;
    background: color-mix(in srgb, var(--warn) 14%, var(--bg));
  }
  .ptext {
    font-weight: 700;
    font-size: 0.92rem;
  }
  .pfixes {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .fine {
    margin: 6px 0 0;
    color: var(--ok);
    font-weight: 700;
  }
  .faq {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  details {
    border-radius: 12px;
    background: var(--bg);
  }
  summary {
    cursor: pointer;
    font-weight: 700;
    padding: 10px 12px;
  }
  details ul {
    margin: 0;
    padding: 0 14px 12px 34px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .actions {
    gap: 8px;
    flex-wrap: wrap;
  }
  .details {
    margin: 4px 0 0;
    padding: 8px 10px;
    border-radius: 12px;
    background: var(--bg);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }
</style>
