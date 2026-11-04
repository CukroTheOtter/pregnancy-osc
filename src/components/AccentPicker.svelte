<script lang="ts">
  // Picks any color, like a painting app: the ring is the color, and the square
  // inside makes it paler or more vivid (left to right) and brighter or darker
  // (top to bottom). `theme.ts` picks text that's readable on whatever you choose.
  import { untrack } from "svelte";
  import { HEX, hexToHsv, hsvToHex } from "../lib/theme";

  let { value, onchange }: { value: string; onchange: (hex: string) => void } = $props();

  const SIZE = 196;
  const RING = 22;
  const SQUARE = 104;

  // The picker keeps its own place while open; it starts from the current color.
  const first = untrack(() => value);
  const start = hexToHsv(HEX.test(first) ? first : "#f59ac0");
  let h = $state(start.h);
  let s = $state(start.s);
  let v = $state(start.v);
  let hex = $state(first);

  const QUICK = ["#f59ac0", "#ff6f91", "#e2557a", "#ffad7a", "#f7c95c", "#9ccf6f", "#6fcfac", "#5fc4d8", "#7fb8f5", "#8f9cff", "#b39bff", "#d58cf0", "#000000", "#808080", "#ffffff"];

  function emit() {
    hex = hsvToHex(h, s, v);
    onchange(hex);
  }

  /** Jumps to a color. A gray or black has no hue of its own, so yours stays put. */
  function setColor(c: string) {
    const next = hexToHsv(c);
    if (next.s > 0 && next.v > 0) h = next.h;
    if (next.v > 0) s = next.s;
    v = next.v;
    hex = c;
    onchange(c);
  }

  function typed() {
    const raw = hex.trim().toLowerCase();
    const c = raw.startsWith("#") ? raw : "#" + raw;
    if (HEX.test(c)) setColor(c);
    else hex = value;
  }

  // Dragging: whichever you pressed first keeps the drag, even if you wander off it.
  let dragging: "ring" | "square" | null = null;
  const half = SIZE / 2;
  const clamp = (x: number) => Math.min(1, Math.max(0, x));

  function place(e: PointerEvent, el: HTMLElement) {
    const box = el.getBoundingClientRect();
    const x = ((e.clientX - box.left) / box.width) * SIZE - half;
    const y = ((e.clientY - box.top) / box.height) * SIZE - half;
    if (dragging === "ring") h = ((Math.atan2(x, -y) * 180) / Math.PI + 360) % 360;
    else {
      s = clamp(x / SQUARE + 0.5);
      v = clamp(0.5 - y / SQUARE);
    }
    emit();
  }

  function down(e: PointerEvent) {
    const el = e.currentTarget as HTMLElement;
    const box = el.getBoundingClientRect();
    const x = ((e.clientX - box.left) / box.width) * SIZE - half;
    const y = ((e.clientY - box.top) / box.height) * SIZE - half;
    if (Math.abs(x) <= SQUARE / 2 && Math.abs(y) <= SQUARE / 2) dragging = "square";
    else if (Math.hypot(x, y) >= half - RING - 2 && Math.hypot(x, y) <= half + 2) dragging = "ring";
    else return;
    el.setPointerCapture(e.pointerId);
    place(e, el);
  }

  function move(e: PointerEvent) {
    if (dragging) place(e, e.currentTarget as HTMLElement);
  }

  function ringKey(e: KeyboardEvent) {
    const d = { ArrowLeft: -5, ArrowDown: -5, ArrowRight: 5, ArrowUp: 5 }[e.key];
    if (d == null) return;
    e.preventDefault();
    h = (h + d + 360) % 360;
    emit();
  }

  function squareKey(e: KeyboardEvent) {
    const d = { ArrowLeft: [-0.02, 0], ArrowRight: [0.02, 0], ArrowUp: [0, 0.02], ArrowDown: [0, -0.02] }[e.key];
    if (!d) return;
    e.preventDefault();
    s = clamp(s + d[0]);
    v = clamp(v + d[1]);
    emit();
  }

  const ringR = half - RING / 2;
  const ringX = $derived(half + ringR * Math.sin((h * Math.PI) / 180));
  const ringY = $derived(half - ringR * Math.cos((h * Math.PI) / 180));
  const dotX = $derived((s - 0.5) * SQUARE + half);
  const dotY = $derived((0.5 - v) * SQUARE + half);
