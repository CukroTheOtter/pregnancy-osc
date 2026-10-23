// Browser-only stand-in for the Rust backend, so the UI can be previewed
// with `npm run dev` in any browser. Data lives in localStorage.

import type { AppData, AvatarProfile, JournalEntry, Keepsake, Mapping, MappingInfo, NewPregnancy, Partner, Person, Pregnancy, ResetKind, Settings, Status } from "./types";
import { emptyData, baby, defaultSettings } from "./testdata";
import { newTimeline, progress } from "./timeline";
import { chance, cycleDay, cycleMs, graceMs } from "./fertility";
import { groupMembers, owner, pendingFor, pregnancyFor, pregnancyIdOf } from "./scope";
import { socketCounts, typeOf } from "./sps";

const KEY = "pregsys-mock-data";
type Listener = (payload: unknown) => void;
const listeners = new Map<string, Set<Listener>>();

function emit(event: string, payload: unknown = null) {
  listeners.get(event)?.forEach((l) => l(payload));
}

export function mockListen(event: string, cb: Listener): () => void {
  if (!listeners.has(event)) listeners.set(event, new Set());
  listeners.get(event)!.add(cb);
  return () => listeners.get(event)?.delete(cb);
}

let idn = 0;
const newId = (p: string) => `${p}_${Date.now().toString(16)}${(idn++).toString(16)}`;

const DEMO_PEOPLE: Person[] = [
  { name: "Ember Fox", userId: "usr_00000000-0000-0000-0000-000000000001", avatarName: "Kitsune Deluxe", lastSeenMs: Date.now() - 60_000, worldName: "Cozy Cabin" },
  { name: "Rex", userId: "usr_00000000-0000-0000-0000-000000000002", avatarName: "Wolf Pup v2", lastSeenMs: Date.now() - 3_600_000, worldName: "Cozy Cabin" },
  { name: "Scales", userId: "usr_00000000-0000-0000-0000-000000000003", avatarName: "Dragon Kin", lastSeenMs: Date.now() - 7_200_000, worldName: "Cozy Cabin" },
];

const prof = (id: string, name: string): AvatarProfile => ({
  id, name, species: null, kind: null, hideBelly: null, bellyScale: null, fetalSounds: null, pregnancyId: null,
  followGlobal: false, lastSeenMs: Date.now(), params: ["Pregnancy/Stage", "Pregnancy/HideBelly", "Pregnancy/AppActive", "OGB/Orf/Pussy/PenOthersNewRoot", "OGB/Orf/Mouth/PenOthersNewRoot", "OGB/Orf/Hole1/PenOthersNewRoot"], groupId: null, paramNames: {},
  sps: { custom: false, sockets: [], types: {} },
});

function load(): AppData {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) {
      // Like the app: settings saved by an older version get the new ones filled in.
      const saved = JSON.parse(raw);
      const def = defaultSettings();
      return { ...emptyData(), ...saved, groups: saved.groups ?? {}, settings: { ...def, ...saved.settings, sps: { ...def.sps, ...saved.settings?.sps } } };
    }
  } catch {
    /* fall through to fresh data */
  }
  const d = emptyData();
  d.instance = {
    local: { name: "You", userId: "usr_demo_me", avatarName: "Bunny Base", lastSeenMs: Date.now() },
    worldId: "wrld_demo",
    worldName: "Cozy Cabin",
    instance: "1~private",
    joinedMs: Date.now() - 3_600_000,
    present: [{ name: "You", userId: "usr_demo_me", avatarName: "Bunny Base", lastSeenMs: Date.now() }, ...DEMO_PEOPLE],
    history: DEMO_PEOPLE,
  };
  const old = (id: string, name: string, days: number, params?: string[]) => ({ ...prof(id, name), lastSeenMs: Date.now() - days * 86_400_000, ...(params ? { params } : {}) });
  d.avatars = {
    avtr_bunny: prof("avtr_bunny", "Bunny Base"),
    avtr_fox: prof("avtr_fox", "Kitsune Girl"),
    avtr_human: prof("avtr_human", "Manuka"),
    avtr_old1: old("avtr_old1", "Summer Bunny", 40),
    avtr_old2: old("avtr_old2", "Witchy Rusk", 75),
    avtr_plain1: old("avtr_plain1", "Avali Default", 1, ["VelocityX", "Grounded"]),
    avtr_plain2: old("avtr_plain2", "Robot Test", 3, ["VelocityX"]),
  };
  d.currentAvatarId = "avtr_bunny";
  d.settings.cycleAnchorMs = Date.now() - (cycleMs(d.settings) * 8) / 28;
  return d;
}

