<script lang="ts">
  // Eggs on Home: laid clutches keeping warm until they hatch (hatchMs). The
  // hatching itself happens in pregsys-core (actions::take_hatched), even
  // after the app was closed; "Hatch now" only moves the time up.
  import { app, errorToast, toast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { eggWord, isIncubating } from "../lib/fertility";
  import { shownName } from "../lib/privacy";
  import { humanDuration } from "../lib/timeline";
  import type { Pregnancy } from "../lib/types";

  const clutches = $derived((app.data?.pregnancies ?? []).filter(isIncubating).sort((a, b) => (a.hatchMs ?? 0) - (b.hatchMs ?? 0)));
  let busy = $state<string | null>(null);

  /** How far along the incubation is (laid at endedMs, hatches at hatchMs). */
  function done(p: Pregnancy): number {
    const from = p.endedMs ?? app.now;
    const to = p.hatchMs ?? app.now;
    return to <= from ? 1 : Math.min(1, Math.max(0, (app.now - from) / (to - from)));
  }

  async function hatch(p: Pregnancy) {
    busy = p.id;
    try {
      await api.hatchEggs(p.id);
      toast(p.babyCount > 1 ? "🐣 Your eggs are hatching!" : "🐣 Your egg is hatching!", undefined, "success", 6000);
    } catch (e) {
      errorToast(e);
    } finally {
      busy = null;
    }
  }
</script>

{#each clutches as p (p.id)}
  {@const partner = shownName(app.data, p.partner.name, p.partner.userId)}
  {@const left = (p.hatchMs ?? app.now) - app.now}
  <section class="card eggs">
    <div class="top">
      <span class="nest" aria-hidden="true">{"🥚".repeat(Math.min(p.babyCount, 4))}</span>
      <div class="grow">
        <strong>{eggWord(p.babyCount).replace("!", "")} keeping warm</strong>
        <span class="muted small">{p.name}{partner ? ` · with ${partner}` : ""}</span>
      </div>
    </div>
    <div class="bar" role="progressbar" aria-label="Until they hatch" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(done(p) * 100)}>
      <div class="fill" style="width:{done(p) * 100}%"></div>
    </div>
    <div class="row">
      <span class="small muted grow">{left > 0 ? `🐣 Hatching in ${humanDuration(left)}` : "🐣 Hatching any moment"}</span>
      <button class="small soft" onclick={() => hatch(p)} disabled={busy === p.id}>Hatch now</button>
    </div>
  </section>
{/each}

<style>
  .eggs {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .grow {
    flex: 1;
    display: flex;
    flex-direction: column;
  }
  .nest {
    font-size: 1.6rem;
    letter-spacing: -4px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .bar {
    height: 10px;
    border-radius: 999px;
    background: var(--bg-3);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--egg-line, var(--accent));
    border-radius: 999px;
    transition: width 0.4s;
  }
</style>
