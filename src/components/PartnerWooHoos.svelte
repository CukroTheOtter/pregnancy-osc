<script lang="ts">
  // Every WooHoo with one partner, one row per wait (like Home's WooHoo history).
  import { app } from "../lib/app.svelte";
  import { ownerName, owner, partnerFunRows } from "../lib/scope";

  let { partnerKey }: { partnerKey: string } = $props();

  const LABEL = { waiting: "⏳ Waiting", pregnant: "🤰 Pregnant", notPregnant: "🤍 Not this time", unknown: "" };
  const d = $derived(app.data!);
  const rows = $derived(partnerFunRows(d, partnerKey));
  const perAvatar = $derived(d.settings.syncScope === "perAvatar");
  const fmt = new Intl.DateTimeFormat(undefined, { weekday: "short", month: "short", day: "numeric", hour: "numeric", minute: "2-digit" });
</script>

<ul class="list">
  {#each rows as r (r.timeMs)}
    <li>
      <div class="grow">
        <strong class="small">{fmt.format(r.timeMs)}</strong>
        <div class="muted small">
          {[r.worldName, perAvatar && r.avatarId ? `on ${ownerName(d, owner(d, r.avatarId))}` : null, r.count > 1 ? `💕 ${r.count}` : null].filter(Boolean).join(" · ") || "Somewhere cozy"}
        </div>
      </div>
      {#if LABEL[r.result]}<span class="small result">{LABEL[r.result]}</span>{/if}
    </li>
  {:else}
    <li class="muted small">No WooHoos with them yet.</li>
  {/each}
</ul>

<style>
  .list {
    list-style: none;
    margin: 12px 0 0;
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
