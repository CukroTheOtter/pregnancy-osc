// One API for the UI. Inside the app it calls Rust; in a plain browser
// (npm run dev) it uses the mock so the UI can still be previewed.

import type { AppData, AvatarProfile, JournalEntry, Keepsake, Mapping, MappingInfo, NewPregnancy, Person, Pregnancy, Settings, Status } from "./types";

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  if (inTauri) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  const { mock } = await import("./mock");
  const fn = (mock as unknown as Record<string, (a: Record<string, unknown>) => Promise<T>>)[cmd];
  if (!fn) throw new Error(`mock has no ${cmd}`);
  // Like the real IPC: arguments travel as plain JSON, never live UI state.
  return fn(JSON.parse(JSON.stringify(args)));
}

export async function listen<T>(event: string, cb: (payload: T) => void): Promise<() => void> {
  if (inTauri) {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<T>(event, (e) => cb(e.payload));
  }
  const { mockListen } = await import("./mock");
  return mockListen(event, (p) => cb(p as T));
}

export const api = {
  getData: () => call<AppData>("get_data"),
  getStatus: () => call<Status>("get_status"),
  takeLoadWarning: () => call<string | null>("take_load_warning"),
  saveSettings: (settings: Settings) => call<string | null>("save_settings", { settings }),
  createPregnancy: (opts: NewPregnancy) => call<Pregnancy>("create_pregnancy", { opts }),
  savePregnancy: (pregnancy: Pregnancy) => call<void>("save_pregnancy", { pregnancy }),
  endPregnancy: (id: string) => call<void>("end_pregnancy", { id }),
  deletePregnancy: (id: string) => call<void>("delete_pregnancy", { id }),
  setCurrentPregnancy: (avatarId: string | null, id: string | null) => call<void>("set_current_pregnancy", { avatarId, id }),
  setPartner: (pregnancyId: string, person: Person | null, name: string | null) => call<void>("set_partner", { pregnancyId, person, name }),
  saveAvatar: (profile: AvatarProfile) => call<void>("save_avatar", { profile }),
  deleteAvatar: (id: string) => call<void>("delete_avatar", { id }),
  createGroup: (name: string, avatarIds: string[]) => call<string>("create_group", { name, avatarIds }),
  setAvatarGroup: (avatarId: string, groupId: string | null) => call<void>("set_avatar_group", { avatarId, groupId }),
  renameGroup: (id: string, name: string) => call<void>("rename_group", { id, name }),
  deleteGroup: (id: string) => call<void>("delete_group", { id }),
  setSpeciesAnswer: (key: string, species: string) => call<void>("set_species_answer", { key, species }),
  forgetPerson: (name: string, userId: string | null) => call<void>("forget_person", { name, userId }),
  importCommunityTable: (json: string) => call<number>("import_community_table", { json }),
  saveJournalEntry: (entry: JournalEntry) => call<JournalEntry>("save_journal_entry", { entry }),
  deleteJournalEntry: (id: string) => call<void>("delete_journal_entry", { id }),
  addKeepsake: (keepsake: Keepsake) => call<Keepsake>("add_keepsake", { keepsake }),
  pulse: (key: string, ms?: number) => call<void>("pulse", { key, ms: ms ?? null }),
  exportBackup: () => call<string>("export_backup"),
  importBackup: (json: string) => call<void>("import_backup", { json }),
  resetData: (kind: import("./types").ResetKind) => call<void>("reset_data", { kind }),
  getMapping: () => call<MappingInfo>("get_mapping"),
  saveMapping: (mapping: Mapping) => call<void>("save_mapping", { mapping }),
  resetMapping: () => call<void>("reset_mapping"),
  reloadMapping: () => call<void>("reload_mapping"),
  openPath: (path: string) => call<void>("open_path", { path }),
  defaultLogDirs: () => call<string[]>("default_log_dirs"),
  quitApp: () => call<void>("quit_app"),
  appVersion: () => call<string>("app_version"),
  checkUpdate: () => call<import("./types").UpdateInfo | null>("check_update"),
  /** Downloads and installs; the app closes and opens again. */
  installUpdate: () => call<void>("install_update"),
  simulateConception: () => call<void>("simulate_conception"),
  partnerPicture: (userId: string) => call<string | null>("partner_picture", { userId }),
  giveBirth: (id: string) => call<void>("give_birth", { id }),
  debugWoohoo: (outcome: boolean | null) => call<void>("debug_woohoo", { outcome }),
  debugRevealNow: () => call<void>("debug_reveal_now"),
  debugKick: (strength: import("./types").KickStrength) => call<void>("debug_kick", { strength }),
  debugAddPerson: (name: string, userId: string | null) => call<void>("debug_add_person", { name, userId }),
  debugRemovePerson: (name: string, userId: string | null) => call<void>("debug_remove_person", { name, userId }),
  debugPicture: (userId: string) => call<string | null>("debug_picture", { userId }),
  debugSetCycleDay: (day: number) => call<void>("debug_set_cycle_day", { day }),
  avatarPreview: (avatarId: string) => call<import("./types").AvatarPreview | null>("avatar_preview", { avatarId }),
  addAvatar: (avatarId: string, name: string | null) => call<import("./types").AvatarProfile>("add_avatar", { avatarId, name }),
  dismissReveal: () => call<void>("dismiss_reveal"),
  restartCycle: () => call<void>("restart_cycle"),
  /** Browser preview only: end the grace period now. */
  skipWaiting: () => call<void>("skip_waiting"),
  /** Browser preview only: pretend you switched avatar. */
  simulateAvatar: (id: string) => call<void>("simulate_avatar", { id }),
};

/** Saves text to a file the user picks (app) or downloads it (browser). */
export async function saveTextFile(defaultName: string, text: string, ext = "json"): Promise<boolean> {
  return saveBlob(defaultName, new Blob([text], { type: ext === "json" ? "application/json" : "text/plain" }), ext);
}

export async function saveBlob(defaultName: string, blob: Blob, ext: string): Promise<boolean> {
  const fileName = defaultName.endsWith(`.${ext}`) ? defaultName : `${defaultName}.${ext}`;
  if (inTauri) {
    // Webview downloads aren't reliable on every platform: use a save dialog.
    const { save } = await import("@tauri-apps/plugin-dialog");
    const path = await save({ defaultPath: fileName, filters: [{ name: ext.toUpperCase(), extensions: [ext] }] });
    if (!path) return false;
    const bytes = Array.from(new Uint8Array(await blob.arrayBuffer()));
    await call<void>("write_file", { path, bytes });
    return true;
  }
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = fileName;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 5000);
  return true;
}

/** Asks the user for a text file and returns its contents. */
export function pickTextFile(accept = ".json,application/json"): Promise<string | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = accept;
    input.onchange = async () => {
      const f = input.files?.[0];
      resolve(f ? await f.text() : null);
    };
    input.oncancel = () => resolve(null);
    input.click();
  });
}

/** Folder picker (app only). */
export async function pickFolder(): Promise<string | null> {
  if (!inTauri) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const r = await open({ directory: true, multiple: false, title: "Choose the VRChat log folder" });
  return typeof r === "string" ? r : null;
}
