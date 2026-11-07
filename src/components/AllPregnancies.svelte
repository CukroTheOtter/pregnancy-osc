<script lang="ts">
  import AvatarPic from "./AvatarPic.svelte";
  import GroupPic from "./GroupPic.svelte";
  import JournalList from "./JournalList.svelte";
  import Modal from "./Modal.svelte";
  import { journalFor } from "../lib/journal";
  import { app, homeAvatarId, currentPregnancy, viewPending, viewPregnancy } from "../lib/app.svelte";
  import { groupMembers, owner, ownerAvatar, ownerName, pregnancyAvatar, pregnancyAvatarNames, pregnancyGroup } from "../lib/scope";
  import { babyWord, genderText } from "../lib/fertility";
  import { progress, humanDuration, TERM_WEEKS } from "../lib/timeline";
  import type { Pregnancy } from "../lib/types";
  import { shownName } from "../lib/privacy";

  const d = $derived(app.data!);
  const shown = $derived(currentPregnancy());
  let showPast = $state(false);
  let showAllPast = $state(false);
  // The past pregnancy whose journal is open.
  let reading = $state<Pregnancy | null>(null);
  /** Past pregnancies shown before "Show all". */
  const PAST_SHOWN = 5;

  const avatarsOf = (p: Pregnancy) => pregnancyAvatarNames(d, p);
  const groupOf = (p: Pregnancy) => pregnancyGroup(d, p);
  const firstAvatarOf = (p: Pregnancy) => pregnancyAvatar(d, p);

  const active = $derived(d.pregnancies.filter((p) => !p.endedMs && p.id !== shown?.id));
  const waiting = $derived(d.waiting.filter((w) => w.avatarId !== owner(d, homeAvatarId())));
  const past = $derived(d.pregnancies.filter((p) => p.endedMs).sort((a, b) => (b.endedMs ?? 0) - (a.endedMs ?? 0)));
  const dateFmt = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", year: "numeric" });

</script>

{#if active.length || waiting.length || past.length}
  <section class="card">
    <h2>🤰 All pregnancies</h2>
    {#if !active.length && !waiting.length}
      <p class="muted small">No other pregnancies right now.</p>
    {/if}
    <ul class="list">
      {#each active as p (p.id)}
        {@const pr = progress(p.timeline, app.now)}
        {@const on = avatarsOf(p)}
        {@const aid = firstAvatarOf(p)}
        {@const g = groupOf(p)}
        <li>
          {#if g}<GroupPic ids={groupMembers(d, g.id)} size={42} />{:else if aid}<AvatarPic id={aid} size={42} />{:else}<span class="dot-pic" aria-hidden="true">🤰</span>{/if}
          <div class="grow">
            <div class="row top">
              <strong>{g ? `💞 ${g.name}` : on.length ? on.join(", ") : "Not on any avatar"}</strong>
              <span class="muted small">Week {Math.floor(pr * TERM_WEEKS)}</span>
            </div>
            <div class="bar"><div style="width:{Math.min(100, pr * 100)}%"></div></div>
            <span class="muted small">{p.partner.name ? `with ${shownName(d, p.partner.name, p.partner.userId)}` : "partner unknown"}{p.timeline.pausedAtMs ? " · paused" : ""}</span>
          </div>
          {#if aid}<button class="small soft" onclick={() => viewPregnancy(p)}>Open</button>{/if}
        </li>
      {/each}
      {#each waiting as w}
        {@const wa = ownerAvatar(d, w.avatarId)}
        {@const wg = w.avatarId ? d.groups?.[w.avatarId] : undefined}
        <li>
          {#if wg}<GroupPic ids={groupMembers(d, wg.id)} size={42} />{:else if wa}<AvatarPic id={wa} size={42} />{:else}<span class="dot-pic" aria-hidden="true">💕</span>{/if}
          <div class="grow">
            <strong>{wg ? "💞 " : ""}{w.avatarId ? ownerName(d, w.avatarId) : "Your avatar"}</strong>
            <div class="muted small">💕 WooHoo'd{w.partner.name ? ` with ${shownName(d, w.partner.name, w.partner.userId)}` : ""} · find out in {humanDuration(Math.max(0, w.revealMs - app.now))}</div>
          </div>
          {#if wa}<button class="small soft" onclick={() => viewPending(w)}>Open</button>{/if}
        </li>
      {/each}
    </ul>

    {#if past.length}
      <button class="link small" onclick={() => { showPast = !showPast; showAllPast = false; }}>{showPast ? "Hide" : "Show"} past pregnancies ({past.length})</button>
      {#if showPast}
        <ul class="list past">
          {#each showAllPast ? past : past.slice(0, PAST_SHOWN) as p (p.id)}
            <li>
              <span class="dot-pic" aria-hidden="true">{p.born ? "🍼" : "🤍"}</span>
              <div class="grow">
                <strong>{p.born ? `${babyWord(p.babyCount).replace("!", "")} born` : "Ended early"}</strong>
                <div class="muted small">
                  {dateFmt.format(p.endedMs ?? 0)}{p.partner.name ? ` · with ${shownName(d, p.partner.name, p.partner.userId)}` : ""}{p.born && d.settings.genderMode === "random" && p.genders.length ? ` · ${genderText(p.genders)}` : ""}
                </div>
              </div>
              <button class="small soft" onclick={() => (reading = p)}>📔 Journal</button>
            </li>
          {/each}
        </ul>
        {#if !showAllPast && past.length > PAST_SHOWN}
          <button class="link small" onclick={() => (showAllPast = true)}>Show all ({past.length})</button>
        {/if}
      {/if}
    {/if}
  </section>
{/if}

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
    list-style: none;
    padding: 0;
    margin: 6px 0 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  li {
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
    margin: 4px 0;
    overflow: hidden;
  }
  .bar div {
    height: 100%;
    background: var(--accent);
    border-radius: 99px;
  }
  .dot-pic {
    width: 42px;
    height: 42px;
    border-radius: 14px;
    background: var(--bg-3);
    display: grid;
    place-items: center;
    font-size: 1.3rem;
    flex-shrink: 0;
  }
  .link {
    background: none;
    color: var(--accent);
    padding: 8px 0 0;
    align-self: flex-start;
  }
  .past li {
    opacity: 0.85;
  }
</style>
