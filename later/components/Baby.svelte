<svelte:options namespace="svg" />

<script lang="ts">
  import { EARS, TAILS, WINGS, type Part } from "../assets/art/parts";
  import { mixColors } from "../lib/species";
  import type { BabyLook } from "../lib/look";

  let {
    look = {},
    stage = 1,
    awake = false,
    sonar = false,
    x = 0,
    y = 0,
    scale = 1,
    rotate = 0,
  }: { look?: BabyLook; stage?: number; awake?: boolean; sonar?: boolean; x?: number; y?: number; scale?: number; rotate?: number } = $props();

  const uid = Math.random().toString(36).slice(2, 8);
  const base = $derived(sonar ? "#d9d9d9" : (look.furColor ?? "#f3cdb8"));
  const light = $derived(sonar ? "#f2f2f2" : mixColors(base, "#ffffff", 0.55));
  const dark = $derived(sonar ? "#9a9a9a" : mixColors(base, "#2a1a20", 0.35));
  const inner = $derived(sonar ? "#eeeeee" : "#ffc4d6");
  const line = $derived(sonar ? "#5a5a5a" : mixColors(base, "#3a2030", 0.6));
  const ears = $derived(look.ears ? EARS[look.ears] : undefined);
  const tail = $derived(look.tail ? TAILS[look.tail] : undefined);
  const wings = $derived(look.wings ? WINGS[look.wings] : undefined);
  const isBean = $derived(stage < 0.2);
  const beanScale = $derived(0.35 + stage * 2.2);
</script>

