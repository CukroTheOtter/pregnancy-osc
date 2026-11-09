<script lang="ts">
  // Me > Share: a picture of your achievements to copy or save (lib/sharecard.ts draws it).
  import Modal from "./Modal.svelte";
  import Toggle from "./Toggle.svelte";
  import { app, errorToast, toast } from "../lib/app.svelte";
  import { api, saveBlob } from "../lib/api";
  import { drawShareCard } from "../lib/sharecard";
  import { hidesMe } from "../lib/privacy";
  import { ACHIEVEMENTS } from "../lib/achievements";
  import { isStopped } from "../lib/scope";

  let { onclose }: { onclose: () => void } = $props();

  const d = $derived(app.data!);
  const me = $derived(d.instance.local);
  let showName = $state(true);
  let showPicture = $state(true);
  let showWoohoos = $state(true);
  let picture = $state<string | null>(null);
  let blob = $state<Blob | null>(null);
  let url = $state<string | null>(null);

  const canPicture = $derived(!!d.settings.showPictures && !!me?.userId);

  $effect(() => {
    const id = me?.userId;
    if (!d.settings.showPictures || !id) return;
    api.partnerPicture(id).then((p) => (picture = p)).catch(() => (picture = null));
  });

  // Redrawn whenever a tick box or the theme's data changes.
  $effect(() => {
    const born = d.pregnancies.filter((p) => p.born);
    const stats = [
      ...(showWoohoos ? [{ label: "WooHoos", value: d.funLog.length }] : []),
      { label: "Pregnancies", value: d.pregnancies.filter((p) => !isStopped(p)).length },
      { label: "Babies", value: born.reduce((n, p) => n + p.babyCount, 0) },
    ];
    const input = {
      // Privacy mode: no name or picture on the card.
      name: (showName && !hidesMe(d) && me?.name) || "Someone",
      picture: showPicture && !hidesMe(d) ? picture : null,
      unlocked: ACHIEVEMENTS.filter((a) => d.achievements?.[a.id] != null && !d.testAchievements?.includes(a.id)).map((a) => a.id),
      stats,
    };
    let stale = false;
    drawShareCard(input)
      .then((b) => {
        if (stale) return;
        blob = b;
        url = URL.createObjectURL(b);
      })
      .catch(errorToast);
    return () => {
      stale = true;
    };
  });
  $effect(() => {
    const u = url;
    return () => {
      if (u) URL.revokeObjectURL(u);
    };
  });

  async function copy() {
    if (!blob) return;
    try {
      await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
      toast("Copied", "Paste it wherever you like.", "success", 3000);
    } catch {
      toast("Couldn't copy the picture", "Use Download instead.", "info");
    }
  }

  async function download() {
    if (!blob) return;
    try {
      if (await saveBlob("pregnancy-osc-achievements", blob, "png")) toast("Saved", undefined, "success", 3000);
    } catch (e) {
      errorToast(e);
    }
  }
</script>

<Modal title="📤 Share" {onclose}>
  <div class="share">
    {#if url}<img src={url} alt="Your achievements" />{/if}
    <Toggle bind:checked={showName} label="Show my name" hint={me ? "" : "The app learns it from VRChat's log"} />
    <Toggle checked={canPicture && showPicture} disabled={!canPicture} label="Show my picture" hint={canPicture ? "" : "Turn on Show VRChat pictures (Settings > Logs and pictures)"} onchange={(v) => (showPicture = v)} />
    <Toggle bind:checked={showWoohoos} label="Show my WooHoo count" />
    <span class="muted small">Only leaves your PC if you share it. It never shows partners, worlds or avatars.</span>
    <div class="row">
      <button onclick={copy} disabled={!blob}>📋 Copy</button>
      <button class="soft" onclick={download} disabled={!blob}>💾 Download</button>
    </div>
  </div>
</Modal>

<style>
  .share {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  img {
    display: block;
    width: 100%;
    max-width: 440px;
    margin: 0 auto 6px;
  }
  .row {
    gap: 8px;
    margin-top: 4px;
  }
</style>
