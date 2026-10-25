<script lang="ts">
  import AvatarPic from "./AvatarPic.svelte";
  import { app, homeAvatarId, currentPregnancy } from "../lib/app.svelte";
  import { owner, ownerAvatar, ownerName, pregnancyIdOf } from "../lib/scope";
  import { babyWord, genderText } from "../lib/fertility";
  import { progress, humanDuration, TERM_WEEKS } from "../lib/timeline";
  import type { Pregnancy } from "../lib/types";

  const d = $derived(app.data!);
  const shown = $derived(currentPregnancy());
  let showPast = $state(false);
  let showAllPast = $state(false);
  /** Past pregnancies shown before "Show all". */
  const PAST_SHOWN = 5;

  /** Avatars a pregnancy is on (the ones linked to it, or all in global mode). */
  function avatarsOf(p: Pregnancy): string[] {
    if (d.settings.syncScope === "global") return p.id === d.activePregnancyId ? ["every avatar"] : [];
    return Object.values(d.avatars)
      .filter((a) => pregnancyIdOf(d, owner(d, a.id)) === p.id)
      .map((a) => a.name ?? "Unnamed avatar");
  }

  function firstAvatarOf(p: Pregnancy): string | null {
    if (d.settings.syncScope === "global") return null;
    const a = Object.values(d.avatars).find((x) => pregnancyIdOf(d, owner(d, x.id)) === p.id);
    return a?.id ?? null;
  }

  const active = $derived(d.pregnancies.filter((p) => !p.endedMs && p.id !== shown?.id));
  const waiting = $derived(d.waiting.filter((w) => w.avatarId !== owner(d, homeAvatarId())));
  const past = $derived(d.pregnancies.filter((p) => p.endedMs).sort((a, b) => (b.endedMs ?? 0) - (a.endedMs ?? 0)));
  const dateFmt = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", year: "numeric" });

  function open(p: Pregnancy) {
    const id = firstAvatarOf(p);
    if (id) app.viewAvatarId = id === d.currentAvatarId ? null : id;
    document.querySelector("main")?.scrollTo({ top: 0, behavior: "smooth" });
  }
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
        <li>
          {#if aid}<AvatarPic id={aid} size={42} />{:else}<span class="dot-pic" aria-hidden="true">🤰</span>{/if}
          <div class="grow">
            <div class="row top">
              <strong>{on.length ? on.join(", ") : "Not on any avatar"}</strong>
              <span class="muted small">Week {Math.floor(pr * TERM_WEEKS)}</span>
            </div>
            <div class="bar"><div style="width:{Math.min(100, pr * 100)}%"></div></div>
            <span class="muted small">{p.partner.name ? `with ${p.partner.name}` : "partner unknown"}{p.timeline.pausedAtMs ? " · paused" : ""}</span>
          </div>
          {#if aid}<button class="small soft" onclick={() => open(p)}>Open</button>{/if}
        </li>
      {/each}
      {#each waiting as w}
        {@const wa = ownerAvatar(d, w.avatarId)}
        <li>
          {#if wa}<AvatarPic id={wa} size={42} />{:else}<span class="dot-pic" aria-hidden="true">💕</span>{/if}
          <div class="grow">
            <strong>{w.avatarId ? ownerName(d, w.avatarId) : "Your avatar"}</strong>
            <div class="muted small">💕 WooHoo'd{w.partner.name ? ` with ${w.partner.name}` : ""} · find out in {humanDuration(Math.max(0, w.revealMs - app.now))}</div>
          </div>
          {#if wa}<button class="small soft" onclick={() => { app.viewAvatarId = wa === d.currentAvatarId ? null : wa; }}>Open</button>{/if}
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
                  {dateFmt.format(p.endedMs ?? 0)}{p.partner.name ? ` · with ${p.partner.name}` : ""}{p.born && d.settings.genderMode === "random" && p.genders.length ? ` · ${genderText(p.genders)}` : ""}
                </div>
              </div>
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
