<script lang="ts" module>
  import type { AvatarPreview } from "../lib/types";
  // One lookup per avatar per session; the app also caches on disk.
  const cache = new Map<string, Promise<AvatarPreview | null>>();

  /** Shared with the "add avatar" form so it can fill in the name. */
  export function lookupAvatar(id: string, preview: (id: string) => Promise<AvatarPreview | null>): Promise<AvatarPreview | null> {
    if (!cache.has(id)) {
      // Only a real answer is remembered: failures and "pictures are off" are retried later.
      cache.set(
        id,
        preview(id)
          .catch(() => null)
          .then((p) => {
            if (!p) cache.delete(id);
            return p;
          }),
      );
    }
    return cache.get(id)!;
  }
</script>

<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { api } from "../lib/api";

  let { id, size = 46 }: { id: string; size?: number } = $props();

  let src = $state<string | null>(null);
  const enabled = $derived(!!app.data?.settings.showPictures);

  $effect(() => {
    src = null;
    if (!enabled || !id.startsWith("avtr_")) return;
    let alive = true;
    lookupAvatar(id, api.avatarPreview).then((p) => alive && (src = p?.picture ?? null));
    return () => (alive = false);
  });
</script>

<span class="pic" style="width:{size}px;height:{size}px;font-size:{size * 0.5}px" aria-hidden="true">
  {#if src}
    <img {src} alt="" />
  {:else}
    🎭
  {/if}
</span>

<style>
  .pic {
    border-radius: 14px;
    display: grid;
    place-items: center;
    background: var(--bg-3);
    flex-shrink: 0;
    overflow: hidden;
    box-shadow: 0 0 0 2px var(--line);
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
