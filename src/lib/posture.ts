// Notes about how "Resting holds off labor" can tell you're resting (see
// pregsys-core/src/posture.rs and steamvr_posture.rs).

import type { Status } from "./types";

/** A warning about the waist (hip) tracker, or "" when all is well. */
export function hipWarning(st: Status | null | undefined): string {
  if (!st || st.postureSource !== "steamvr" || !st.vrTrackers) return "";
  if (!st.vrWaist) return "Full-body trackers found, but none is set as your waist. Set one in SteamVR > Settings > Controllers > Manage Trackers, or lying down with your playspace floated up may not count as resting.";
  if (!st.vrHipTracked) return "Your waist tracker isn't being tracked right now, so resting is going by your headset height only.";
  return "";
}
