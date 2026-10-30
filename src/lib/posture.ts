// Notes about how "Resting holds off labor" can tell you're resting (see
// pregsys-core/src/posture.rs and steamvr.rs).

import type { Status } from "./types";
import { app, errorToast, toast } from "./app.svelte";
import { api } from "./api";

/** "Set height now": SteamVR measures your headset. Shows it straight away. */
export async function setHeightNow(): Promise<void> {
  try {
    const m = await api.setStandingHeightNow();
    if (app.status) app.status.vrStandingM = m;
    toast("Height set", `${m.toFixed(2)} m. Sitting or lying well below it counts as resting.`);
  } catch (e) {
    errorToast(e);
  }
}

/** Forgets your height (learned again as you play, if that's on). */
export async function forgetHeight(): Promise<void> {
  try {
    await api.forgetStandingHeight();
    if (app.status) app.status.vrStandingM = null;
    toast("Height forgotten", app.data?.settings.vrHeightAuto !== false ? "It's learned again as you play." : "Set it again when you're ready.");
  } catch (e) {
    errorToast(e);
  }
}

/** A warning about the waist (hip) tracker, or "" when all is well. */
export function hipWarning(st: Status | null | undefined): string {
  if (!st || st.postureSource !== "steamvr" || !st.vrTrackers) return "";
  if (!st.vrWaist) return "Full-body trackers found, but none is set as your waist. Set one in SteamVR > Settings > Controllers > Manage Trackers, or lying down with your playspace floated up may not count as resting.";
  if (!st.vrHipTracked) return "Your waist tracker isn't being tracked right now, so resting is going by your headset height only.";
  return "";
}
