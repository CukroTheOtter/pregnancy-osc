<script lang="ts">
  import { onMount } from "svelte";
  import AvatarPic, { lookupAvatar } from "../components/AvatarPic.svelte";
  import AvatarPicker from "../components/AvatarPicker.svelte";
  import Toggle from "../components/Toggle.svelte";
  import { app, confirmAsk, errorToast, toast } from "../lib/app.svelte";
  import { api, inTauri } from "../lib/api";
  import { avatarName, groupMembers, pendingFor, pregnancyFor } from "../lib/scope";
  import { matches, seenText, shelfOf } from "../lib/avatars";
  import { progress, TERM_WEEKS } from "../lib/timeline";
  import { FEATURES, FIXED, PARAM_INFO, REQUIRED, hasParam, syncLabel } from "../lib/params";
  import { HOLE_TYPES, socketCounts, socketsFrom, typeOf } from "../lib/sps";
  import type { HoleType } from "../lib/types";
  import type { AvatarGroup, AvatarProfile, Mapping, Settings } from "../lib/types";

  let mapping = $state<Mapping | null>(null);
  let open = $state<string | null>(null);
  let draftNames = $state<Record<string, string>>({});
  let adding = $state(false);
  let query = $state("");
  // Group being added to (its id), or "new" while making a group.
  let picking = $state<string | null>(null);
  let newGroupName = $state("");
  let newGroupIds = $state<string[]>([]);
  let renaming = $state<string | null>(null);
  // Which avatars VRChat shows publicly (private ones have no thumbnail).
  let isPublic = $state<Record<string, boolean>>({});
  $effect(() => {
    if (!d.settings.showPictures) return;
    for (const p of profiles) {
      if (p.id in isPublic) continue;
      lookupAvatar(p.id, api.avatarPreview).then((r) => {
        if (r) isPublic[p.id] = r.public;
      });
    }
  });

  async function setAutoAdd(on: boolean) {
    const s: Settings = { ...($state.snapshot(d.settings) as Settings), autoAddAvatars: on };
    try {
      await api.saveSettings(s);
      app.data!.settings.autoAddAvatars = on;
    } catch (e) {
      errorToast(e);
    }
  }

  async function removeWithout() {
    const ids = shelves.other.map((p) => p.id);
    const n = ids.length;
    const back = d.settings.autoAddAvatars ? " They come back if you wear them again." : "";
    if (await confirmAsk({ title: `Remove ${n} avatar${n === 1 ? "" : "s"}?`, text: `Every avatar without the Pregnancy System is removed from the list.${back}`, ok: "Remove all", danger: true })) {
      try {
        await api.deleteAvatars(ids);
        toast(`Removed ${n} avatar${n === 1 ? "" : "s"}`, undefined, "success");
      } catch (e) {
        errorToast(e);
      }
    }
  }

  async function turnOnPictures() {
    const s: Settings = { ...($state.snapshot(d.settings) as Settings), showPictures: true };
    try {
      await api.saveSettings(s);
      app.data!.settings.showPictures = true;
    } catch (e) {
      errorToast(e);
    }
  }
  let newId = $state("");
  let newName = $state("");
  let looking = $state(false);
  const idOk = $derived(/^avtr_[0-9a-f-]{36}$/i.test(newId.trim()));

  // The avatar you're wearing in VRChat right now (from the live link).
  const wearingId = $derived(app.status?.avatarId ?? app.data?.currentAvatarId ?? null);
  const wearingAdded = $derived(!!wearingId && !!app.data?.avatars[wearingId]);

  /** Fills the form with the avatar you're wearing, if it isn't in the list yet. */
  function useWearing() {
    if (!wearingId || wearingAdded) return;
    newId = wearingId;
    // The log knows the name of the avatar you switched into last.
    newName = d.instance.local?.avatarName ?? "";
    lookup();
  }

  function openAdd() {
    adding = true;
    newId = "";
    newName = "";
    useWearing();
  }

  /** Fill in the name from VRChat's public page (if pictures are on). */
  async function lookup() {
    if (!idOk || newName.trim() || !d.settings.showPictures) return;
    looking = true;
    try {
      const p = await lookupAvatar(newId.trim(), api.avatarPreview);
      if (p?.name && !newName.trim()) newName = p.name;
      if (p && !p.public) toast("Private avatar", "VRChat doesn't show private avatars publicly, so type a name yourself.", "info", 7000);
    } finally {
      looking = false;
    }
  }

  async function addAvatar() {
    try {
      const p = await api.addAvatar(newId.trim(), newName.trim() || null);
      toast("Avatar added 🎭", p.name ?? undefined, "success");
      adding = false;
      newId = "";
      newName = "";
      open = p.id;
    } catch (e) {
      errorToast(e);
    }
  }

  onMount(async () => {
    try {
      mapping = (await api.getMapping()).mapping;
    } catch {
      /* the parameter list is optional here */
    }
  });

  const d = $derived(app.data!);
  const perAvatar = $derived(d.settings.syncScope === "perAvatar");
  const profiles = $derived(Object.values(d.avatars).sort((a, b) => b.lastSeenMs - a.lastSeenMs));
  const groups = $derived(Object.values(d.groups ?? {}).sort((a, b) => a.name.localeCompare(b.name)));
  const searching = $derived(!!query.trim());
  const results = $derived(searching ? profiles.filter((p) => matches(d, p, query)) : []);

  /** Avatars outside groups (all of them in global mode), sorted onto shelves. */
  const shelves = $derived.by(() => {
    const out = { main: [] as AvatarProfile[], older: [] as AvatarProfile[], other: [] as AvatarProfile[] };
    for (const p of profiles) {
      if (perAvatar && p.groupId && d.groups?.[p.groupId]) continue;
      out[shelfOf(d, p, app.now)].push(p);
    }
    return out;
  });

  function statusOf(id: string | null): string {
    const p = pregnancyFor(d, id);
    if (p) {
      const w = Math.floor(progress(p.timeline, app.now) * TERM_WEEKS);
      return `🤰 Week ${w}${p.partner.name ? ` · with ${p.partner.name}` : ""}`;
    }
    if (pendingFor(d, id)) return "💕 WooHoo'd · waiting to find out…";
    return "Not pregnant";
  }

  async function setScope(per: boolean) {
    const s: Settings = { ...($state.snapshot(d.settings) as Settings), syncScope: per ? "perAvatar" : "global" };
    try {
      await api.saveSettings(s);
    } catch (e) {
      errorToast(e);
    }
  }

  async function save(p: AvatarProfile, change: (x: AvatarProfile) => void) {
    const copy = structuredClone($state.snapshot(p)) as AvatarProfile;
    change(copy);
    try {
      await api.saveAvatar(copy);
    } catch (e) {
      errorToast(e);
    }
  }

  function toggle(p: AvatarProfile) {
    open = open === p.id ? null : p.id;
    draftNames = { ...(p.paramNames ?? {}) };
  }

  /** Group moves can be refused (two pregnancies); say why in a dialog. */
  async function refused(e: unknown) {
    await confirmAsk({ title: "Can't move it yet", text: e instanceof Error ? e.message : String(e), ok: "OK", cancel: "Close" });
  }

  async function moveTo(p: AvatarProfile, groupId: string | null) {
    try {
      await api.setAvatarGroup(p.id, groupId);
      if (groupId) toast("Added to group 💞", `${avatarName(d, p.id)} now shares ${d.groups[groupId]?.name}'s pregnancy.`, "success");
    } catch (e) {
      await refused(e);
    }
  }

  async function chooseGroup(p: AvatarProfile, value: string) {
    if (value === "__new") {
      startNewGroup([p.id]);
      return;
    }
    await moveTo(p, value || null);
  }

  function startNewGroup(ids: string[] = []) {
    query = "";
    picking = "new";
    newGroupName = "";
    newGroupIds = ids;
  }

  async function createGroup() {
    try {
      const id = await api.createGroup(newGroupName, newGroupIds);
      toast("Group made 💞", d.groups[id]?.name, "success");
      picking = null;
    } catch (e) {
      picking = null;
      await refused(e);
    }
  }

  async function saveGroupName(g: AvatarGroup, name: string) {
    renaming = null;
    if (name.trim() && name.trim() !== g.name) await api.renameGroup(g.id, name);
  }

  async function deleteGroup(g: AvatarGroup) {
    const members = groupMembers(d, g.id);
    const keeper = members[0] ? avatarName(d, members[0]) : null;
    const text = pregnancyFor(d, g.id) || pendingFor(d, g.id)
      ? `Its avatars go back to their own pregnancies. The group's pregnancy stays with ${keeper ?? "the avatar worn last"}.`
      : "Its avatars go back to their own pregnancies.";
    if (await confirmAsk({ title: `Remove the group ${g.name}?`, text, ok: "Remove group", danger: true })) await api.deleteGroup(g.id);
  }

  async function saveNames(p: AvatarProfile) {
    const clean = Object.fromEntries(Object.entries(draftNames).filter(([k, v]) => !FIXED.includes(k) && v.trim()).map(([k, v]) => [k, v.trim()]));
    await save(p, (x) => (x.paramNames = clean));
    toast("Saved", "This avatar now uses its own parameter names.", "success");
  }

  async function forget(p: AvatarProfile) {
    if (await confirmAsk({ title: `Forget ${avatarName(d, p.id)}?`, text: "Its profile is removed. It comes back if you wear it again (without its settings).", ok: "Forget", danger: true })) {
      await api.deleteAvatar(p.id);
      open = null;
    }
  }

  const on = (k: string) => !!mapping?.parameters[k];

  /** One line under the "What this avatar can do" heading. */
  function canDo(p: AvatarProfile): string {
    if (!p.params.length) return "not checked yet";
    const missing = REQUIRED.filter((k) => on(k) && hasParam(mapping, p, k) === false);
    if (missing.length) return `missing ${missing.map((k) => PARAM_INFO[k]?.label ?? k).join(", ")}`;
    const feats = FEATURES.filter((f) => f.keys.some((k) => on(k)));
    const has = feats.filter((f) => f.keys.filter(on).every((k) => hasParam(mapping, p, k) === true));
    return `ready · ${has.length} of ${feats.length} features`;
  }

  function openHome(id: string) {
    app.viewAvatarId = id === d.currentAvatarId ? null : id;
    app.view = "home";
  }