</script>

<div class="picker">
  <div class="top">
    <div class="wheel" style="width:{SIZE}px;height:{SIZE}px" role="presentation" onpointerdown={down} onpointermove={move} onpointerup={() => (dragging = null)} onpointercancel={() => (dragging = null)}>
      <div class="ring" style="--ring:{RING}px"></div>
      <div class="square" style="width:{SQUARE}px;height:{SQUARE}px;background:linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, hsl({h} 100% 50%))"></div>
      <span class="thumb" tabindex="0" role="slider" aria-label="Color" aria-valuemin="0" aria-valuemax="359" aria-valuenow={Math.round(h)} onkeydown={ringKey} style="left:{ringX}px;top:{ringY}px;background:hsl({h} 100% 50%)"></span>
      <span class="thumb" tabindex="0" role="slider" aria-label="Paler or more vivid, brighter or darker" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(v * 100)} onkeydown={squareKey} style="left:{dotX}px;top:{dotY}px;background:{hsvToHex(h, s, v)}"></span>
    </div>
    <div class="side">
      <span class="preview" style="background:{value}"></span>
      <input class="hex" bind:value={hex} onchange={typed} maxlength="7" spellcheck="false" aria-label="Hex color" />
      <span class="hint">The ring picks the color. In the square, drag up to brighten, down to darken, and left to fade it towards white.</span>
    </div>
  </div>
  <div class="quick">
    {#each QUICK as c}
      <button class="q" class:on={value.toLowerCase() === c} style="background:{c}" aria-label={c} onclick={() => setColor(c)}></button>
    {/each}
  </div>
</div>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px;
    border-radius: var(--radius-sm);
    background: var(--bg-3);
  }
  .top {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
    align-items: center;
  }
  .wheel {
    position: relative;
    flex-shrink: 0;
    touch-action: none;
    cursor: crosshair;
    user-select: none;
  }
  .ring {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: conic-gradient(#f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00);
    -webkit-mask: radial-gradient(closest-side, transparent calc(100% - var(--ring)), #000 calc(100% - var(--ring) + 1px));
    mask: radial-gradient(closest-side, transparent calc(100% - var(--ring)), #000 calc(100% - var(--ring) + 1px));
  }
  .square {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    border-radius: 6px;
  }
  .thumb {
    position: absolute;
    width: 16px;
    height: 16px;
    margin: -8px 0 0 -8px;
    border-radius: 50%;
    border: 2px solid #fff;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.5), 0 1px 4px rgba(0, 0, 0, 0.4);
  }
  .thumb:focus-visible {
    outline: 3px solid var(--text);
    outline-offset: 2px;
  }
  .side {
    flex: 1;
    min-width: 130px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .preview {
    width: 46px;
    height: 46px;
    border-radius: 50%;
    box-shadow: 0 0 0 1px var(--line), inset 0 0 0 3px color-mix(in srgb, #fff 35%, transparent);
  }
  .hint {
    color: var(--muted);
    font-size: 0.82rem;
  }
  .quick {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }
  .q {
    width: 26px;
    height: 26px;
    padding: 0;
    border-radius: 50%;
    border: 2px solid var(--line);
    box-shadow: none;
  }
  .q.on {
    border-color: var(--text);
  }
  .hex {
    width: 92px;
    padding: 5px 10px;
    border-radius: 999px;
    font-family: ui-monospace, monospace;
    font-size: 0.85rem;
    text-transform: lowercase;
  }
</style>
