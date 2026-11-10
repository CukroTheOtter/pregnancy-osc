// Global UI state (Svelte 5 runes).

import { api, listen } from "./api";
import type { AppData, Pregnancy, PendingConception, Problem, Status, UpdateInfo } from "./types";
import { ownerAvatar, pendingFor, pregnancyAvatar, pregnancyFor } from "./scope";
import { ACHIEVEMENTS } from "./achievements";

export type View = "home" | "me" | "partners" | "avatars" | "settings";

export interface Toast {
  id: number;
  title: string;
  text?: string;
  kind: "info" | "error" | "success";
}

export interface Ask {
  title: string;
  text?: string;
  ok: string;
  /** null = no Cancel button (just letting you know). */
  cancel?: string | null;
  danger?: boolean;
  resolve: (yes: boolean) => void;
}

export const app = $state({
  data: null as AppData | null,
  status: null as Status | null,
  now: Date.now(),
  view: "home" as View,
  /** Avatar shown on Home (null = the one you're wearing). */
  viewAvatarId: null as string | null,
  toasts: [] as Toast[],
  ask: null as Ask | null,
  ready: false,
  version: "",
  /** A newer version on GitHub, if the last check found one. */
  update: null as UpdateInfo | null,
  /** Problems in the save or settings (Settings > Troubleshooting; the Settings tab is outlined while there are any). */
  problems: [] as Problem[],
  /** Asks a Settings section to open and scroll into view (n changes each time). */
  openSection: null as { id: string; n: number } | null,
  /** Something new on Home (a WooHoo, a result) while you were on another tab: its tab is outlined until you look. */
  homeNews: false,
});

/** Checks the save and settings again (pregsys-core/src/check.rs). */
export async function checkProblems() {
  try {
    app.problems = await api.checkProblems();
  } catch {
    // Keeps the last list; the check runs again on the next change.
  }
}

let toastId = 0;
export function toast(title: string, text?: string, kind: Toast["kind"] = "info", ms = 5000) {
  const id = ++toastId;
  app.toasts.push({ id, title, text, kind });
  setTimeout(() => (app.toasts = app.toasts.filter((t) => t.id !== id)), ms);
}

export function errorToast(e: unknown) {
  toast("Something went wrong", e instanceof Error ? e.message : String(e), "error", 8000);
}

/** In-app confirm dialog (instead of the system popup). */
export function confirmAsk(opts: Omit<Ask, "resolve">): Promise<boolean> {
  return new Promise((resolve) => {
    app.ask = {
      ...opts,
      resolve: (yes) => {
        app.ask = null;
        resolve(yes);
      },
    };
  });
}

/** The avatar Home is about. */
export function homeAvatarId(): string | null {
  return app.viewAvatarId ?? app.data?.currentAvatarId ?? null;
}

/** Shows an avatar on Home (null or the worn one: back to the worn one) and scrolls up. */
export function viewOnHome(avatarId: string | null) {
  app.viewAvatarId = avatarId && avatarId !== app.data?.currentAvatarId ? avatarId : null;
  app.view = "home";
  document.querySelector("main")?.scrollTo({ top: 0, behavior: "smooth" });
}

/** Shows a pregnancy on Home: an avatar it's on (the worn one in global mode). */
export function viewPregnancy(p: Pregnancy) {
  if (app.data) viewOnHome(pregnancyAvatar(app.data, p));
}

/** Shows a WooHoo waiting for its result on Home. */
export function viewPending(w: PendingConception) {
  if (app.data) viewOnHome(ownerAvatar(app.data, w.avatarId));
}

export function currentPregnancy(): Pregnancy | undefined {
  return app.data ? pregnancyFor(app.data, homeAvatarId()) : undefined;
}

export function currentPending(): PendingConception | undefined {
  return app.data ? pendingFor(app.data, homeAvatarId()) : undefined;
}

// Settings changed here and not saved yet: a reload (they happen all the time,
// like on every new VRChat log line) mustn't bring the old ones back.
let unsaved = 0;
let settingsEpoch = 0;
export function settingsChanged() {
  unsaved++;
  settingsEpoch++;
}
export function settingsSaved() {
  unsaved = Math.max(0, unsaved - 1);
  settingsEpoch++;
}

export async function refresh() {
  try {
    const epoch = settingsEpoch;
    const data = await api.getData();
    // Also keeps ours if the settings changed while this reload was on its way.
    if (app.data && (unsaved > 0 || epoch !== settingsEpoch)) data.settings = app.data.settings;
    app.data = data;
  } catch (e) {
    errorToast(e);
  }
  checkProblems();
}

/** Edits a pregnancy locally (instant UI) and saves it. */
export async function updatePregnancy(id: string, change: (p: Pregnancy) => void) {
  const p = app.data?.pregnancies.find((x) => x.id === id);
  if (!p) return;
  change(p);
  try {
    await api.savePregnancy($state.snapshot(p) as Pregnancy);
  } catch (e) {
    errorToast(e);
    await refresh();
  }
}

/** Shows Home after something happened in-game, unless you're testing in the
 * Debug panel (Settings with debug mode on): then you stay where you are. */
function showHome() {
  if (app.view === "home") return;
  // Busy on another tab (the window has focus), or testing in Debug: outline Home instead of pulling you away.
  if ((app.view === "settings" && app.data?.settings.debug) || (typeof document !== "undefined" && document.hasFocus())) {
    app.homeNews = true;
    return;
  }
  app.view = "home";
  app.viewAvatarId = null;
}

export async function init() {
  await refresh();
  try {
    app.status = await api.getStatus();
    const warning = await api.takeLoadWarning();
    // A popup that stays until you've seen it (settings reset, a save that couldn't be read).
    if (warning) void confirmAsk({ title: "Heads up", text: warning, ok: "OK", cancel: null });
  } catch (e) {
    errorToast(e);
  }
  await listen<Status>("status", (s) => (app.status = s));
  await listen("data-changed", () => refresh());
  await listen("instance-changed", () => refresh());
  await listen("conception", () => {
    showHome();
    toast("A new little bean! 💕", "Conception was detected in-game, so the pregnancy has started.", "success", 10000);
    refresh();
  });
  await listen("fun", () => {
    showHome();
    refresh();
  });
  await listen<string[]>("achievements", (ids) => {
    const one = ids.length === 1 ? ACHIEVEMENTS.find((a) => a.id === ids[0]) : null;
    toast(app.data?.settings.debug ? "Test achievement 🛠" : "Achievement unlocked! 🏆", one ? `${one.emoji} ${one.title}` : `You unlocked ${ids.length} achievements. See them on the Me tab.`, "success", 8000);
    refresh();
  });
  await listen("reveal", () => {
    showHome();
    refresh();
  });
  setInterval(() => (app.now = Date.now()), 1000);
  app.ready = true;
  app.version = await api.appVersion().catch(() => "");
  // Quietly: being offline at start-up isn't worth a toast.
  checkForUpdate().then((u) => u && toast("An update is ready ✨", `Version ${u.version} is out. Get it in Settings → Updates.`, "info", 10000)).catch(() => {});
}

/** Asks GitHub for a newer version. Throws if it can't check. */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  app.update = await api.checkUpdate();
  return app.update;
}
