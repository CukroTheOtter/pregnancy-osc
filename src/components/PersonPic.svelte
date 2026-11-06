<script lang="ts" module>
  import { api as moduleApi } from "../lib/api";
  // One fetch per person per session; the app also caches on disk.
  const cache = new Map<string, Promise<string | null>>();

  /** Someone's VRChat profile picture as a data URL (null: none). Shared with the journal's polaroids. */
  export function personPicture(userId: string): Promise<string | null> {
    if (!cache.has(userId)) {
      // Only a real picture is remembered: failures and "nothing yet" are
      // retried next time (e.g. right after pictures are turned on).
      const p = moduleApi
        .partnerPicture(userId)
        .catch(() => null)
        .then((url) => {
          if (!url) cache.delete(userId);
          return url;
        });
      cache.set(userId, p);
    }
    return cache.get(userId)!;
  }
</script>

<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { testBuddyPicture } from "../lib/testbuddy";

  let { name, userId = null, size = 46 }: { name: string; userId?: string | null; size?: number } = $props();

  let src = $state<string | null>(null);
  const enabled = $derived(!!app.data?.settings.showPictures);

  $effect(() => {
    src = null;
    // Debug: test buddies' pictures are made up, so they show even with pictures off.
    const made = testBuddyPicture(userId);
    if (made) {
      src = made;
      return;
    }
    if (!enabled || !userId) return;
    let alive = true;
    personPicture(userId).then((url) => alive && (src = url));
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
