<script lang="ts">
  // Every WooHoo on this avatar (or its group) and how it turned out.
  import { app } from "../lib/app.svelte";
  import { funHistory, funResult } from "../lib/scope";
  import Section from "./Section.svelte";

  let { avatarId }: { avatarId: string | null } = $props();

  const LABEL = { waiting: "⏳ Waiting", pregnant: "🤰 Pregnant", notPregnant: "🤍 Not this time", unknown: "–" };
  const rows = $derived(app.data ? funHistory(app.data, avatarId).map((f) => ({ f, result: funResult(app.data!, f) })) : []);
  const fmt = new Intl.DateTimeFormat(undefined, { weekday: "short", month: "short", day: "numeric", hour: "numeric", minute: "2-digit" });
</script>

{#if rows.length}
  <Section id="woohoo-history" title="💕 WooHoo history">
    {#snippet extra()}<span class="pill">{rows.length}</span>{/snippet}
    <ul class="list">
      {#each rows as { f, result } (f.timeMs)}
        <li>
          <div class="grow">
            <strong class="small">{fmt.format(f.timeMs)}</strong>
            <div class="muted small">{[f.worldName, f.partnerName && `with ${f.partnerName}`].filter(Boolean).join(" · ") || "Nobody picked"}</div>
          </div>
          <span class="small result">{LABEL[result]}</span>
        </li>
      {/each}
    </ul>
    {#if rows.some((r) => r.result === "unknown")}
      <span class="muted small">– means no result was saved (older WooHoos).</span>
    {/if}
  </Section>
{/if}

<style>
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 320px;
    overflow-y: auto;
  }
  li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 12px;
    background: var(--bg);
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .result {
    flex: none;
    font-weight: 700;
  }
</style>
