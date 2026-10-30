<script lang="ts">
  // "Resting holds off labor": SteamVR has said you're resting for a while
  // only because you're below your saved height, yet your avatar has been
  // standing. Offers to fix the height (never done by itself).
  import { app } from "../lib/app.svelte";
  import { forgetHeight, setHeightNow } from "../lib/posture";

  let busy = $state(false);

  async function setNow() {
    busy = true;
    await setHeightNow();
    busy = false;
  }
</script>

{#if app.status?.vrHeightDoubt}
  <div class="doubt small">
    <span>🤔 SteamVR thinks you're sitting, but your avatar's been standing for 10 minutes. If you're up, your saved height may be off.</span>
    <div class="row">
      <button class="small soft" onclick={setNow} disabled={busy}>📏 Set height now</button>
      <button class="small soft" onclick={forgetHeight}>Forget it</button>
    </div>
  </div>
{/if}

<style>
  .doubt {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--warn) 18%, transparent);
    text-align: left;
  }
</style>