let data: AppData = load();
const save = () => {
  try {
    localStorage.setItem(KEY, JSON.stringify(data));
  } catch {
    /* storage may be blocked; keep in memory */
  }
};
const changed = () => {
  save();
  emit("data-changed");
};

function status(): Status {
  const p = pregnancyFor(data, data.currentAvatarId);
  return {
    vrchatRunning: true,
    steamvrRunning: false,
    oscActive: true,
    oscquery: true,
    vrchatFound: true,
    sendTarget: "127.0.0.1:9000 (preview)",
    recvPort: 52011,
    backupPort: 9001,
    queryPort: 52012,
    vrchatQuery: "127.0.0.1:52100 (preview)",
    lastHeardMs: Date.now() - 3000,
    avatarId: data.currentAvatarId,
    avatarParams: data.currentAvatarId ? data.avatars[data.currentAvatarId]?.params ?? null : null,
    lastStage: p ? Math.min(1, progress(p.timeline, Date.now())) : pendingFor(data, data.currentAvatarId) ? 0 : null,
    lastExtra: p ? (data.settings.biggerForMultiples ? ([0, 0, 0.35, 0.6][p.babyCount] ?? 0.8) : 0) : null,
    logDir: "(browser preview)",
    logFile: null,
    worldName: data.instance.worldName,
    others: data.instance.present.filter((x) => x.userId !== data.instance.local?.userId),
    note: "Browser preview: no real OSC or log reading.",
    trayAvailable: false,
    spsFound: ["Hole1", "Mouth", "Pussy"],
    spsLive: ["Hole1", "Mouth", "Pussy"].map((name) => {
      const a = data.currentAvatarId ? data.avatars[data.currentAvatarId] : undefined;
      return { name, channel: "PenOthersNewRoot", value: 0, holeType: typeOf(a, name), counts: socketCounts(data.settings, a, name) };
    }),
    spsProgress: 0,
    mappingPath: "config/osc-parameters.json",
    dataDir: "(localStorage)",
  };
}

const unknownPartner = (candidates: Person[]): Partner => ({ status: "unknown", name: null, userId: null, avatarName: null, species: null, speciesSource: "fallback", candidates });

function guessPartner(): Partner {
  const others = data.instance.present.filter((x) => x.userId !== data.instance.local?.userId);
  const p = others.length === 1 ? others[0] : null;
  return p
    ? { status: "guessed", name: p.name, userId: p.userId, avatarName: p.avatarName, species: null, speciesSource: "guess", candidates: [...others] }
    : unknownPartner([...others]);
}

function rollBabies(): number {
  const r = Math.random();
  const [q, t, w] = data.settings.multiples === "realistic" ? [0, 0.001, 0.031] : data.settings.multiples === "lots" ? [0.02, 0.1, 0.35] : [0.002, 0.022, 0.122];
  return r < q ? 4 : r < t ? 3 : r < w ? 2 : 1;
}

function setCurrent(avatarId: string | null, id: string | null) {
  if (data.settings.syncScope === "perAvatar" && avatarId) {
    const o = owner(data, avatarId) ?? avatarId;
    if (data.groups[o]) {
      data.groups[o].pregnancyId = id;
      return;
    }
    const p = (data.avatars[o] ??= prof(o, "Unnamed avatar"));
    if (p.followGlobal) data.activePregnancyId = id;
    else p.pregnancyId = id;
  } else data.activePregnancyId = id;
}

