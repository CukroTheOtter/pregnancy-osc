<script lang="ts">
  import MomentCard from "./MomentCard.svelte";
  import ScanShare from "./ScanShare.svelte";
  import Ultrasound from "./Ultrasound.svelte";
  import { app, confirmAsk, errorToast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { scanMeta, type JournalItem } from "../lib/journal";
  import { progress, TERM_WEEKS } from "../lib/timeline";
  import type { Pregnancy } from "../lib/types";

  // A pregnancy's journal as a little timeline, newest at the top.
  // `focus`: an entry to scroll to and highlight (the calendar's "Open").
  let { p, items, focus = null }: { p: Pregnancy; items: JournalItem[]; focus?: string | null } = $props();
  let list = $state<HTMLOListElement>();
  $effect(() => {
    if (!focus || !list) return;
    list.querySelector(`[data-key="${CSS.escape(focus)}"]`)?.scrollIntoView({ block: "center" });
  });

  const dateFmt = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" });
  let sharing = $state<JournalItem | null>(null);
  const weekAt = (t: number) => Math.min(40, Math.floor(progress(p.timeline, Math.min(t, p.endedMs ?? app.now)) * TERM_WEEKS));

  async function remove(key: string) {
    if (!(await confirmAsk({ title: "Delete this note?", text: "It's removed from the journal.", ok: "Delete", danger: true }))) return;
    try {
      await api.deleteJournalEntry(key);
    } catch (e) {
      errorToast(e);
    }
  }
</script>

<ol class="timeline" bind:this={list}>
  {#each items as it (it.key)}
    <li class:note={it.note} class:rare={it.rare} class:focus={focus === it.key} data-key={it.key}>
      <span class="dot" aria-hidden="true">{it.emoji}</span>
      <div class="body">
        <div class="head">
          <strong class="grow">{it.title || "Note"}{#if it.rare}<span class="sparkle" title="A rare one">&nbsp;✨</span>{/if}</strong>
          <span class="muted small when">Week {it.week != null ? Math.max(0, Math.floor(it.week)) : weekAt(it.timeMs)} · {dateFmt.format(it.timeMs)}</span>
          {#if it.note}<button class="icon ghost del" onclick={() => remove(it.key)} aria-label="Delete note" title="Delete">✕</button>{/if}
        </div>
        {#if it.text}<p class="text">{it.text}</p>{/if}
        {#if it.scan && app.data}
          <button class="pic" onclick={() => (sharing = it)} title="Copy or save this scan" aria-label="Copy or save this scan">
            <Ultrasound scan={it.scan} meta={scanMeta(app.data, p, it.timeMs, it.scan.week)} />
            <span class="hint-pill">📤 Copy or save</span>
          </button>
        {:else if it.card}
          <button class="pic" onclick={() => (sharing = it)} title="Copy or save this picture" aria-label="Copy or save this picture">
            <MomentCard card={it.card} />
            <span class="hint-pill">📤 Copy or save</span>
          </button>
        {/if}
      </div>
    </li>
  {/each}
</ol>

{#if sharing && (sharing.scan || sharing.card)}
  <ScanShare {p} scan={sharing.scan} card={sharing.card} timeMs={sharing.timeMs} title="{sharing.emoji} {sharing.title || (sharing.scan ? `Week ${sharing.scan.week} scan` : 'Moment')}" onclose={() => (sharing = null)} />
{/if}

<style>
  .timeline {
    list-style: none;
    margin: 0;
    padding: 0;
    position: relative;
  }
  .timeline::before {
    content: "";
    position: absolute;
    left: 15px;
    top: 10px;
    bottom: 10px;
    border-left: 2px dashed var(--line);
  }
  li.focus {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    border-radius: 12px;
  }
  li {
    display: flex;
    gap: 10px;
    padding: 6px 0;
    position: relative;
  }
  .dot {
    width: 32px;
    height: 32px;
    flex-shrink: 0;
    border-radius: 50%;
    background: var(--bg-3);
    display: grid;
    place-items: center;
    font-size: 1rem;
    box-shadow: 0 0 0 3px var(--bg-2);
  }
  .note .dot {
    background: var(--accent);
  }
  .rare .dot {
    background: linear-gradient(135deg, var(--accent), var(--accent-2));
  }
  .sparkle {
    font-weight: 400;
  }
  .body {
    flex: 1;
    min-width: 0;
    padding-top: 5px;
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-wrap: wrap;
  }
  .when {
    white-space: nowrap;
  }
  .del {
    padding: 0 6px;
    font-size: 0.8rem;
    align-self: center;
  }
  .text {
    margin: 2px 0 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .pic {
    position: relative;
    display: block;
    margin-top: 8px;
    padding: 0;
    width: 100%;
    max-width: 420px;
    background: none;
    border: none;
    box-shadow: none;
    border-radius: 14px;
    cursor: pointer;
  }
  .hint-pill {
    position: absolute;
    right: 8px;
    top: 52px;
    padding: 3px 10px;
    border-radius: 99px;
    background: rgba(0, 0, 0, 0.6);
    color: #fff;
    font-size: 0.75rem;
    font-weight: 700;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .pic:hover .hint-pill,
  .pic:focus-visible .hint-pill {
    opacity: 1;
  }
</style>