{#snippet part(p: Part, mirror: boolean)}
  <g transform="translate({mirror ? -p.at.x : p.at.x} {p.at.y}) scale({mirror ? -1 : 1} 1) rotate({p.at.r})">
    {#each p.dark ?? [] as d}<path {d} fill={dark} />{/each}
    {#each p.main ?? [] as d}<path {d} fill={base} stroke={line} stroke-width="1.2" stroke-opacity="0.35" />{/each}
    {#each p.inner ?? [] as d}<path {d} fill={inner} opacity="0.85" />{/each}
    {#each p.stroke ?? [] as d}<path {d} fill="none" stroke={base} stroke-width="6" stroke-linecap="round" />{/each}
  </g>
{/snippet}

<g transform="translate({x} {y}) rotate({rotate}) scale({scale})" class:sonar>
  {#if isBean}
    <!-- Early weeks: a little bean with a flickering heart. -->
    <g transform="scale({beanScale})">
      <path d="M-10 -14 C6 -20 18 -6 14 8 C10 20 -8 22 -14 10 C-18 2 -20 -10 -10 -14 Z" fill={sonar ? "#e0e0e0" : "#ffd3df"} stroke={sonar ? "#888" : "#f19bb5"} stroke-width="1.5" />
      <circle cx="-2" cy="-4" r="5" fill={sonar ? "#cfcfcf" : "#ffe6ee"} />
      <path class="heart" d="M2 6 c-2 -3 -6 -1 -4 2 l4 4 l4 -4 c2 -3 -2 -5 -4 -2z" fill={sonar ? "#fff" : "#ff6f9a"} />
    </g>
  {:else}
    <!-- Behind: wings, tail, back ears -->
    {#if wings && (wings.main?.length || wings.dark?.length)}
      {@render part(wings, false)}
      {@render part(wings, true)}
    {/if}
    {#if tail && (tail.main || tail.stroke || tail.dark)}
      {@render part(tail, false)}
    {/if}
    {#if ears?.behind}
      {@render part(ears, false)}
      {@render part(ears, true)}
    {/if}

    <!-- Body: curled up -->
    <ellipse cx="0" cy="20" rx="30" ry="25" fill={base} stroke={line} stroke-width="1.5" stroke-opacity="0.4" />
    {#if look.pattern === "belly" || look.covering === "scales"}
      <ellipse cx="0" cy="24" rx="17" ry="15" fill={light} />
    {/if}
    {#if look.pattern === "stripes"}
      {#each [-14, 0, 14] as sx}
        <path d="M{sx - 4} 2 q4 6 0 12" fill="none" stroke={dark} stroke-width="3" stroke-linecap="round" opacity="0.7" />
      {/each}
    {/if}
    {#if look.pattern === "spots"}
      {#each [[-16, 12, 3.5], [14, 8, 3], [18, 26, 2.5], [-10, 32, 3], [6, 36, 2]] as [sx, sy, r]}
        <circle cx={sx} cy={sy} {r} fill={look.covering === "scales" ? dark : light} opacity="0.85" />
      {/each}
    {/if}
    {#if look.covering === "scales"}
      {#each [[-20, 18], [-12, 30], [20, 18], [12, 32]] as [sx, sy]}
        <path d="M{sx - 4} {sy} q4 -5 8 0" fill="none" stroke={dark} stroke-width="1.5" opacity="0.6" />
      {/each}
    {/if}
    <!-- Little hands and feet -->
    <ellipse cx="-14" cy="10" rx="7" ry="6" fill={look.pattern === "socks" ? light : base} stroke={line} stroke-opacity="0.3" />
    <ellipse cx="14" cy="10" rx="7" ry="6" fill={look.pattern === "socks" ? light : base} stroke={line} stroke-opacity="0.3" />
    <ellipse cx="-16" cy="40" rx="9" ry="6" fill={look.pattern === "socks" ? light : base} stroke={line} stroke-opacity="0.3" />
    <ellipse cx="16" cy="40" rx="9" ry="6" fill={look.pattern === "socks" ? light : base} stroke={line} stroke-opacity="0.3" />

    <!-- Head -->
    <circle cx="0" cy="-14" r="26" fill={base} stroke={line} stroke-width="1.5" stroke-opacity="0.4" />
    {#if look.covering === "fur"}
      <path d="M-26 -8 l-5 3 l5 2 M26 -8 l5 3 l-5 2" fill={base} stroke={line} stroke-opacity="0.35" stroke-width="1.2" />
    {/if}
    {#if look.covering === "feathers"}
      <path d="M-4 -40 q2 -10 6 -2 q3 -8 6 0 q-6 4 -12 2z" fill={dark} />
    {/if}
    {#if look.pattern === "mask"}
      <path d="M-20 -18 q10 -8 20 -2 q10 -6 20 2 q-4 10 -20 8 q-16 2 -20 -8z" fill={dark} opacity="0.55" />
    {/if}
    {#if ears && !ears.behind}
      {@render part(ears, false)}
      {@render part(ears, true)}
    {/if}

    <!-- Face -->
    {#if look.muzzle === "snout"}
      <ellipse cx="0" cy="-4" rx="11" ry="8" fill={light} />
      <ellipse cx="0" cy="-8" rx="4" ry="3" fill={line} />
    {:else if look.muzzle === "beak"}
      <path d="M-6 -8 L6 -8 L0 2 Z" fill={sonar ? "#bbb" : "#f2b54a"} stroke={line} stroke-opacity="0.4" />
    {:else if look.muzzle === "short"}
      <path d="M-3 -8 h6 l-3 3z" fill={sonar ? "#888" : "#e98aa6"} />
      <path d="M-5 -3 q2.5 3 5 0 q2.5 3 5 0" fill="none" stroke={line} stroke-width="1.5" stroke-linecap="round" />
    {:else}
      <path d="M-4 -2 q4 4 8 0" fill="none" stroke={line} stroke-width="1.8" stroke-linecap="round" />
    {/if}
    {#if awake}
      <circle cx="-10" cy="-18" r="4.5" fill={sonar ? "#555" : (look.eyeColor ?? "#5a3a2a")} />
      <circle cx="10" cy="-18" r="4.5" fill={sonar ? "#555" : (look.eyeColor ?? "#5a3a2a")} />
      <circle cx="-8.5" cy="-19.5" r="1.5" fill="#fff" />
      <circle cx="11.5" cy="-19.5" r="1.5" fill="#fff" />
    {:else}
      <path d="M-15 -18 q5 -5 10 0" fill="none" stroke={line} stroke-width="2" stroke-linecap="round" />
      <path d="M5 -18 q5 -5 10 0" fill="none" stroke={line} stroke-width="2" stroke-linecap="round" />
    {/if}
    {#if !sonar}
      <ellipse cx="-16" cy="-8" rx="4.5" ry="3" fill="#ff9fb8" opacity="0.55" />
      <ellipse cx="16" cy="-8" rx="4.5" ry="3" fill="#ff9fb8" opacity="0.55" />
    {/if}
    {#if look.sex === "girl"}
      <g transform="translate(-16 -36) rotate(-15)">
        <path d="M0 0 l-9 -6 v12z M0 0 l9 -6 v12z" fill={sonar ? "#ccc" : "#ff7aa8"} />
        <circle r="3" fill={sonar ? "#aaa" : "#ff4f8b"} />
      </g>
    {:else if look.sex === "boy"}
      <path d="M-2 -40 q4 -8 8 -2 q-4 -1 -4 4" fill="none" stroke={dark} stroke-width="2.5" stroke-linecap="round" />
    {/if}
  {/if}
</g>

<style>
  .heart {
    animation: beat 0.45s ease-in-out infinite alternate;
    transform-box: fill-box;
    transform-origin: center;
  }
  @keyframes beat {
    from { transform: scale(0.85); }
    to { transform: scale(1.15); }
  }
  .sonar {
    filter: blur(0.3px);
  }
</style>
