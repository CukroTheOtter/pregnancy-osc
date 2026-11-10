<script lang="ts">
  import Modal from "./Modal.svelte";
  import { app } from "../lib/app.svelte";

  let checked = $state(false);
  // A new question starts unticked.
  $effect(() => {
    if (app.ask) checked = false;
  });
</script>

{#if app.ask}
  {@const a = app.ask}
  <Modal title={a.title} onclose={() => a.resolve(false, checked)}>
    {#if a.text}<p class="muted">{a.text}</p>{/if}
    {#if a.check}<label class="row small muted" style="gap:6px;margin-top:10px"><input type="checkbox" bind:checked />{a.check}</label>{/if}
    <div class="row" style="justify-content:flex-end;margin-top:14px">
      {#if a.cancel !== null}<button class="soft" onclick={() => a.resolve(false, checked)}>{a.cancel ?? "Cancel"}</button>{/if}
      <button class:danger={a.danger} onclick={() => a.resolve(true, checked)}>{a.ok}</button>
    </div>
  </Modal>
{/if}