</script>

{#snippet mark(h: boolean | null)}
  {#if h === true}<span class="ok-mark">✓</span>{:else if h === false}<span class="no-mark">–</span>{:else}<span class="muted">·</span>{/if}
{/snippet}

{#snippet prow(p: AvatarProfile, k: string, label: string, tag: string | null)}
  <label class="prow">
    <span class="pname">
      {@render mark(hasParam(mapping, p, k))}
      {label}{#if tag}<span class="muted small tag">{tag}</span>{/if}
    </span>
    {#if FIXED.includes(k)}
      <span class="muted small">{mapping?.parameters[k]?.name} (fixed)</span>
    {:else}
      <input bind:value={draftNames[k]} placeholder={mapping?.parameters[k]?.name || "(off)"} />
    {/if}
  </label>
{/snippet}

{#snippet row(p: AvatarProfile, showStatus: boolean)}
  <div class="avatar" class:open={open === p.id}>
    <div class="row">
      <AvatarPic id={p.id} size={44} />
      <div class="grow">
        <strong>{p.name ?? "Unnamed avatar"}</strong>
        {#if p.id === d.currentAvatarId}<span class="pill ok">Wearing</span>{/if}
        {#if d.settings.showPictures && isPublic[p.id] === false}<span class="pill off" title="VRChat doesn't share thumbnails or names for private avatars">🔒 Private</span>{/if}
        <div class="muted small">
          {showStatus ? `${statusOf(p.id)} · ` : ""}{seenText(p.lastSeenMs, app.now)}
        </div>
      </div>
      <button class="small soft" onclick={() => openHome(p.id)}>Open</button>
      <button class="small ghost" onclick={() => toggle(p)}>{open === p.id ? "Done" : "Edit"}</button>
    </div>

    {#if open === p.id}
      <div class="edit stack">
        <label class="field">Name
          <input value={p.name ?? ""} onchange={(e) => save(p, (x) => ((x.name = (e.target as HTMLInputElement).value.trim() || null), (x.autoName = false)))} />
        </label>

        {#if perAvatar}
          <label class="field">Group
            <select value={p.groupId && d.groups[p.groupId] ? p.groupId : ""} onchange={(e) => chooseGroup(p, (e.target as HTMLSelectElement).value)}>
              <option value="">None (its own pregnancy)</option>
              {#each groups as g (g.id)}<option value={g.id}>💞 {g.name}</option>{/each}
              <option value="__new">+ New group…</option>
            </select>
            <span class="hint">Avatars in a group share one pregnancy and partner.</span>
          </label>
        {/if}

        {#if !socketsFrom(p.params).length}
          <details class="field fold">
            <summary><strong>SPS sockets</strong> <span class="muted small">none found yet</span></summary>
            <span class="hint">{p.params.length ? "This avatar has no SPS sockets (VRCFury SPS adds OGB/Orf/... parameters)." : "Wear this avatar in VRChat so the app can read its sockets."}</span>
          </details>
        {:else}
          {@const socks = socketsFrom(p.params)}
          <details class="field fold">
            <summary><strong>SPS sockets</strong> <span class="muted small">{socks.filter((n) => socketCounts(d.settings, p, n)).length} of {socks.length} count</span></summary>
            <div class="segmented">
              <button class:on={!p.sps?.custom} onclick={() => save(p, (x) => (x.sps = { ...(x.sps ?? { sockets: [], types: {} }), custom: false }))}>Use global hole types</button>
              <button class:on={!!p.sps?.custom} onclick={() => save(p, (x) => (x.sps = { ...(x.sps ?? { sockets: [], types: {} }), custom: true, sockets: x.sps?.sockets?.length ? x.sps.sockets : socks.filter((n) => socketCounts(d.settings, x, n)) }))}>Pick sockets</button>
            </div>
            {#each socks as name}
              <div class="sockrow">
                {#if p.sps?.custom}
                  <input type="checkbox" checked={p.sps.sockets.includes(name)} aria-label="Counts"
                    onchange={(e) => { const on = (e.target as HTMLInputElement).checked; save(p, (x) => (x.sps.sockets = on ? [...x.sps.sockets, name] : x.sps.sockets.filter((n) => n !== name))); }} />
                {:else}
                  <span class="mark" title={socketCounts(d.settings, p, name) ? "Counts" : "Doesn't count"}>{socketCounts(d.settings, p, name) ? "✓" : "–"}</span>
                {/if}
                <span class="grow">{name}</span>
                <select value={typeOf(p, name)} aria-label="Hole type"
                  onchange={(e) => { const t = (e.target as HTMLSelectElement).value as HoleType; save(p, (x) => (x.sps = { ...(x.sps ?? { custom: false, sockets: [] }), types: { ...(x.sps?.types ?? {}), [name]: t } })); }}>
                  {#each HOLE_TYPES as h}<option value={h.id}>{h.emoji} {h.label}</option>{/each}
                </select>
              </div>
            {/each}
            <span class="hint">{p.sps?.custom ? "Only the ticked sockets count on this avatar." : "Counts if its type is on in Settings > Detecting WooHoos. Fix a type if the name is unusual."}</span>
          </details>
        {/if}

        <details class="field fold">
          <summary><strong>What this avatar can do</strong> <span class="muted small">{canDo(p)}</span></summary>
          <span class="hint">{p.params.length ? "What this avatar has (✓) and doesn't (–)." : "Wear it in VRChat once to check its parameters."} If it uses its own parameter names, type them here; leave a name empty to use the shared one from Settings.</span>
          <h4 class="pgroup">Required <span class="muted small">the app can't work without these</span></h4>
          <div class="params">
            {#each REQUIRED.filter(on) as k}{@render prow(p, k, PARAM_INFO[k]?.label ?? k, null)}{/each}
          </div>
          <h4 class="pgroup">Features <span class="muted small">optional, each turns something on</span></h4>
          <div class="params">
            {#each FEATURES.filter((f) => f.keys.some(on)) as f}
              {#if f.keys.length === 1}
                {@const k = f.keys[0]}
                {@render prow(p, k, f.label, PARAM_INFO[k]?.label !== f.label ? PARAM_INFO[k]?.label ?? k : null)}
              {:else}
                <span class="fname">{f.label}</span>
                {#each f.keys.filter(on) as k}{@render prow(p, k, PARAM_INFO[k]?.label ?? k, null)}{/each}
              {/if}
            {/each}
          </div>
          <button class="small soft" style="align-self:flex-start" onclick={() => saveNames(p)}>Save names</button>
          {#if mapping?.sync.length}
            <h4 class="pgroup">Follow you <span class="muted small">your Sync list, carried between avatars</span></h4>
            <div class="params">
              {#each mapping.sync as sp}
                <div class="prow">
                  <span class="pname">{@render mark(p.params.length ? p.params.includes(sp.name) : null)} {syncLabel(sp)}</span>
                </div>
              {/each}
            </div>
          {/if}
        </details>

        <div class="row">
          {#if !inTauri && p.id !== d.currentAvatarId}
            <button class="small ghost" onclick={() => api.simulateAvatar(p.id)}>Preview: wear this</button>
          {/if}
          <span class="grow"></span>
          <button class="small ghost" onclick={() => forget(p)}>Forget avatar</button>
        </div>
      </div>
    {/if}
  </div>
{/snippet}

<div class="page">
  <section class="card">
    <h2>One pregnancy, or one per avatar?</h2>
    <div class="choices">
      <button class="opt" class:on={!perAvatar} onclick={() => setScope(false)}>
        <strong>Same on every avatar</strong>
        <span>One pregnancy follows you everywhere.</span>
      </button>
      <button class="opt" class:on={perAvatar} onclick={() => setScope(true)}>
        <strong>Each avatar has its own</strong>
        <span>Different partners and progress. Put avatars in a group to share.</span>
      </button>
    </div>
  </section>

  <section class="card">
    {#if adding}
      <h2>Add an avatar</h2>
      <p class="muted small">Paste its ID from VRChat (on the avatar's page or in your avatar list). It starts with <code>avtr_</code>.</p>
      {#if wearingId}
        <div class="wearing-note small">
          {#if wearingAdded}
            You're wearing <strong>{avatarName(d, wearingId)}</strong>, and it's already in your list.
          {:else if newId.trim() === wearingId}
            ✨ Filled in with the avatar you're wearing.
          {:else}
            <button class="small soft" onclick={useWearing}>👗 Use the one I'm wearing</button>
          {/if}
        </div>
      {:else}
        <p class="muted small">Not linked to VRChat right now, so paste the ID yourself.</p>
      {/if}
      <label class="field">Avatar ID
        <input bind:value={newId} placeholder="avtr_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx" onblur={lookup} onpaste={() => setTimeout(lookup, 0)} />
        {#if newId.trim() && !idOk}<span class="hint warn-text">That doesn't look like an avatar ID yet.</span>{/if}
      </label>
      <label class="field">Name
        <input bind:value={newName} placeholder={looking ? "Looking it up…" : "What do you call this avatar?"} />
        {#if !d.settings.showPictures}<span class="hint">Turn on "Show VRChat pictures" in Settings to fill this in automatically.</span>{/if}
      </label>
      <div class="row">
        <button onclick={addAvatar} disabled={!idOk}>Add avatar</button>
        <button class="soft" onclick={() => (adding = false)}>Cancel</button>
      </div>
    {:else}
      <div class="row">
        <input class="grow search" type="search" bind:value={query} placeholder="🔍 Search {profiles.length} avatars" aria-label="Search avatars" />
        {#if perAvatar}<button class="small soft" onclick={() => startNewGroup()}>+ Group</button>{/if}
        <button class="small" onclick={openAdd}>+ Avatar</button>
      </div>
      <Toggle checked={d.settings.autoAddAvatars} label="Add avatars I wear" hint={d.settings.autoAddAvatars ? "New avatars show up here when you wear them" : "Only avatars you add yourself (+ Avatar) are listed"} onchange={setAutoAdd} />
      <span class="muted small">Ones you haven't worn in a while, or that don't have the Pregnancy System, fold away at the bottom.</span>
      {#if !d.settings.showPictures}
        <div class="row pics-off">
          <span class="grow small">Thumbnails are off.</span>
          <button class="small soft" onclick={turnOnPictures}>📷 Show VRChat pictures</button>
        </div>
      {/if}
    {/if}
  </section>

  {#if picking === "new"}
    <section class="card stack">
      <h2>💞 New group</h2>
      <p class="muted small">Avatars in a group share one pregnancy and partner. If one of them already has a pregnancy, the group takes it over.</p>
      <label class="field">Name<input bind:value={newGroupName} placeholder="Like “Bunnies” or “Main look”" /></label>
      {#if newGroupIds.length}
        <div class="chips">
          {#each newGroupIds as id (id)}
            <button class="chip" onclick={() => (newGroupIds = newGroupIds.filter((x) => x !== id))} title="Take it out">{avatarName(d, id)} ✕</button>
          {/each}
        </div>
      {/if}
      <AvatarPicker choices={profiles.filter((p) => !newGroupIds.includes(p.id))} onpick={(id) => (newGroupIds = [...newGroupIds, id])} />
      <div class="row">
        <button onclick={createGroup} disabled={!newGroupIds.length}>Make group</button>
        <button class="soft" onclick={() => (picking = null)}>Cancel</button>
      </div>
    </section>
  {/if}

  {#if profiles.length === 0}
    <section class="card empty">
      <div class="big" aria-hidden="true">🎭</div>
      <p class="muted">Avatars show up here when you switch to them in VRChat (with OSC on).</p>
    </section>
  {:else if searching}
    <section class="card">
      <h2>Found {results.length}</h2>
      {#each results as p (p.id)}
        {@render row(p, perAvatar)}
      {:else}
        <p class="muted small">No avatar or group matches “{query.trim()}”.</p>
      {/each}
    </section>
  {:else}
    {#if perAvatar}
      {#each groups as g (g.id)}
        {@const members = groupMembers(d, g.id)}
        <section class="card group">
          <div class="ghead">
            {#if renaming === g.id}
              <!-- svelte-ignore a11y_autofocus -->
              <input class="grow" value={g.name} autofocus aria-label="Group name"
                onblur={(e) => saveGroupName(g, (e.target as HTMLInputElement).value)}
                onkeydown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()} />
            {:else}
              <h2 class="grow">💞 {g.name}</h2>
              <button class="small ghost" onclick={() => (renaming = g.id)}>Rename</button>
              <button class="small ghost" onclick={() => deleteGroup(g)}>Remove</button>
            {/if}
          </div>
          <div class="status">{statusOf(g.id)} <span class="pill">{members.length} {members.length === 1 ? "avatar" : "avatars"}</span></div>
          {#each members as id (id)}
            {@render row(d.avatars[id], false)}
          {/each}
          {#if picking === g.id}
            <AvatarPicker choices={profiles.filter((p) => p.groupId !== g.id)} onpick={(id) => moveTo(d.avatars[id], g.id)} placeholder="Search for an avatar to add…" focus />
            <button class="small soft" style="align-self:flex-start" onclick={() => (picking = null)}>Done adding</button>
          {:else}
            <button class="small soft" style="align-self:flex-start" onclick={() => (picking = g.id)}>+ Add an avatar</button>
          {/if}
        </section>
      {/each}
    {/if}

    {#if shelves.main.length}
      <section class="card">
        <h2>{perAvatar && groups.length ? "On their own" : "Your avatars"}</h2>
        {#each shelves.main as p (p.id)}
          {@render row(p, perAvatar)}
        {/each}
      </section>
    {/if}

    {#if shelves.older.length || shelves.other.length}
      <section class="card folds">
        {#if shelves.older.length}
          <details>
            <summary><strong>🕰 Older</strong> <span class="muted small">{shelves.older.length} not worn in 2 weeks</span></summary>
            {#each shelves.older as p (p.id)}
              {@render row(p, perAvatar)}
            {/each}
          </details>
        {/if}
        {#if shelves.other.length}
          <details>
            <summary><strong>🚫 Without the Pregnancy System</strong> <span class="muted small">{shelves.other.length}</span></summary>
            <div class="row other-head">
              <span class="hint grow">VRChat didn't show any <code>Pregnancy/</code> parameters on these. Only the 15 worn most recently are kept.</span>
              <button class="small soft" onclick={removeWithout}>🗑 Remove all</button>
            </div>
            {#each shelves.other as p (p.id)}
              {@render row(p, perAvatar)}
            {/each}
          </details>
        {/if}
      </section>
    {/if}
  {/if}
</div>

<style>
  .other-head {
    gap: 8px;
    align-items: center;
    margin: 4px 0 6px;
  }
  .page {
    max-width: 560px;
    margin: 0 auto;
    padding: 6px 16px 40px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .choices {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .opt {
    flex-direction: column;
    align-items: flex-start;
    text-align: left;
    white-space: normal;
    gap: 2px;
    background: var(--bg);
    color: var(--text);
    border: 2px solid var(--line);
    border-radius: 16px;
    padding: 12px;
  }
  .opt span {
    font-weight: 600;
    font-size: 0.8rem;
    color: var(--muted);
  }
  .opt.on {
    border-color: var(--accent);
    background: var(--bg-3);
  }
  .search {
    min-width: 0;
  }
  .ghead {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .ghead h2 {
    margin: 0;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .status {
    font-weight: 800;
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
    margin: 4px 0;
  }
  .avatar {
    border-top: 1.5px dashed var(--line);
    padding: 10px 0;
  }
  :global(h2) + .avatar,
  .status + .avatar,
  summary + .avatar,
  .other-head + .avatar {
    border-top: none;
  }
  .edit {
    margin-top: 10px;
  }
  .folds {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .folds summary {
    cursor: pointer;
    padding: 2px 0;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    background: var(--bg-3);
    color: var(--text);
    border: none;
    box-shadow: none;
    border-radius: 99px;
    padding: 4px 12px;
    font-size: 0.85rem;
  }
  .params {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 4px 0 6px;
  }
  .prow {
    display: grid;
    grid-template-columns: 1fr 1.2fr;
    gap: 8px;
    align-items: center;
    font-weight: 600;
    font-size: 0.88rem;
  }
  .pgroup {
    margin: 10px 0 0;
    font-size: 0.9rem;
  }
  .pgroup .small {
    font-weight: 600;
  }
  .fname {
    font-weight: 700;
    font-size: 0.85rem;
    color: var(--muted);
  }
  .tag {
    margin-left: 6px;
  }
  .ok-mark {
    color: var(--ok);
    font-weight: 900;
  }
  .no-mark {
    color: var(--muted);
    font-weight: 900;
  }
  .empty {
    text-align: center;
  }
  .wearing-note {
    background: var(--bg-3);
    border-radius: 12px;
    padding: 6px 10px;
  }
  .fold summary {
    cursor: pointer;
    list-style-position: outside;
    margin-left: 14px;
    padding: 2px 0;
  }
  .fold[open] summary {
    margin-bottom: 6px;
  }
  .sockrow {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 3px 0;
    font-weight: 600;
    font-size: 0.9rem;
  }
  .sockrow select {
    width: 130px;
  }
  .mark {
    width: 16px;
    text-align: center;
    font-weight: 900;
    color: var(--ok);
  }
  .pics-off {
    margin-top: 8px;
    background: var(--bg-3);
    border-radius: 12px;
    padding: 6px 10px;
  }
  .warn-text {
    color: var(--danger);
    font-weight: 700;
  }
  code {
    background: var(--bg-3);
    padding: 1px 6px;
    border-radius: 6px;
  }
  .big {
    font-size: 2.6rem;
  }
  @media (max-width: 440px) {
    .choices,
    .prow {
      grid-template-columns: 1fr;
    }
  }
</style>
