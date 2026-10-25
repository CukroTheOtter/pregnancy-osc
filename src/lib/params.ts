// Friendly names and explanations for the app's own avatar parameters.

import type { AvatarProfile, Mapping, SyncRole } from "./types";

export const PARAM_INFO: Record<string, { label: string; text: string }> = {
  stage: { label: "Belly size", text: "0 to 1. The app writes this." },
  bellyExtra: { label: "Extra size", text: "0 to 1. Bigger for twins and more, and when overdue. For the SDK's Bigger Belly." },
  appActive: { label: "App is running", text: "True while the app runs, so the avatar can pause its own counter." },
  appHeartbeat: { label: "Heartbeat", text: "Flips every second. If it stops, the app has closed." },
  fertileLevel: { label: "Fertile level", text: "0 low, 1 medium, 2 high, 3 peak." },
  waiting: { label: "Waiting", text: "True during the grace period after a WooHoo." },
  babyCount: { label: "Baby count", text: "0 until it can be seen (week 12), then 1 to 4." },
  fetalMovement: { label: "Fetal movement", text: "Kicks: the values sent are set under Kick values below." },
  controlStart: { label: "Menu: Start", text: "Button in the avatar's App Controls menu. Starts a pregnancy." },
  controlPause: { label: "Menu: Pause", text: "Button in the avatar's App Controls menu. Pauses or resumes." },
  controlStop: { label: "Menu: Stop", text: "Button in the avatar's App Controls menu. Hold 2 seconds to end the pregnancy." },
};

/** The app can't work on an avatar without these. */
export const REQUIRED = ["stage", "appActive", "appHeartbeat"];

/** Optional parameters, by the feature each one turns on. */
export const FEATURES: { label: string; keys: string[] }[] = [
  { label: "Bigger belly", keys: ["bellyExtra"] },
  { label: "Kicks", keys: ["fetalMovement"] },
  { label: "Menu buttons", keys: ["controlStart", "controlPause", "controlStop"] },
  { label: "Fertile level", keys: ["fertileLevel"] },
  { label: "Waiting", keys: ["waiting"] },
  { label: "Baby count", keys: ["babyCount"] },
];

/** Every app parameter: required first, then features. */
export const PARAM_ORDER = [...REQUIRED, ...FEATURES.flatMap((f) => f.keys)];

/** The app's own menu buttons: names fixed, made for this app by the SDK. */
export const FIXED = ["controlStart", "controlPause", "controlStop"];

/** Built-in Sync entries, by role. */
export const SYNC_ROLES: Record<SyncRole, string> = { hideBelly: "🙈 Hide belly", fetalSounds: "🔊 Fetal sounds" };

/** Friendly name for a Sync entry (your own ones show their parameter name). */
export function syncLabel(row: { name: string; role?: SyncRole }): string {
  return row.role ? SYNC_ROLES[row.role] : row.name;
}

/** Whether an avatar has a parameter (null: not checked yet, or the parameter is off). */
export function hasParam(mapping: Mapping | null | undefined, p: AvatarProfile | null | undefined, key: string): boolean | null {
  if (!mapping || !p || p.params.length === 0) return null;
  const custom = FIXED.includes(key) ? undefined : p.paramNames?.[key];
  const def = mapping.parameters[key];
  const names = custom ? [custom] : def?.name ? [def.name, ...(def.aliases ?? [])] : [];
  if (!names.length) return null;
  return names.some((n) => p.params.includes(n));
}
