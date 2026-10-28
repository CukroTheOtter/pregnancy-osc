<script lang="ts">
  import { untrack } from "svelte";
  // Picks your own accent color, in the app's style: a hue slider and a
  // soft-to-bold slider, plus a few quick picks. Colors stay bright enough
  // for white text on buttons (no white or near-black accents).

  let { value, onchange }: { value: string; onchange: (hex: string) => void } = $props();

  // Soft = pastel, bold = deeper. Lightness and saturation follow one slider.
  const tone = (t: number) => ({ s: 62 + t * 30, l: 76 - t * 22 });

  function hslToHex(h: number, s: number, l: number): string {
    s /= 100;
    l /= 100;
    const k = (n: number) => (n + h / 30) % 12;
    const a = s * Math.min(l, 1 - l);
    const f = (n: number) => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
    return "#" + [f(0), f(8), f(4)].map((x) => Math.round(x * 255).toString(16).padStart(2, "0")).join("");
  }

  /** Hue (0-360) and tone (0-1) closest to a hex color. */
  function fromHex(hex: string): { hue: number; t: number } {
    const n = parseInt(hex.slice(1), 16);
    const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((x) => x / 255);
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    const d = max - min;
    let hue = 0;
    if (d) hue = max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
    const l = ((max + min) / 2) * 100;
    return { hue: Math.round((hue * 60 + 360) % 360), t: Math.min(1, Math.max(0, (76 - l) / 22)) };
  }

  // The picker keeps its own sliders while open; it starts from the current color.
  const first = untrack(() => value);
  const start = fromHex(/^#[0-9a-f]{6}$/i.test(first) ? first : "#f59ac0");
  let hue = $state(start.hue);
  let t = $state(start.t);
  let hex = $state(first);

  const QUICK = ["#f59ac0", "#ff6f91", "#e2557a", "#ffad7a", "#f7c95c", "#9ccf6f", "#6fcfac", "#5fc4d8", "#7fb8f5", "#8f9cff", "#b39bff", "#d58cf0"];

  function emit() {
    const { s, l } = tone(t);
    hex = hslToHex(hue, s, l);
    onchange(hex);
  }

  function pick(c: string) {
    ({ hue, t } = fromHex(c));
    hex = c;
    onchange(c);
  }

  function typed() {
    const v = hex.trim().startsWith("#") ? hex.trim() : "#" + hex.trim();
    // Brought into range like the sliders, so it stays readable.
    if (/^#[0-9a-f]{6}$/i.test(v)) {
      ({ hue, t } = fromHex(v));
      emit();
    }
  }

  const toneTrack = $derived(`linear-gradient(90deg, ${hslToHex(hue, tone(0).s, tone(0).l)}, ${hslToHex(hue, tone(1).s, tone(1).l)})`);
</script>

<div class="picker">
  <div class="top">
    <span class="preview" style="background:{value}"></span>
    <div class="sliders">
      <label class="slider">
        <span class="small muted">Color</span>
        <input type="range" class="hue" min="0" max="359" bind:value={hue} oninput={emit} aria-label="Color" />
      </label>
      <label class="slider">
        <span class="small muted">Soft to bold</span>
        <input type="range" class="tone" min="0" max="1" step="0.01" bind:value={t} oninput={emit} style="--track:{toneTrack}" aria-label="Soft to bold" />
      </label>
    </div>
  </div>
  <div class="quick">
    {#each QUICK as c}
      <button class="q" class:on={value.toLowerCase() === c} style="background:{c}" aria-label={c} onclick={() => pick(c)}></button>
    {/each}
    <input class="hex" bind:value={hex} onchange={typed} maxlength="7" spellcheck="false" aria-label="Hex color" />
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
    gap: 12px;
    align-items: center;
  }
  .preview {
    width: 46px;
    height: 46px;
    border-radius: 50%;
    flex-shrink: 0;
    box-shadow: inset 0 0 0 3px color-mix(in srgb, #fff 35%, transparent);
  }
  .sliders {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .slider {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  input[type="range"] {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 14px;
    border-radius: 999px;
    border: none;
    padding: 0;
    cursor: pointer;
  }
  .hue {
    background: linear-gradient(90deg, #f99, #fc9, #ff9, #9f9, #9ff, #99f, #f9f, #f99);
  }
  .tone {
    background: var(--track);
  }
  input[type="range"]::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #fff;
    border: 3px solid var(--accent);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.3);
  }
  input[type="range"]::-moz-range-thumb {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #fff;
    border: 3px solid var(--accent);
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
    border: 2px solid transparent;
    box-shadow: none;
  }
  .q.on {
    border-color: var(--text);
  }
  .hex {
    width: 84px;
    margin-left: auto;
    padding: 5px 10px;
    border-radius: 999px;
    font-family: ui-monospace, monospace;
    font-size: 0.85rem;
    text-transform: lowercase;
  }
</style>
