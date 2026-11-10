<script lang="ts">
  import { personPicture } from "./PersonPic.svelte";
  import { app } from "../lib/app.svelte";
  import { isPrivate } from "../lib/privacy";
  import type { Card } from "../lib/journal";
  import { rng } from "../lib/scanstats";
  import { testBuddyPicture } from "../lib/testbuddy";

  // A little polaroid for journal moments (late-night kicks, …).
  // Plain SVG attributes only, so it can be copied or saved like a scan.
  let { card, svg = $bindable(null) }: { card: Card; svg?: SVGSVGElement | null } = $props();

  const W = 400;
  const H = 340;
  const HAND = "'Comic Sans MS', 'Segoe Print', cursive";
  const EMOJI = "'Segoe UI Emoji', 'Apple Color Emoji', 'Noto Color Emoji', sans-serif";

  const LOOK: Record<Card["kind"], { from: string; to: string; ink: string }> = {
    "partner-kicks": { from: "#ffc9de", to: "#e5d4ff", ink: "#6d2f57" },
    "well-travelled": { from: "#cdeeff", to: "#d9f7e4", ink: "#25536b" },
    "night-owl": { from: "#1d2150", to: "#3b2a6b", ink: "#2a2350" },
  };
  const look = $derived(LOOK[card.kind]);

  // The partner's profile picture (a data URL, so it's kept when saved).
  let face = $state<string | null>(null);
  $effect(() => {
    face = null;
    const id = card.person?.userId;
    const made = testBuddyPicture(id ?? null);
    if (made) {
      face = made;
      return;
    }
    // Privacy mode: no face at all (a saved polaroid can't be un-blurred if it was never there).
    if (!id || !app.data?.settings.showPictures || isPrivate(app.data)) return;
    let alive = true;
    personPicture(id).then((url) => alive && (face = url));
    return () => (alive = false);
  });
  const tilt = $derived((rng(card.seed)() - 0.5) * 6);
  const stars = $derived.by(() => {
    const r = rng(card.seed + 1);
    return Array.from({ length: 14 }, () => ({ x: 60 + r() * 280, y: 50 + r() * 170, s: 0.6 + r() * 1.6 }));
  });
</script>

<svg bind:this={svg} xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width={W} height={H} class="moment" role="img" aria-label={card.caption}>
  <defs>
    <clipPath id="mc-face-{card.seed}"><circle cx="200" cy="140" r="74" /></clipPath>
    <linearGradient id="mc-ring-{card.seed}" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#f59ac0" />
      <stop offset="1" stop-color="#c9b6ff" />
    </linearGradient>
    <linearGradient id="mc-{card.seed}" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color={look.from} />
      <stop offset="1" stop-color={look.to} />
    </linearGradient>
  </defs>
  <rect width={W} height={H} fill="#f3e9dc" />
  <g transform="rotate({tilt} 200 170)">
    <!-- The polaroid. -->
    <rect x="62" y="22" width="276" height="296" rx="6" fill="#000" opacity="0.12" transform="translate(4 5)" />
    <rect x="62" y="22" width="276" height="296" rx="6" fill="#fffdf8" />
    <rect x="80" y="40" width="240" height="200" rx="3" fill="url(#mc-{card.seed})" />
    {#if card.kind === "night-owl"}
      {#each stars as s}<circle cx={s.x} cy={s.y} r={s.s} fill="#fff6c8" opacity="0.9" />{/each}
    {/if}
    {#if card.person}
      <!-- Their profile picture (or their initial), with a little 💞🦶 on the corner. -->
      <circle cx="200" cy="140" r="80" fill="url(#mc-ring-{card.seed})" />
      {#if face}
        <image href={face} x="126" y="66" width="148" height="148" preserveAspectRatio="xMidYMid slice" clip-path="url(#mc-face-{card.seed})" />
      {:else}
        <text x="200" y="166" text-anchor="middle" font-family="'Segoe UI', system-ui, sans-serif" font-size="78" font-weight="900" fill="#fff">{isPrivate(app.data) ? "?" : card.person.name.charAt(0).toUpperCase()}</text>
      {/if}
      <text x="286" y="226" text-anchor="middle" font-family={EMOJI} font-size="34">{card.emoji}</text>
    {:else}
      <text x="200" y="{card.emoji.length > 2 ? 158 : 165}" text-anchor="middle" font-family={EMOJI} font-size="72">{card.emoji}</text>
    {/if}
    <text x="200" y="278" text-anchor="middle" font-family={HAND} font-size="19" font-weight="700" fill={look.ink}>{card.caption}</text>
    {#if card.sub}<text x="200" y="302" text-anchor="middle" font-family={HAND} font-size="13" fill={look.ink} opacity="0.75">{card.sub}</text>{/if}
    <!-- A bit of tape. -->
    <rect x="170" y="12" width="60" height="20" fill="#ffe7a3" opacity="0.8" transform="rotate(-4 200 22)" />
  </g>
</svg>

<style>
  .moment {
    display: block;
    width: 100%;
    max-width: 420px;
    height: auto;
    border-radius: 14px;
  }
</style>
