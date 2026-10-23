<script lang="ts">
  import XrayBelly from "../components/XrayBelly.svelte";
  import { app, currentPregnancy, errorToast } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { babyViews } from "../lib/look";
  import { sizeOf } from "../lib/reveals";
  import { progress, week } from "../lib/timeline";
  import { saveSvgAsPng } from "../lib/share";
  import type { JournalEntry } from "../lib/types";

  const p = $derived(currentPregnancy());
  let onlyThis = $state(true);
  let text = $state("");
  let title = $state("");
  let image = $state<string | null>(null);
  let card = $state<SVGSVGElement | null>(null);

  const entries = $derived(
    (app.data?.journal ?? []).filter((j) => !onlyThis || !p || j.pregnancyId === p.id).sort((a, b) => b.timeMs - a.timeMs),
  );
  const dateFmt = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });
  const icons: Record<string, string> = { note: "📝", milestone: "🎀", reveal: "💫", event: "🌟" };

  async function pickImage(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (!f) return;
    // Shrink so the save file stays small.
    const bmp = await createImageBitmap(f);
    const scale = Math.min(1, 900 / Math.max(bmp.width, bmp.height));
    const c = document.createElement("canvas");
    c.width = Math.round(bmp.width * scale);
    c.height = Math.round(bmp.height * scale);
    c.getContext("2d")!.drawImage(bmp, 0, 0, c.width, c.height);
    image = c.toDataURL("image/jpeg", 0.85);
  }

  async function add() {
    if (!text.trim() && !image) return;
    try {
      await api.saveJournalEntry({ id: "", timeMs: 0, pregnancyId: p?.id ?? null, kind: "note", title: title.trim() || "Note", text: text.trim(), image } as JournalEntry);
      text = "";
      title = "";
      image = null;
    } catch (e) {
      errorToast(e);
    }
  }

  async function remove(id: string) {
    if (confirm("Delete this entry?")) await api.deleteJournalEntry(id);
  }

  const w = $derived(p ? week(p.timeline, app.now) : 0);
  const views = $derived(p && app.data ? babyViews(p, app.data) : []);
</script>

<div class="view">
  <header class="row">
    <h1 class="grow">📔 Journal</h1>
    {#if p}<label class="row small"><input type="checkbox" bind:checked={onlyThis} /> Only {p.name}</label>{/if}
  </header>

  <div class="layout">
    <section class="stack">
      <div class="card stack">
        <input bind:value={title} placeholder="Title (optional)" maxlength="80" aria-label="Title" />
        <textarea bind:value={text} placeholder="How are you feeling today?" aria-label="Note"></textarea>
        {#if image}<img src={image} alt="Attached" class="preview" />{/if}
        <div class="row">
          <label class="btn soft small" style="cursor:pointer">📷 Add picture<input type="file" accept="image/*" onchange={pickImage} hidden /></label>
          {#if image}<button class="small ghost" onclick={() => (image = null)}>Remove picture</button>{/if}
          <span class="grow"></span>
          <button onclick={add} disabled={!text.trim() && !image}>Add to journal</button>
        </div>
      </div>

      {#each entries as e (e.id)}
        <article class="card entry {e.kind}">
          <div class="row">
            <span class="ico">{icons[e.kind] ?? "📝"}</span>
            <strong class="grow">{e.title}</strong>
            <span class="muted small">{dateFmt.format(e.timeMs)}</span>
            <button class="icon soft small" onclick={() => remove(e.id)} aria-label="Delete entry">✕</button>
          </div>
          {#if e.text}<p class="text">{e.text}</p>{/if}
          {#if e.image}<img src={e.image} alt="" class="preview" />{/if}
        </article>
      {:else}
        <p class="muted">Nothing here yet. Milestones and scans are added on their own.</p>
      {/each}
    </section>

    {#if p}
      <aside class="card share">
        <h3>Share card</h3>
        <svg bind:this={card} viewBox="0 0 360 460" class="cardsvg" role="img" aria-label="Share card">
          <defs>
            <linearGradient id="share-bg" x1="0" y1="0" x2="1" y2="1">
              <stop offset="0" stop-color="var(--bg-3)" />
              <stop offset="1" stop-color="var(--accent-2)" />
            </linearGradient>
          </defs>
          <rect width="360" height="460" rx="28" fill="url(#share-bg)" />
          <g transform="translate(55 30) scale(0.625)">
            <XrayBelly progress={progress(p.timeline, app.now)} babies={views.map((v) => v.look)} egg={p.kind === "egg"} />
          </g>
          <text x="180" y="365" text-anchor="middle" font-family="Nunito, sans-serif" font-size="26" font-weight="900" fill="var(--text)">{p.name}</text>
          <text x="180" y="395" text-anchor="middle" font-family="Nunito, sans-serif" font-size="16" fill="var(--text)">Week {Math.floor(w)} · about the size of {sizeOf(w, p.kind === "egg")}</text>
          <text x="180" y="425" text-anchor="middle" font-family="Nunito, sans-serif" font-size="12" fill="var(--muted)">{p.babyCount > 1 ? `${p.babyCount} little ones` : "One little one"} · Preg Sys Companion</text>
        </svg>
        <button onclick={() => card && saveSvgAsPng(card, 360, 460, `${p!.name}-week-${Math.floor(w)}`).catch(errorToast)}>Save as image</button>
      </aside>
    {/if}
  </div>
</div>

<style>
  .layout { display: grid; grid-template-columns: minmax(0, 1fr) 320px; gap: 18px; margin-top: 10px; align-items: start; }
  @media (max-width: 900px) { .layout { grid-template-columns: 1fr; } }
  .entry.milestone { border-color: var(--accent); }
  .entry.reveal { border-color: var(--accent-2); }
  .ico { font-size: 1.2rem; }
  .text { white-space: pre-wrap; margin: 8px 0 0; }
  .preview { max-width: 100%; max-height: 320px; border-radius: 12px; margin-top: 8px; display: block; }
  .share { display: flex; flex-direction: column; gap: 10px; position: sticky; top: 16px; }
  .cardsvg { width: 100%; height: auto; }
</style>
