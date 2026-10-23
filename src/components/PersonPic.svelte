<script lang="ts" module>
  // One fetch per person per session; the app also caches on disk.
  const cache = new Map<string, Promise<string | null>>();
</script>

<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { api } from "../lib/api";

  let { name, userId = null, size = 46 }: { name: string; userId?: string | null; size?: number } = $props();

  let src = $state<string | null>(null);
  const enabled = $derived(!!app.data?.settings.showPictures);

  $effect(() => {
    src = null;
    if (!enabled || !userId) return;
    const id = userId;
    if (!cache.has(id)) {
      // Only a real picture is remembered: failures and "nothing yet" are
      // retried next time (e.g. right after pictures are turned on).
      const p = api
        .partnerPicture(id)
        .catch(() => null)
        .then((url) => {
          if (!url) cache.delete(id);
          return url;
        });
      cache.set(id, p);
    }
    let alive = true;
    cache.get(id)!.then((url) => alive && (src = url));
    return () => (alive = false);
  });
</script>

<span class="pic" style="width:{size}px;height:{size}px;font-size:{size * 0.42}px" aria-hidden="true">
  {#if src}
    <img {src} alt="" />
  {:else}
    {name.charAt(0).toUpperCase()}
  {/if}
</span>

<style>
  .pic {
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-weight: 900;
    color: #fff;
    background: linear-gradient(135deg, var(--accent), var(--accent-2));
    flex-shrink: 0;
    overflow: hidden;
    box-shadow: 0 0 0 2px var(--bg-2), 0 0 0 4px var(--line);
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
