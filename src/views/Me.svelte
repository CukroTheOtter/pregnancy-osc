<script lang="ts">
  import { myShownName, shownName } from "../lib/privacy";
  import PersonPic from "../components/PersonPic.svelte";
  import AvatarPic from "../components/AvatarPic.svelte";
  import ShareCard from "../components/ShareCard.svelte";
  import { app } from "../lib/app.svelte";
  import { avatarName, isStopped, partnerStats } from "../lib/scope";
  import { hasSystem } from "../lib/avatars";
  import { humanDuration } from "../lib/timeline";
  import { ACHIEVEMENTS, TIERS } from "../lib/achievements";

  const d = $derived(app.data!);
  const me = $derived(d.instance.local);
  const dateFmt = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", year: "numeric" });

  const stats = $derived.by(() => {
    const now = app.now;
    const lengths = d.pregnancies.map((p) => (p.endedMs ?? now) - p.timeline.startMs);
    const born = d.pregnancies.filter((p) => p.born);
    const fav = partnerStats(d).filter((s) => s.fun || s.pregnancies).sort((a, b) => b.pregnancies - a.pregnancies || b.fun - a.fun)[0];
    return {
      woohoos: d.funLog.length,
      pregnancies: d.pregnancies.filter((p) => !isStopped(p)).length,
      babies: born.reduce((n, p) => n + p.babyCount, 0),
      pregnantMs: lengths.reduce((n, ms) => n + Math.max(0, ms), 0),
      longestMs: Math.max(0, ...born.map((p) => (p.endedMs ?? now) - p.timeline.startMs)),
      favourite: fav ? shownName(d, fav.name, fav.userId) : null,
      avatars: Object.values(d.avatars).filter((a) => hasSystem(a) === true).length,
    };
  });

  const isTest = (id: string) => !!d.testAchievements?.includes(id);
  // Ones unlocked in debug mode don't count.
  const unlocked = $derived(ACHIEVEMENTS.filter((a) => d.achievements?.[a.id] != null && !isTest(a.id)).length);
  let sharing = $state(false);
  const got = (id: string) => d.achievements?.[id] != null && !isTest(id);
  let show = $state<"all" | "unlocked" | "locked">("all");
  const listed = $derived(show === "all" ? ACHIEVEMENTS : ACHIEVEMENTS.filter((a) => got(a.id) === (show === "unlocked")));
</script>

