<script lang="ts">
  import Modal from "./Modal.svelte";
  import { app } from "../lib/app.svelte";
</script>

{#if app.ask}
  {@const a = app.ask}
  <Modal title={a.title} onclose={() => a.resolve(false)}>
    {#if a.text}<p class="muted">{a.text}</p>{/if}
    <div class="row" style="justify-content:flex-end;margin-top:14px">
      <button class="soft" onclick={() => a.resolve(false)}>{a.cancel ?? "Cancel"}</button>
      <button class:danger={a.danger} onclick={() => a.resolve(true)}>{a.ok}</button>
    </div>
  </Modal>
{/if}