function createPregnancy(opts: NewPregnancy & { partner?: Partner; babyCount?: number | null }): Pregnancy {
  const now = Date.now();
  const s = data.settings;
  const aid = opts.avatarId ?? data.currentAvatarId;
  const p: Pregnancy = {
    id: newId("preg"),
    name: opts.name?.trim() || `Little Bean #${data.pregnancies.length + 1}`,
    createdMs: now,
    timeline: newTimeline(opts.startMs ?? now, opts.mode ?? s.defaultMode, opts.speed ?? s.defaultSpeed, opts.customMs ?? s.defaultCustomMs),
    babyCount: opts.babyCount ?? rollBabies(),
    kind: "human",
    motherSpecies: s.userSpecies,
    partner: opts.partner ?? guessPartner(),
    babies: [],
    reveals: {},
    source: opts.source ?? "manual",
    avatarId: aid,
    worldName: data.instance.worldName,
    endedMs: null,
    born: false,
    conceivedMs: opts.source === "conception" ? (opts.startMs ?? now) : null,
    genders: [],
    kicks: [],
    milestones: [],
    oscHints: { speciesIndex: null },
  };
  p.babies = Array.from({ length: p.babyCount }, (_, i) => baby(i));
  p.genders = Array.from({ length: p.babyCount }, () => (Math.random() < 0.5 ? "girl" : "boy"));
  data.pregnancies.push(p);
  setCurrent(aid, p.id);
  return p;
}

function resolveDue(force = false) {
  const now = Date.now();
  const i = data.waiting.findIndex((w) => force || w.revealMs <= now);
  if (i < 0) return;
  const w = data.waiting.splice(i, 1)[0];
  if (w.positive) createPregnancy({ startMs: w.timeMs, source: "conception", avatarId: w.avatarId, partner: w.partner, babyCount: w.babyCount });
  data.lastReveal = { timeMs: now, eventMs: w.timeMs, positive: w.positive, seen: false };
  changed();
  emit("reveal", w.positive);
}

setInterval(() => {
  resolveDue();
  emit("status", status());
}, 1000);

function endPregnancy(id: string) {
  const p = data.pregnancies.find((x) => x.id === id);
  if (p) p.endedMs = Date.now();
  if (data.activePregnancyId === id) data.activePregnancyId = null;
  Object.values(data.avatars).forEach((a) => a.pregnancyId === id && (a.pregnancyId = null));
  Object.values(data.groups).forEach((g) => g.pregnancyId === id && (g.pregnancyId = null));
}

// Groups: same rules as pregsys-core/src/actions.rs.
function hasPregnancy(ownerId: string): boolean {
  const id = pregnancyIdOf(data, ownerId);
  return data.pregnancies.some((p) => p.id === id && !p.endedMs) || data.waiting.some((w) => w.avatarId === ownerId);
}

function movePregnancy(from: string, to: string) {
  const src = data.groups[from] ?? data.avatars[from];
  const preg = src?.pregnancyId ?? null;
  if (src) src.pregnancyId = null;
  const dst = data.groups[to] ?? data.avatars[to];
  if (dst) dst.pregnancyId = preg;
  data.waiting.forEach((w) => w.avatarId === from && (w.avatarId = to));
}

function pruneGroups() {
  const used = new Set(Object.values(data.avatars).map((a) => a.groupId));
  for (const g of Object.keys(data.groups)) {
    if (used.has(g)) continue;
    delete data.groups[g];
    data.waiting = data.waiting.filter((w) => w.avatarId !== g);
  }
}

