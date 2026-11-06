<script lang="ts">
  import JournalList from "./JournalList.svelte";
  import Section from "./Section.svelte";
  import Ultrasound from "./Ultrasound.svelte";
  import { app, errorToast, toast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { journalFor, scanMeta, scanTag, seedOf, shownBabies, type Scan } from "../lib/journal";
  import { progress, TERM_WEEKS } from "../lib/timeline";
  import type { JournalEntry, Pregnancy } from "../lib/types";

  // The journal on Home: what happened so far, and your own notes.
  let { p }: { p: Pregnancy } = $props();

  /** Entries shown before "Show all". */
  const SHOWN = 4;
  const items = $derived(app.data ? journalFor(app.data, p, app.now) : []);
  let all = $state(false);
  let writing = $state(false);
  let text = $state("");
  let scan = $state<Scan | null>(null);

  const week = $derived(Math.floor(progress(p.timeline, app.now) * TERM_WEEKS));

  function open() {
    writing = true;
    text = "";
    scan = null;
  }

  function addScan() {
    // A new look each time, from the same pregnancy.
    scan = { babies: shownBabies(p, week), week, seed: (seedOf(p.id) + Math.floor(Math.random() * 1e6)) >>> 0 };
  }

  async function save() {
    if (!text.trim() && !scan) return;
    const entry: JournalEntry = { id: "", timeMs: 0, pregnancyId: p.id, kind: "note", title: "", text: text.trim(), image: scan ? scanTag(scan) : null };
    try {
      await api.saveJournalEntry(entry);
      writing = false;
      toast("Saved to the journal 📔", undefined, "success");
    } catch (e) {
      errorToast(e);
    }
  }
</script>

<Section id="home-journal" title="📔 Journal" open>
  {#snippet extra()}<span class="pill">{items.length}</span>{/snippet}
  {#if !writing}<button class="small soft add" onclick={open}>📝 Add a note</button>{/if}

  {#if writing}
    <div class="write stack">
      <!-- svelte-ignore a11y_autofocus -->
      <textarea bind:value={text} rows="3" placeholder="How are you feeling? Anything happen today?" autofocus></textarea>
      {#if scan}
        <Ultrasound {scan} meta={app.data ? scanMeta(app.data, p, app.now, scan.week) : null} />
        <div class="row">
          <button class="small soft" onclick={addScan}>🔄 Another scan</button>
          <button class="small ghost" onclick={() => (scan = null)}>Remove scan</button>
        </div>
      {:else if week >= 6}
        <button class="small soft" style="align-self:flex-start" onclick={addScan}>🩻 Add a scan from today</button>
      {/if}
      <div class="row">
        <button onclick={save} disabled={!text.trim() && !scan}>Save</button>
        <button class="soft" onclick={() => (writing = false)}>Cancel</button>
      </div>
    </div>
  {/if}

  <JournalList {p} items={all ? items : items.slice(0, SHOWN)} />
  {#if items.length > SHOWN}
    <button class="link small" onclick={() => (all = !all)}>{all ? "Show less" : `Show all (${items.length})`}</button>
  {/if}
</Section>

<style>
  .add {
    align-self: flex-start;
    margin-bottom: 4px;
  }
  .write {
    margin-bottom: 10px;
  }
  textarea {
    width: 100%;
    resize: vertical;
    font: inherit;
  }
  .link {
    background: none;
    color: var(--accent);
    padding: 8px 0 0;
    align-self: flex-start;
  }
</style>
