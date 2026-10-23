<script lang="ts">
  import Baby from "./Baby.svelte";
  import { species } from "../lib/species";

  let { id, size = 40 }: { id: string; size?: number } = $props();
  const sp = $derived(species(id));
  const look = $derived({
    ears: sp.ears,
    muzzle: sp.muzzle,
    covering: sp.covering,
    pattern: sp.pattern === "mask" ? "mask" : undefined,
    furColor: sp.furColors[0],
    eyeColor: sp.eyeColors[0],
  });
</script>

<svg viewBox="-48 -62 96 92" width={size} height={size} aria-label={sp.name} role="img">
  <Baby {look} awake />
</svg>