<div class="page">
  <section class="card hero">
    {#if me}
      <PersonPic name={me.name} userId={me.userId} size={64} />
    {:else}
      <div class="pic" aria-hidden="true">🤰</div>
    {/if}
    <div class="grow">
      <strong class="name">{myShownName(d, me?.name ?? null) ?? "You"}</strong>
      {#if d.currentAvatarId}
        <div class="wearing muted small">
          {#if d.settings.showPictures}<AvatarPic id={d.currentAvatarId} size={18} />{:else}👗{/if}
          <span>Wearing {avatarName(d, d.currentAvatarId)}</span>
        </div>
      {:else}
        <div class="muted small">{me ? "Not wearing an avatar the app knows yet" : "Your name shows up once the app reads VRChat's log"}</div>
      {/if}
    </div>
  </section>

  <section class="card">
    <h2>📊 Your stats</h2>
    <div class="nums">
      <div><strong>{stats.woohoos}</strong><span>💕 WooHoos</span></div>
      <div><strong>{stats.pregnancies}</strong><span>🤰 pregnancies</span></div>
      <div><strong>{stats.babies}</strong><span>👶 babies</span></div>
      <div><strong>{stats.pregnantMs ? humanDuration(stats.pregnantMs) : "–"}</strong><span>⏳ time pregnant</span></div>
      <div><strong>{stats.longestMs ? humanDuration(stats.longestMs) : "–"}</strong><span>🐢 longest to birth</span></div>
      <div><strong>{stats.avatars}</strong><span>🎭 avatars</span></div>
    </div>
    {#if stats.favourite}<p class="muted small fav">💞 Most pregnancies with: <strong>{stats.favourite}</strong></p>{/if}
  </section>

  <section class="card">
    <div class="row">
      <h2 class="grow">🏆 Achievements</h2>
      <span class="pill">{unlocked} / {ACHIEVEMENTS.length}</span>
      <button class="small soft" onclick={() => (sharing = true)}>📤 Share</button>
    </div>
    <div class="bar" role="progressbar" aria-valuemin="0" aria-valuemax={ACHIEVEMENTS.length} aria-valuenow={unlocked} aria-label="Achievements unlocked">
      <span style="width:{(unlocked / ACHIEVEMENTS.length) * 100}%"></span>
    </div>
    <div class="segmented filter">
      <button class:on={show === "all"} onclick={() => (show = "all")}>All</button>
      <button class:on={show === "unlocked"} onclick={() => (show = "unlocked")}>Unlocked</button>
      <button class:on={show === "locked"} onclick={() => (show = "locked")}>Locked</button>
    </div>
    <div class="achievements">
      {#each listed as a (a.id)}
        {@const at = d.achievements?.[a.id]}
        <div class="achievement" class:locked={at == null} class:test={isTest(a.id)}>
          <span class="emoji" aria-hidden="true">{at == null ? "🔒" : a.emoji}</span>
          <span class="text">
            <strong>{a.title}</strong>
            <span class="tier tier-{a.tier}" title={TIERS[a.tier]}>{"⭐".repeat(a.tier)} {TIERS[a.tier]}</span>
            {#if isTest(a.id)}<span class="tier" title="Unlocked in debug mode, so it doesn't count">🛠 Test</span>{/if}
            <span class="muted small">{at == null ? (a.secret ?? a.hint) : `${a.hint} · ${dateFmt.format(at)}`}</span>
          </span>
        </div>
      {:else}
        <p class="muted small none">{show === "unlocked" ? "None yet. Your first one is close! 💕" : "You've got them all! 🎉"}</p>
      {/each}
    </div>
  </section>
</div>

{#if sharing}<ShareCard onclose={() => (sharing = false)} />{/if}

<style>
  .page {
    max-width: 560px;
    margin: 0 auto;
    padding: 6px 16px 40px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .hero {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .pic {
    font-size: 2.6rem;
  }
  .name {
    font-size: 1.25rem;
  }
  .wearing {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 2px;
  }
  h2 {
    margin: 0 0 10px;
  }
  .nums {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
    text-align: center;
  }
  .nums div {
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border-radius: 14px;
    padding: 8px 4px;
  }
  .nums strong {
    font-size: 1.2rem;
  }
  .nums span {
    font-size: 0.78rem;
    color: var(--muted);
  }
  .fav {
    margin: 10px 0 0;
  }
  .bar {
    height: 8px;
    border-radius: 999px;
    background: var(--bg-3);
    overflow: hidden;
    margin: 0 0 10px;
  }
  .bar span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
    transition: width 0.4s ease;
  }
  .filter {
    margin-bottom: 10px;
  }
  .none {
    grid-column: 1 / -1;
    text-align: center;
    margin: 6px 0;
  }
  .achievements {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 8px;
  }
  .achievement {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 14px;
    background: color-mix(in srgb, var(--accent) 14%, var(--bg));
  }
  .achievement.locked,
  .achievement.test {
    background: var(--bg);
    opacity: 0.6;
  }
  .emoji {
    font-size: 1.6rem;
  }
  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .tier {
    font-size: 0.72rem;
    font-weight: 700;
    color: var(--muted);
  }
  .tier-2 {
    color: var(--tier-uncommon);
  }
  .tier-3 {
    color: var(--tier-rare);
  }
  .tier-4 {
    color: var(--tier-epic);
  }
  .tier-5 {
    color: var(--tier-legendary);
  }
  @media (max-width: 440px) {
    .nums,
    .achievements {
      grid-template-columns: repeat(2, 1fr);
    }
    .achievements {
      grid-template-columns: 1fr;
    }
  }
</style>
