<script lang="ts">
  import PersonPic from "../components/PersonPic.svelte";
  import AvatarPic from "../components/AvatarPic.svelte";
  import { app } from "../lib/app.svelte";
  import { avatarName, partnerStats } from "../lib/scope";
  import { hasSystem } from "../lib/avatars";
  import { humanDuration } from "../lib/timeline";
  import { ACHIEVEMENTS } from "../lib/achievements";

  const d = $derived(app.data!);
  const me = $derived(d.instance.local);
  const dateFmt = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", year: "numeric" });

  const stats = $derived.by(() => {
    const now = app.now;
    const lengths = d.pregnancies.map((p) => (p.endedMs ?? now) - p.timeline.startMs);
    const born = d.pregnancies.filter((p) => p.born);
    const fav = [...partnerStats(d)].sort((a, b) => b.pregnancies - a.pregnancies || b.fun - a.fun)[0];
    return {
      woohoos: d.funLog.length,
      pregnancies: d.pregnancies.length,
      babies: born.reduce((n, p) => n + p.babyCount, 0),
      pregnantMs: lengths.reduce((n, ms) => n + Math.max(0, ms), 0),
      longestMs: Math.max(0, ...born.map((p) => (p.endedMs ?? now) - p.timeline.startMs)),
      favourite: fav?.name ?? null,
      avatars: Object.values(d.avatars).filter((a) => hasSystem(a) === true).length,
    };
  });

  const unlocked = $derived(ACHIEVEMENTS.filter((a) => d.achievements?.[a.id] != null).length);
</script>

<div class="page">
  <section class="card hero">
    {#if me}
      <PersonPic name={me.name} userId={me.userId} size={64} />
    {:else}
      <div class="pic" aria-hidden="true">🤰</div>
    {/if}
    <div class="grow">
      <strong class="name">{me?.name ?? "You"}</strong>
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
    {#if stats.favourite}<p class="muted small fav">💞 Favourite partner: <strong>{stats.favourite}</strong></p>{/if}
  </section>

  <section class="card">
    <div class="row">
      <h2 class="grow">🏆 Achievements</h2>
      <span class="pill">{unlocked} / {ACHIEVEMENTS.length}</span>
    </div>
    <div class="achievements">
      {#each ACHIEVEMENTS as a (a.id)}
        {@const at = d.achievements?.[a.id]}
        <div class="achievement" class:locked={at == null}>
          <span class="emoji" aria-hidden="true">{at == null ? "🔒" : a.emoji}</span>
          <span class="text">
            <strong>{a.title}</strong>
            <span class="muted small">{at == null ? a.hint : `${a.hint} · ${dateFmt.format(at)}`}</span>
          </span>
        </div>
      {/each}
    </div>
  </section>
</div>

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
  .achievement.locked {
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
