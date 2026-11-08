<script lang="ts">
  // What came of each wait on this avatar (or its group): one row per wait, one result.
  import { app } from "../lib/app.svelte";
  import { funRows } from "../lib/scope";
  import { shownName } from "../lib/privacy";
  import Section from "./Section.svelte";

  let { avatarId }: { avatarId: string | null } = $props();

  const LABEL = { waiting: "⏳ Waiting", pregnant: "🤰 Pregnant", notPregnant: "🤍 Not this time", alreadyPregnant: "🤰 Already pregnant", notPartner: "🚫 Not a partner", protected: "🛡️ Protected" };
  const rows = $derived(app.data ? funRows(app.data, avatarId) : []);
  const fmt = new Intl.DateTimeFormat(undefined, { weekday: "short", month: "short", day: "numeric", hour: "numeric", minute: "2-digit" });
</script>

{#if rows.length}
  <Section id="woohoo-history" title="💕 WooHoo history">
    {#snippet extra()}<span class="pill">{rows.length}</span>{/snippet}
    <ul class="list">
      {#each rows as r (r.timeMs)}
        <li>
          <div class="grow">
            <strong class="small">{fmt.format(r.timeMs)}</strong>
            <div class="muted small">{[r.worldName, r.partnerName ? `with ${shownName(app.data, r.partnerName, r.partnerUserId)}` : "With someone", r.count > 1 ? `💕 ${r.count}` : null].filter(Boolean).join(" · ")}</div>
          </div>
          <span class="small result">{LABEL[r.result]}</span>
        </li>
      {/each}
    </ul>
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
