<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { partnerStats, personFromStats, personKey } from "../lib/scope";
  import type { Person } from "../lib/types";
  import PersonPic from "./PersonPic.svelte";

  // A dropdown with a search box: people from the instance where it happened
  // first, then past partners. Typing a new name offers to use it.
  let {
    candidates,
    selected = null,
    onpick,
  }: { candidates: Person[]; selected?: string | null; onpick: (person: Person | null, name: string | null) => void } = $props();

  let query = $state("");
  let open = $state(false);
  let active = $state(0);
  let input: HTMLInputElement | undefined = $state();

  type Option = { person: Person; tag: string };
  const all = $derived.by(() => {
    const seen = new Set<string>();
    const out: Option[] = [];
    const add = (person: Person, tag: string) => {
      const k = personKey(person.name, person.userId);
      if (!person.name || seen.has(k)) return;
      seen.add(k);
      out.push({ person, tag });
    };
    candidates.forEach((p) => add(p, "in the instance"));
    if (app.data) partnerStats(app.data).forEach((s) => add(personFromStats(s), "past partner"));
    return out;
  });

  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return (q ? all.filter((o) => o.person.name.toLowerCase().includes(q)) : all).slice(0, 30);
  });
  const typedNew = $derived(query.trim() && !all.some((o) => o.person.name.toLowerCase() === query.trim().toLowerCase()) ? query.trim() : "");
  const count = $derived(shown.length + (typedNew ? 1 : 0));

  function pick(i: number) {
    if (i < shown.length) onpick(shown[i].person, null);
    else if (typedNew) onpick(null, typedNew);
    query = "";
    open = false;
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      open = true;
      active = Math.min(active + 1, count - 1);
      e.preventDefault();
    } else if (e.key === "ArrowUp") {
      active = Math.max(active - 1, 0);
      e.preventDefault();
    } else if (e.key === "Enter" && open && count) {
      pick(active);
      e.preventDefault();
    } else if (e.key === "Escape") {
      open = false;
    }
  }

  $effect(() => {
    query;
    active = 0;
  });
</script>

<div class="combo" onfocusout={(e) => { if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) open = false; }}>
  <input
    bind:this={input}
    bind:value={query}
    onfocus={() => (open = true)}
    oninput={() => (open = true)}
    onkeydown={onkey}
    placeholder={selected ? `${selected} (search to change)` : "Search or type a name"}
    maxlength="60"
    role="combobox"
    aria-expanded={open}
    aria-controls="person-list"
    aria-autocomplete="list"
    aria-label="Partner"
  />
  <button class="caret" tabindex="-1" aria-label="Show the list" onclick={() => { open = !open; input?.focus(); }}>▾</button>
  {#if open}
    <ul class="list" id="person-list" role="listbox">
      {#each shown as o, i (personKey(o.person.name, o.person.userId))}
        <li role="option" aria-selected={i === active}>
          <button class="opt" class:active={i === active} class:on={o.person.name === selected} onmousedown={(e) => e.preventDefault()} onclick={() => pick(i)}>
            <PersonPic name={o.person.name} userId={o.person.userId} size={26} />
            <span class="grow">{o.person.name}</span>
            <span class="tag">{o.tag}</span>
          </button>
        </li>
      {/each}
      {#if typedNew}
        <li role="option" aria-selected={active === shown.length}>
          <button class="opt" class:active={active === shown.length} onmousedown={(e) => e.preventDefault()} onclick={() => pick(shown.length)}>
            <span class="plus" aria-hidden="true">＋</span>
            <span class="grow">Use “{typedNew}”</span>
          </button>
        </li>
      {/if}
      {#if !count}
        <li class="empty">No one from that instance or past partners yet. Type a name.</li>
      {/if}
    </ul>
  {/if}
</div>

<style>
  .combo {
    position: relative;
    margin: 8px 0;
  }
  .combo input {
    width: 100%;
    padding-right: 36px;
  }
  .caret {
    position: absolute;
    right: 4px;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    color: var(--muted);
    padding: 4px 8px;
  }
  .list {
    position: absolute;
    left: 0;
    right: 0;
    top: calc(100% + 4px);
    z-index: 30;
    list-style: none;
    margin: 0;
    padding: 4px;
    max-height: 260px;
    overflow-y: auto;
    background: var(--bg-2);
    border: 1px solid var(--line);
    border-radius: 14px;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.25);
  }
  .opt {
    width: 100%;
    justify-content: flex-start;
    gap: 10px;
    background: none;
    color: var(--text);
    padding: 6px 8px;
    border-radius: 10px;
    font-weight: 600;
  }
  .opt .grow {
    text-align: left;
  }
  .opt.active {
    background: var(--bg-3);
  }
  .opt.on {
    color: var(--accent);
  }
  .tag {
    color: var(--muted);
    font-size: 0.75rem;
    font-weight: 400;
  }
  .plus {
    width: 26px;
    text-align: center;
    color: var(--accent);
  }
  .empty {
    padding: 8px;
    color: var(--muted);
    font-size: 0.85rem;
  }
</style>
