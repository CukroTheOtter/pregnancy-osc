<script lang="ts">
  import { onMount } from "svelte";
  import { app, init, toast, type View } from "./lib/app.svelte";
  import { api, inTauri } from "./lib/api";
  import type { Settings as SettingsData } from "./lib/types";
  import { avatarName } from "./lib/scope";
  import Toasts from "./components/Toasts.svelte";
  import ConfirmDialog from "./components/ConfirmDialog.svelte";
  import AvatarPic from "./components/AvatarPic.svelte";
  import LinkInfo from "./components/LinkInfo.svelte";
  import Home from "./views/Home.svelte";
  import Partners from "./views/Partners.svelte";
  import Avatars from "./views/Avatars.svelte";
  import Settings from "./views/Settings.svelte";
  import Me from "./views/Me.svelte";

  onMount(() => {
    init();
  });

  // Light/dark, following the system unless chosen.
  let systemDark = $state(typeof matchMedia !== "undefined" && matchMedia("(prefers-color-scheme: dark)").matches);
  onMount(() => {
    const mq = matchMedia("(prefers-color-scheme: dark)");
    const on = (e: MediaQueryListEvent) => (systemDark = e.matches);
    mq.addEventListener("change", on);
    return () => mq.removeEventListener("change", on);
  });
  $effect(() => {
    const mode = app.data?.settings.theme.mode ?? "system";
    const root = document.documentElement;
    root.dataset.palette = "strawberry";
    root.dataset.theme = mode === "dark" || (mode !== "light" && systemDark) ? "dark" : "light";
  });

  const TABS: { id: View; label: string; icon: string }[] = [
    { id: "home", label: "Home", icon: "🏡" },
    { id: "me", label: "Me", icon: "🏆" },
    { id: "partners", label: "Partners", icon: "💞" },
    { id: "avatars", label: "Avatars", icon: "🎭" },
    { id: "settings", label: "Settings", icon: "⚙️" },
  ];

  // Hidden debug mode: tap the title 5 times within 3 seconds.
  let taps: number[] = [];
  async function tapTitle() {
    const now = Date.now();
    taps = [...taps.filter((t) => now - t < 3000), now];
    if (taps.length < 5 || !app.data) return;
    taps = [];
    const on = !app.data.settings.debug;
    const s = { ...($state.snapshot(app.data.settings) as SettingsData), debug: on };
    await api.saveSettings(s);
    app.data.settings.debug = on;
    toast(on ? "🛠 Debug mode on" : "Debug mode off", on ? "Find it at the bottom of Settings." : undefined, "info", 4000);
    if (on) app.view = "settings";
  }

  const status = $derived(app.status);
  const wearing = $derived(app.data?.currentAvatarId ? avatarName(app.data, app.data.currentAvatarId) : null);
  // Switching avatars: the app is waiting for the new one (and keeps a hidden belly hidden).
  const hidden = $derived(!!status?.hideBelly && (app.data?.syncValues?.[status.hideBelly] ?? 0) > 0.5);
</script>

<div class="shell">
  <header class="top">
    <img src="/icon.svg" alt="" width="30" height="30" />
    <div class="titles">
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
      <strong class="title" onclick={tapTitle}>Pregnancy OSC</strong>
      {#if wearing}
        <button class="wearing" onclick={() => (app.view = "avatars")} title="Your avatars">
          {#if app.data?.settings.showPictures && app.data.currentAvatarId}<AvatarPic id={app.data.currentAvatarId} size={16} />{:else}👗{/if}
          {wearing}
        </button>
      {/if}
      {#if status?.avatarLoading}
        <span class="switching">⏳ {hidden ? "Loading avatar · keeping your belly hidden 🙈" : "Loading avatar…"}</span>
      {/if}
    </div>
    <span class="grow"></span>
    <!-- Hover (or focus) shows how it's linked; click opens Settings. -->
    <div class="link">
      <button class="link-btn" onclick={() => (app.view = "settings")} aria-label="Link details (opens Settings)">
        {#if !inTauri}
          <span class="pill warn">Preview</span>
        {:else if status?.oscActive && status.vrchatRunning}
          <span class="pill ok"><span class="dot"></span>Linked</span>
        {:else}
          <span class="pill off"><span class="dot"></span>{status?.vrchatRunning ? "No OSC" : "Waiting for VRChat"}</span>
        {/if}
      </button>
      {#if status}
        <div class="link-pop" role="tooltip"><LinkInfo {status} /></div>
      {/if}
    </div>
  </header>

  <nav class="tabs" aria-label="Main">
    {#each TABS as t}
      <button
        class="tab"
        class:on={app.view === t.id}
        aria-current={app.view === t.id ? "page" : undefined}
        onclick={() => (app.view = t.id)}
      >
        <span aria-hidden="true">{t.icon}</span>
        {t.label}
      </button>
    {/each}
  </nav>

  <main>
    {#if !app.ready || !app.data}
      <div class="loading">Warming up…</div>
    {:else if app.view === "me"}
      <Me />
    {:else if app.view === "partners"}
      <Partners />
    {:else if app.view === "avatars"}
      <Avatars />
    {:else if app.view === "settings"}
      <Settings />
    {:else}
      <Home />
    {/if}
  </main>
</div>

<ConfirmDialog />
<Toasts />

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .link {
    position: relative;
  }
  .link-btn {
    background: none;
    border: 0;
    padding: 0;
    cursor: pointer;
    font: inherit;
  }
  .link-pop {
    display: none;
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    z-index: 50;
    width: max-content;
    max-width: min(320px, calc(100vw - 32px));
    padding: 10px 12px;
    border-radius: 12px;
    background: var(--bg-2);
    border: 1px solid var(--line);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.25);
  }
  .link:hover .link-pop,
  .link:focus-within .link-pop {
    display: block;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 16px 4px;
    max-width: 592px;
    width: 100%;
    margin: 0 auto;
  }
  .titles {
    display: flex;
    flex-direction: column;
    line-height: 1.15;
    min-width: 0;
  }
  .title {
    font-size: 1.1rem;
    user-select: none;
  }
  .wearing {
    background: none;
    color: var(--muted);
    padding: 0;
    font-size: 0.8rem;
    font-weight: 700;
    justify-content: flex-start;
    /* Long avatar names ("… （OwO Edition）") wrap instead of being cut off. */
    white-space: normal;
    text-align: left;
    overflow-wrap: anywhere;
  }
  .switching {
    color: var(--muted);
    font-size: 0.75rem;
  }
  .wearing:hover {
    color: var(--accent);
    transform: none;
  }
  .tabs {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 4px;
    max-width: 560px;
    width: calc(100% - 32px);
    margin: 6px auto 10px;
    background: var(--bg-3);
    border-radius: 999px;
    padding: 4px;
  }
  .tab {
    background: transparent;
    color: var(--text);
    padding: 7px 4px;
    font-size: 0.85rem;
    gap: 4px;
  }
  .tab.on {
    background: var(--bg-2);
    box-shadow: var(--shadow);
  }
  .tab:hover {
    transform: none;
  }
  main {
    flex: 1;
    overflow-y: auto;
  }
  .loading {
    display: grid;
    place-items: center;
    height: 100%;
    color: var(--muted);
    font-weight: 700;
  }
  @media (max-width: 420px) {
    .tab span {
      display: none;
    }
  }
</style>
