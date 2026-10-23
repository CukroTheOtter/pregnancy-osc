<script lang="ts">
  import type { Status } from "../lib/types";

  // How the app is linked to VRChat: who it found, which ports, and whether VRChat is talking back.
  let { status }: { status: Status | null } = $props();

  function ago(ms: number | null): string {
    if (ms == null) return "never";
    const s = Math.max(0, Math.round((Date.now() - ms) / 1000));
    if (s < 2) return "just now";
    if (s < 60) return `${s} s ago`;
    if (s < 3600) return `${Math.round(s / 60)} min ago`;
    return `${Math.round(s / 3600)} h ago`;
  }

  const quiet = $derived(!!status?.oscActive && (status.lastHeardMs == null || Date.now() - status.lastHeardMs > 60_000));
</script>

{#if status}
  <dl class="link-info">
    <dt>VRChat</dt>
    <dd>{status.vrchatRunning ? "running" : "not running"}</dd>
    <dt>Mode</dt>
    <dd>{status.oscquery ? "automatic (OSCQuery)" : "fixed ports"}</dd>
    <dt>Sending to</dt>
    <dd>{status.sendTarget ?? "-"}</dd>
    <dt>Listening on</dt>
    <dd>
      {status.recvPort ?? "-"}{#if status.backupPort && status.backupPort !== status.recvPort}&nbsp;and {status.backupPort}{/if}
    </dd>
    {#if status.oscquery}
      <dt>App's OSCQuery</dt>
      <dd>{status.queryPort ? `127.0.0.1:${status.queryPort}` : "-"}</dd>
    {/if}
    <dt>VRChat's OSCQuery</dt>
    <dd>{status.vrchatQuery ?? "not found yet"}</dd>
    <dt>Heard from VRChat</dt>
    <dd>{ago(status.lastHeardMs)}</dd>
  </dl>
  {#if quiet && status.vrchatRunning}
    <p class="hint">VRChat isn't sending to the app (menu buttons won't work). Toggle OSC off and on in VRChat's Action Menu.</p>
  {/if}
{/if}

<style>
  .link-info {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 2px 12px;
    margin: 0;
    font-size: 0.8rem;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    overflow-wrap: anywhere;
  }
  .hint {
    margin: 6px 0 0;
    font-size: 0.8rem;
    color: var(--warn);
  }
</style>