function setAvatarGroup(avatarId: string, groupId: string | null) {
  if (groupId && !data.groups[groupId]) throw new Error("That group doesn't exist anymore.");
  const p = data.avatars[avatarId];
  if (!p) throw new Error("That avatar isn't in your list.");
  const old = p.groupId && data.groups[p.groupId] ? p.groupId : null;
  if (old === groupId) return;
  const carried = old ? (groupMembers(data, old).length === 1 ? old : "") : avatarId;
  const carries = !!carried && hasPregnancy(carried);
  if (groupId && carries && hasPregnancy(groupId)) throw new Error(`${p.name ?? "This avatar"} and ${data.groups[groupId].name} both have a pregnancy. End or finish one first.`);
  p.groupId = groupId;
  if (carries) movePregnancy(carried, groupId ?? avatarId);
  pruneGroups();
}

let mapping: Mapping | null = null;

export const mock = {
  async get_data() {
    return structuredClone(data);
  },
  async get_status() {
    return status();
  },
  async take_load_warning() {
    return null;
  },
  async save_settings({ settings }: { settings: Settings }) {
    data.settings = settings;
    changed();
    return null;
  },
  async create_pregnancy({ opts }: { opts: NewPregnancy }) {
    const p = createPregnancy(opts);
    changed();
    return structuredClone(p);
  },
  async save_pregnancy({ pregnancy }: { pregnancy: Pregnancy }) {
    const i = data.pregnancies.findIndex((p) => p.id === pregnancy.id);
    if (i < 0) throw new Error("pregnancy not found");
    data.pregnancies[i] = structuredClone(pregnancy);
    changed();
  },
  async end_pregnancy({ id }: { id: string }) {
    endPregnancy(id);
    changed();
  },
  async give_birth({ id }: { id: string }) {
    endPregnancy(id);
    const p = data.pregnancies.find((x) => x.id === id);
    if (p) p.born = true;
    changed();
  },
  async delete_pregnancy({ id }: { id: string }) {
    endPregnancy(id);
    data.pregnancies = data.pregnancies.filter((p) => p.id !== id);
    changed();
  },
  async set_current_pregnancy({ avatarId, id }: { avatarId: string | null; id: string | null }) {
    setCurrent(avatarId, id);
    changed();
  },
  async set_partner({ pregnancyId, person, name }: { pregnancyId: string; person: Person | null; name: string | null }) {
    const p = data.pregnancies.find((x) => x.id === pregnancyId);
    if (!p) return;
    const candidates = p.partner.candidates;
    // A pregnancy from in-game fun: credit that moment to whoever is chosen.
    const f = p.conceivedMs != null ? data.funLog.find((x) => x.timeMs === p.conceivedMs) : undefined;
    if (f) {
      f.partnerName = person?.name ?? (name?.trim() || null);
      f.partnerUserId = person?.userId ?? null;
    }
    if (person) p.partner = { status: "confirmed", name: person.name, userId: person.userId, avatarName: person.avatarName, species: null, speciesSource: "guess", candidates };
    else if (name?.trim()) p.partner = { status: "manual", name: name.trim(), userId: null, avatarName: null, species: null, speciesSource: "fallback", candidates };
    else p.partner = unknownPartner(candidates);
    changed();
  },
  async save_avatar({ profile }: { profile: AvatarProfile }) {
    data.avatars[profile.id] = profile;
    changed();
  },
  async delete_avatar({ id }: { id: string }) {
    delete data.avatars[id];
    pruneGroups();
    changed();
  },
  async create_group({ name, avatarIds }: { name: string; avatarIds: string[] }) {
    const id = newId("grp");
    data.groups[id] = { id, name: name.trim() || `Group ${Object.keys(data.groups).length + 1}`, pregnancyId: null };
    try {
      avatarIds.forEach((a) => setAvatarGroup(a, id));
    } finally {
      pruneGroups();
      changed();
    }
    return id;
  },
  async set_avatar_group({ avatarId, groupId }: { avatarId: string; groupId: string | null }) {
    setAvatarGroup(avatarId, groupId);
    changed();
  },
  async rename_group({ id, name }: { id: string; name: string }) {
    if (data.groups[id] && name.trim()) data.groups[id].name = name.trim();
    changed();
  },
  async delete_group({ id }: { id: string }) {
    const members = groupMembers(data, id);
    if (members[0]) movePregnancy(id, members[0]);
    members.forEach((m) => (data.avatars[m].groupId = null));
    delete data.groups[id];
    data.waiting = data.waiting.filter((w) => w.avatarId !== id);
    changed();
  },
  async set_species_answer() {},
  async forget_person({ name, userId }: { name: string; userId: string | null }) {
    const same = (n: string | null, u: string | null) => (userId && u ? u === userId : n === name);
    data.instance.history = data.instance.history.filter((p) => !same(p.name, p.userId));
    data.funLog = data.funLog.filter((f) => !same(f.partnerName, f.partnerUserId));
    for (const p of data.pregnancies) {
      p.partner.candidates = p.partner.candidates.filter((c) => !same(c.name, c.userId));
      if (same(p.partner.name, p.partner.userId)) p.partner = { ...p.partner, status: "unknown", name: null, userId: null, avatarName: null, species: null };
    }
    changed();
  },
  async import_community_table() {
    return 0;
  },
  async save_journal_entry({ entry }: { entry: JournalEntry }) {
    return entry;
  },
  async delete_journal_entry() {},
  async add_keepsake({ keepsake }: { keepsake: Keepsake }) {
    return keepsake;
  },
  async pulse() {},
  async export_backup() {
    return JSON.stringify(data, null, 2);
  },
  async reset_data({ kind }: { kind: ResetKind }) {
    const fresh = emptyData();
    if (kind === "everything") data = fresh;
    if (kind === "avatars") {
      data.avatars = {};
      data.groups = {};
      data.pregnancies.forEach((p) => (p.avatarId = null));
    }
    if (kind === "pregnancies") {
      Object.assign(data, { pregnancies: [], activePregnancyId: null, waiting: [], lastReveal: null, journal: [], keepsakes: [] });
      Object.values(data.avatars).forEach((a) => (a.pregnancyId = null));
      Object.values(data.groups).forEach((g) => (g.pregnancyId = null));
    }
    if (kind === "partners") {
      data.funLog = [];
      data.instance.history = [];
      data.speciesAnswers = {};
    }
    if (kind === "achievements") data.achievements = {};
    if (kind === "settings") data.settings = fresh.settings;
    changed();
  },
  async import_backup({ json }: { json: string }) {
    data = { ...emptyData(), ...JSON.parse(json) };
    changed();
  },
  async get_mapping(): Promise<MappingInfo> {
    if (!mapping) mapping = (await import("../../config/osc-parameters.json")).default as unknown as Mapping;
    const bits = (all: boolean) =>
      Object.values(mapping!.parameters)
        .filter((p) => p.name && p.synced && (all || !p.optional))
        .reduce((s, p) => s + (p.type === "bool" ? 1 : 8), 0);
    return { mapping: structuredClone(mapping), path: "config/osc-parameters.json (preview)", syncedBits: bits(false), syncedBitsAll: bits(true) };
  },
  async save_mapping({ mapping: m }: { mapping: Mapping }) {
    // Same rule as the app: app parameters can't be in Sync.
    const app = new Set(Object.values(m.parameters).flatMap((p) => [p.name, ...(p.aliases ?? [])]).filter(Boolean));
    const bad = m.sync.filter((s) => app.has(s.name)).map((s) => s.name);
    if (bad.length) throw new Error(`${bad.join(", ")} is an app parameter, so it can't be in Sync.`);
    mapping = m;
  },
  async reset_mapping() {
    mapping = null;
  },
  async reload_mapping() {},
  async open_path() {},
  async default_log_dirs() {
    return [];
  },
  async quit_app() {},
  async app_version() {
    return "0.1.0";
  },
  // Pretend there's always a newer version so the Updates card can be previewed.
  async check_update() {
    return { version: "0.2.0", notes: "Browser preview: a pretend update." };
  },
  async install_update() {},
  // The browser preview has no network access to VRChat; show letters.
  async partner_picture() {
    return null;
  },
  async avatar_preview() {
    return null;
  },
  async add_avatar({ avatarId, name }: { avatarId: string; name: string | null }) {
    if (!/^avtr_[0-9a-f-]{36}$/i.test(avatarId.trim())) throw new Error("That isn't an avatar ID. It looks like avtr_ followed by letters and numbers.");
    if (data.avatars[avatarId]) throw new Error("That avatar is already in your list.");
    const p = { ...prof(avatarId.trim(), name?.trim() || "Unnamed avatar"), lastSeenMs: 0, params: [] as string[] };
    data.avatars[p.id] = p;
    changed();
    return p;
  },
  async dismiss_reveal() {
    if (data.lastReveal) data.lastReveal.seen = true;
    changed();
  },
  async restart_cycle() {
    data.settings.cycleAnchorMs = Date.now();
    changed();
  },
  async skip_waiting() {
    resolveDue(true);
  },
  async debug_woohoo({ outcome }: { outcome: boolean | null }) {
    await woohoo(outcome);
  },
  async debug_add_person({ name, userId }: { name: string; userId: string | null }) {
    if (!name.trim()) throw new Error("Give the test partner a name.");
    if (userId && !/^usr_[0-9a-f-]{36}$/i.test(userId.trim())) throw new Error("That isn't a user ID. It looks like usr_ followed by letters and numbers.");
    data.instance.present.push({ name: name.trim(), userId: userId?.trim() || null, avatarName: null, lastSeenMs: Date.now(), worldName: data.instance.worldName });
    changed();
    emit("instance-changed");
  },
  async debug_remove_person({ name, userId }: { name: string; userId: string | null }) {
    data.instance.present = data.instance.present.filter((p) => !(p.name === name && p.userId === userId));
    changed();
    emit("instance-changed");
  },
  async debug_picture() {
    throw new Error("The browser preview can't reach vrchat.com. Try this in the installed app.");
  },
  async debug_kick({ strength }: { strength: "weak" | "strong" | "extreme" }) {
    const p = pregnancyFor(data, data.currentAvatarId);
    if (p) {
      (p.kicks ??= []).push({ timeMs: Date.now(), strength });
      changed();
    }
    emit("kick", strength);
  },
  async debug_reveal_now() {
    data.waiting.forEach((w) => (w.revealMs = Date.now()));
    changed();
  },
  async debug_set_cycle_day({ day }: { day: number }) {
    data.settings.cycleAnchorMs = Date.now() - ((Math.min(28.99, Math.max(1, day)) - 1) / 28) * cycleMs(data.settings);
    changed();
  },
  /** Preview helper: pretend a WooHoo happened. */
  async simulate_conception() {
    await woohoo(null);
  },
  /** Preview helper: pretend you switched avatar in VRChat. */
  async simulate_avatar({ id }: { id: string }) {
    data.currentAvatarId = id;
    changed();
  },
};

async function woohoo(forced: boolean | null) {
  {
    const now = Date.now();
    const s = data.settings;
    if (pregnancyFor(data, data.currentAvatarId)) return;
    const partner = guessPartner();
    data.funLog.push({ timeMs: now, partnerName: partner.name, partnerUserId: partner.userId, avatarId: data.currentAvatarId, worldName: data.instance.worldName });
    if (!s.fertility) {
      createPregnancy({ source: "conception", partner });
      changed();
      emit("conception");
      return;
    }
    s.cycleAnchorMs ??= now;
    const day = cycleDay(now, s.cycleAnchorMs, cycleMs(s));
    const c = chance(day, s.fertilityPeak);
    const existing = pendingFor(data, data.currentAvatarId);
    const positive = forced ?? Math.random() < c;
    if (existing) {
      if (!existing.positive && positive) Object.assign(existing, { positive, timeMs: now, partner, worldName: data.instance.worldName });
    } else {
      data.waiting.push({ timeMs: now, revealMs: now + graceMs(s), chance: c, cycleDay: day, positive, babyCount: rollBabies(), partner, avatarId: owner(data, data.currentAvatarId), worldName: data.instance.worldName });
    }
    changed();
    emit("fun");
  }
}
