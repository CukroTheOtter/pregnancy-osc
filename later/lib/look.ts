// Turns genetics results into what the baby sprite draws, hiding traits
// that haven't been revealed yet.

import type { AppData, Pregnancy } from "./types";
import { computeBaby, contextFrom, type BabyResult } from "./genetics";
import { isTraitVisible } from "./reveals";
import type { TraitKey } from "./species";

export interface BabyLook {
  ears?: string;
  tail?: string;
  wings?: string;
  muzzle?: string;
  covering?: string;
  pattern?: string;
  furColor?: string;
  eyeColor?: string;
  sex?: string;
}

export function lookFrom(r: BabyResult, visible: (k: TraitKey) => boolean = () => true): BabyLook {
  const v = (k: TraitKey) => (visible(k) ? r.traits[k].value : undefined);
  return {
    ears: v("ears"),
    tail: v("tail"),
    wings: v("wings"),
    muzzle: v("muzzle"),
    covering: v("covering"),
    pattern: v("pattern"),
    furColor: v("furColor"),
    eyeColor: v("eyeColor"),
    sex: v("sex"),
  };
}

export interface BabyView {
  result: BabyResult;
  look: BabyLook;
  name: string;
}

/** Every baby of a pregnancy with its look, respecting the spoiler mode. */
export function babyViews(p: Pregnancy, data: AppData, peek = false): BabyView[] {
  const ctx = contextFrom(data);
  const spoiler = peek ? "all" : data.settings.spoilerMode;
  return p.babies.map((b) => {
    const result = computeBaby(p, b, ctx);
    return {
      result,
      look: lookFrom(result, (k) => isTraitVisible(p, k, spoiler, b.locks[k] != null)),
      name: b.name ?? `Baby ${b.index + 1}`,
    };
  });
}
