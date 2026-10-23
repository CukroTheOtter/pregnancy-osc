// Global UI state (Svelte 5 runes).

import { api, listen } from "./api";
import type { AppData, Pregnancy, PendingConception, Status, UpdateInfo } from "./types";
import { pendingFor, pregnancyFor } from "./scope";
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
  cancel?: string;
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
});

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

export function currentPregnancy(): Pregnancy | undefined {
  return app.data ? pregnancyFor(app.data, homeAvatarId()) : undefined;
}

export function currentPending(): PendingConception | undefined {
  return app.data ? pendingFor(app.data, homeAvatarId()) : undefined;
}

export async function refresh() {
  try {
    app.data = await api.getData();
  } catch (e) {
    errorToast(e);
  }
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

export async function init() {
  await refresh();
  try {
    app.status = await api.getStatus();
    const warning = await api.takeLoadWarning();
    if (warning) toast("Your saved data had a problem", warning, "error", 15000);
  } catch (e) {
    errorToast(e);
  }
  await listen<Status>("status", (s) => (app.status = s));
  await listen("data-changed", () => refresh());
  await listen("instance-changed", () => refresh());
  await listen("conception", () => {
    app.view = "home";
    app.viewAvatarId = null;
    toast("A new little bean! 💕", "Conception was detected in-game, so the pregnancy has started.", "success", 10000);
    refresh();
  });
  await listen("fun", () => {
    app.view = "home";
    app.viewAvatarId = null;
    refresh();
  });
  await listen<string[]>("achievements", (ids) => {
    const one = ids.length === 1 ? ACHIEVEMENTS.find((a) => a.id === ids[0]) : null;
    toast("Achievement unlocked! 🏆", one ? `${one.emoji} ${one.title}` : `You unlocked ${ids.length} achievements. See them on the Me tab.`, "success", 8000);
    refresh();
  });
  await listen("reveal", () => {
    app.view = "home";
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
