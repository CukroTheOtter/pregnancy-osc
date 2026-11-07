<script lang="ts">
  import AvatarPic from "./AvatarPic.svelte";
  import GroupPic from "./GroupPic.svelte";
  import JournalList from "./JournalList.svelte";
  import Modal from "./Modal.svelte";
  import { journalFor } from "../lib/journal";
  import { app, viewPending, viewPregnancy } from "../lib/app.svelte";
  import { canOpenPregnancy, groupMembers, ownerAvatar, ownerName, personKey, pregnancyAvatar, pregnancyAvatarNames, pregnancyGroup } from "../lib/scope";
  import { babyWord, genderText } from "../lib/fertility";
  import { humanDuration, progress, TERM_WEEKS } from "../lib/timeline";
  import type { Pregnancy } from "../lib/types";

  // Every pregnancy with one partner. Per avatar there can be several at once
  // (one per avatar or group), so each says which avatar it's on.
  let { partnerKey }: { partnerKey: string } = $props();

  const d = $derived(app.data!);
  const theirs = (x: { partner: { name: string | null; userId: string | null } }) => personKey(x.partner.name, x.partner.userId) === partnerKey;
  const active = $derived(d.pregnancies.filter((p) => !p.endedMs && theirs(p)).sort((a, b) => progress(b.timeline, app.now) - progress(a.timeline, app.now)));
  const waiting = $derived(d.waiting.filter(theirs));
  const past = $derived(d.pregnancies.filter((p) => p.endedMs && theirs(p)).sort((a, b) => (b.endedMs ?? 0) - (a.endedMs ?? 0)));
  const dateFmt = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", year: "numeric" });
  let reading = $state<Pregnancy | null>(null);
</script>

<div class="list">
  {#each active as p (p.id)}
    {@const pr = progress(p.timeline, app.now)}
    {@const g = pregnancyGroup(d, p)}
    {@const aid = pregnancyAvatar(d, p)}
    {@const on = pregnancyAvatarNames(d, p)}
    <div class="item">
      {#if g}<GroupPic ids={groupMembers(d, g.id)} size={40} />{:else if aid}<AvatarPic id={aid} size={40} />{:else}<span class="dot-pic" aria-hidden="true">🤰</span>{/if}
      <div class="grow">
        <div class="row top">
          <strong>{g ? `💞 ${g.name}` : on.length ? on.join(", ") : "Not on any avatar"}</strong>
          <span class="muted small">Week {Math.floor(pr * TERM_WEEKS)}{p.timeline.pausedAtMs ? " · paused" : ""}</span>
        </div>
        <div class="bar"><div style="width:{Math.min(100, pr * 100)}%"></div></div>
      </div>
      {#if canOpenPregnancy(d, p)}<button class="small soft" onclick={() => viewPregnancy(p)}>View</button>{/if}
    </div>
  {/each}

  {#each waiting as w (w.timeMs)}
    {@const wa = ownerAvatar(d, w.avatarId)}
    {@const wg = w.avatarId ? d.groups?.[w.avatarId] : undefined}
    <div class="item">
      {#if wg}<GroupPic ids={groupMembers(d, wg.id)} size={40} />{:else if wa}<AvatarPic id={wa} size={40} />{:else}<span class="dot-pic" aria-hidden="true">💕</span>{/if}
      <div class="grow">
        <strong>{wg ? "💞 " : ""}{w.avatarId ? ownerName(d, w.avatarId) : "Your avatar"}</strong>
        <div class="muted small">💕 Waiting to find out · {humanDuration(Math.max(0, w.revealMs - app.now))} left</div>
      </div>
      <button class="small soft" onclick={() => viewPending(w)}>View</button>
    </div>
  {/each}

  {#each past as p (p.id)}
    <div class="item past">
      <span class="dot-pic" aria-hidden="true">{p.born ? "🍼" : "🤍"}</span>
      <div class="grow">
        <strong>{p.born ? `${babyWord(p.babyCount).replace("!", "")} born` : "Ended early"}</strong>
        <div class="muted small">
          {dateFmt.format(p.endedMs ?? 0)}{p.born && d.settings.genderMode === "random" && p.genders.length ? ` · ${genderText(p.genders)}` : ""}
        </div>
      </div>
      <button class="small soft" onclick={() => (reading = p)}>📔 Journal</button>
    </div>
  {/each}

  {#if !active.length && !waiting.length && !past.length}
    <p class="muted small none">No pregnancies with them yet.</p>
  {/if}
</div>

{#if reading}
  {@const r = d.pregnancies.find((x) => x.id === reading?.id)}
  {#if r}
    <Modal title="📔 {r.born ? `${babyWord(r.babyCount).replace("!", "")} born` : "Ended early"} · {dateFmt.format(r.endedMs ?? 0)}" onclose={() => (reading = null)}>
      <JournalList p={r} items={journalFor(d, r, app.now)} />
    </Modal>
  {/if}
{/if}

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-top: 12px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .top {
    justify-content: space-between;
  }
  .bar {
    height: 8px;
    background: var(--bg-3);
    border-radius: 99px;
    margin-top: 4px;
    overflow: hidden;
  }
  .bar div {
    height: 100%;
    background: var(--accent);
    border-radius: 99px;
  }
  .dot-pic {
    width: 40px;
    height: 40px;
    border-radius: 14px;
    background: var(--bg-3);
    display: grid;
    place-items: center;
    font-size: 1.2rem;
    flex-shrink: 0;
  }
  .past {
    opacity: 0.85;
  }
  .none {
    margin: 0;
  }
</style>
