<script lang="ts">
  // Settings: the common fixes, short. The full guide is docs/troubleshooting.md (keep them in step).
  import { errorToast, toast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import Section from "./Section.svelte";

  const RESET = "Action Menu > Options > OSC > Reset Config, then switch avatars and back.";
  const FIXES: { title: string; steps: string[] }[] = [
    {
      title: "It says an avatar doesn't have the system",
      steps: ["Wear it and wait about 30 seconds.", RESET, "Check the system is on it in Unity, then upload it again."],
    },
    { title: "A feature is missing after an upload", steps: [`VRChat can keep the old list. ${RESET}`, "Check that parameter is on the uploaded avatar."] },
    {
      title: "The belly isn't changing",
      steps: ["Hide Belly on, or hidden in a public instance? Home has a Show button.", "Paused or waiting to find out? The app leaves it alone, or keeps it flat.", "Your avatar's own belly name goes in Avatars > Edit."],
    },
    { title: "It says No OSC", steps: ["In VRChat: Action Menu > Options > OSC > Enabled.", "Using an OSC router? Set your ports under VRChat above."] },
    {
      title: "Labor won't start, or is paused",
      steps: ["It needs VRChat open and an avatar with Pregnancy/Labor.", "It waits in public instances by default (Safety).", "AFK, Resting holds off labor and Labor waits for your partner can hold it too."],
    },
    {
      title: "WooHoos aren't detected",
      steps: ["Under Detecting WooHoos: is it on, and does your socket count?", "No sockets listed? They need VRCFury haptics on your avatar.", "WooHoos are ignored while you're pregnant."],
    },
    { title: "It says Partner unknown", steps: ["That's normal. VRChat doesn't say who it was, so the app only names someone when there's no doubt. You can pick them on the WooHoo card."] },
    { title: "The weeks and the wait don't match", steps: ["Only the weeks are scaled to your pregnancy length. Hours, days and dates are always real time."] },
  ];

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
  {#each FIXES as f (f.title)}
    <details class="fix">
      <summary>{f.title}</summary>
      <ol>
        {#each f.steps as step}<li class="small">{step}</li>{/each}
      </ol>
    </details>
  {/each}
  <div class="row actions">
    <button class="small soft" onclick={() => api.openTroubleshooting().catch(errorToast)}>📖 Full guide</button>
    <button class="small soft" onclick={showDetails}>📋 Copy details</button>
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
  .fix {
    padding: 8px 10px;
    border-radius: 12px;
    background: var(--bg);
  }
  summary {
    cursor: pointer;
    font-weight: 700;
  }
  ol {
    margin: 6px 0 0;
    padding-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .actions {
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 4px;
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
