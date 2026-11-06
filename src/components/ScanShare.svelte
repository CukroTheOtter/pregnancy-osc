<script lang="ts">
  // A journal picture (a scan or a moment's polaroid), big, to copy or save
  // (a scan's text is stamped in).
  import MomentCard from "./MomentCard.svelte";
  import Modal from "./Modal.svelte";
  import Toggle from "./Toggle.svelte";
  import Ultrasound from "./Ultrasound.svelte";
  import { app, errorToast, toast } from "../lib/app.svelte";
  import { saveBlob } from "../lib/api";
  import { scanMeta, type Card, type Scan } from "../lib/journal";
  import { svgToPng } from "../lib/scanimage";
  import type { Pregnancy } from "../lib/types";

  let { p, scan = null, card = null, timeMs, title, onclose }: { p: Pregnancy; scan?: Scan | null; card?: Card | null; timeMs: number; title: string; onclose: () => void } = $props();

  let showName = $state(true);
  let svg = $state<SVGSVGElement | null>(null);
  let busy = $state(false);
  const meta = $derived(app.data && scan ? scanMeta(app.data, p, timeMs, scan.week, showName) : null);
  const hasName = $derived(!!app.data?.instance.local?.name);

  async function picture(): Promise<Blob | null> {
    if (!svg) return null;
    busy = true;
    try {
      return await svgToPng(svg);
    } catch (e) {
      errorToast(e);
      return null;
    } finally {
      busy = false;
    }
  }

  async function copy() {
    const b = await picture();
    if (!b) return;
    try {
      await navigator.clipboard.write([new ClipboardItem({ "image/png": b })]);
      toast("Copied", "Paste it wherever you like.", "success", 3000);
    } catch {
      toast("Couldn't copy the picture", "Use Download instead.", "info");
    }
  }

  async function download() {
    const b = await picture();
    if (!b) return;
    try {
      const name = scan ? `ultrasound-week-${scan.week}${scan.variant ? `-${scan.variant}` : ""}` : `journal-${card?.kind ?? "moment"}`;
      if (await saveBlob(name, b, "png")) toast("Saved", undefined, "success", 3000);
    } catch (e) {
      errorToast(e);
    }
  }
</script>

<Modal {title} {onclose}>
  <div class="share">
    {#if scan}
      <Ultrasound {scan} {meta} bind:svg />
      <Toggle checked={hasName && showName} disabled={!hasName} label="Show my name on it" hint={hasName ? "" : "The app learns it from VRChat's log"} onchange={(v) => (showName = v)} />
    {:else if card}
      <MomentCard {card} bind:svg />
    {/if}
    <span class="muted small">Only leaves your PC if you share it. Copy and Download stamp what you see into the picture.</span>
    <div class="row">
      <button onclick={copy} disabled={busy}>📋 Copy</button>
      <button class="soft" onclick={download} disabled={busy}>💾 Download</button>
    </div>
  </div>
</Modal>

<style>
  .share {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .row {
    gap: 8px;
    margin-top: 4px;
  }
</style>
