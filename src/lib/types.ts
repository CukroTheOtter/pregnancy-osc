// Mirrors crates/pregsys-core/src/model.rs (camelCase on disk).

export type DurationMode = "realtime" | "accelerated" | "custom";
export type SyncScope = "global" | "perAvatar";
export type SpoilerMode = "surprise" | "hints" | "all";
export type ThemeMode = "system" | "light" | "dark";
export type BodyKind = "human" | "furry" | "egg";
export type ResetKind = "avatars" | "pregnancies" | "partners" | "achievements" | "settings" | "everything";
export type PartnerStatus = "guessed" | "confirmed" | "unknown" | "manual" | "random";
export type SpeciesSource = "guess" | "confirmed" | "fallback" | "manual";
export type BabyMode = "rolled" | "manual";
export type PregnancySource = "manual" | "conception";
export type Multiples = "realistic" | "playful" | "lots";
export type GenderMode = "off" | "random";
export type KickStrength = "weak" | "strong" | "extreme";
export type KickFrequency = "calm" | "normal" | "active";
export type HoleType = "vaginal" | "anal" | "oral" | "other";

export interface Timeline {
  startMs: number;
  durationMs: number;
  mode: DurationMode;
  speed: number;
  pausedAtMs: number | null;
  pausedTotalMs: number;
}

export interface Theme {
  palette: string;
  mode: ThemeMode;
  accent: string;
}

export interface OscSettings {
  enabled: boolean;
  useOscquery: boolean;
  sendHost: string;
  sendPort: number;
  recvPort: number;
}

export interface ChatMessage {
  on: boolean;
  text: string;
}

export interface ChatboxSettings {
  enabled: boolean;
  sound: boolean;
  showSeconds: number;
  status: boolean;
  statusText: string;
  statusPartner: boolean;
  messages: Record<string, ChatMessage>;
  debug: Record<string, boolean>;
}

export interface Settings {
  syncScope: SyncScope;
  syncWaitMs: number;
  userSpecies: string;
  hybridChance: number;
  spoilerMode: SpoilerMode;
  theme: Theme;
  startWithSteamvr: boolean;
  startWithOs: boolean;
  startInTray: boolean;
  exitWithSteamvr: boolean;
  heartbeatSound: boolean;
  soundVolume: number;
  revealPulse: boolean;
  kickEvents: boolean;
  notifications: boolean;
  defaultMode: DurationMode;
  defaultSpeed: number;
  defaultCustomMs: number | null;
  settleMinutes: number;
  hideBelly: boolean;
  bellyScale: number;
  fetalSounds: boolean | null;
  osc: OscSettings;
  chatbox: ChatboxSettings;
  logDir: string | null;
  readLogs: boolean;
  showPictures: boolean;
  fertility: boolean;
  fertilityPeak: number;
  cycleAnchorMs: number | null;
  multiples: Multiples;
  biggerForMultiples: boolean;
  growWhenOverdue: boolean;
  genderMode: GenderMode;
  debug: boolean;
  kicks: boolean;
  kickFrequency: KickFrequency;
  allowedKicks: { weak: boolean; strong: boolean; extreme: boolean };
  sps: { enabled: boolean; types: Record<HoleType, boolean>; depth: number; seconds: number; gapSeconds: number };
  onboarded: boolean;
}

export interface Person {
  name: string;
  userId: string | null;
  avatarName: string | null;
  lastSeenMs: number;
  worldName?: string | null;
}

export interface Partner {
  status: PartnerStatus;
  name: string | null;
  userId: string | null;
  avatarName: string | null;
  species: string | null;
  speciesSource: SpeciesSource;
  candidates: Person[];
}

export interface SpeciesBlend {
  parts: Record<string, number>;
}

export interface Baby {
  index: number;
  name: string | null;
  mode: BabyMode;
  manualSpecies: SpeciesBlend | null;
  locks: Record<string, string>;
  reroll: number;
}

export interface Pregnancy {
  id: string;
  name: string;
  createdMs: number;
  timeline: Timeline;
  babyCount: number;
  kind: BodyKind;
  motherSpecies: string;
  partner: Partner;
  babies: Baby[];
  reveals: Record<string, number>;
  source: PregnancySource;
  avatarId: string | null;
  worldName: string | null;
  endedMs: number | null;
  born: boolean;
  conceivedMs: number | null;
  genders: string[];
  kicks: { timeMs: number; strength: KickStrength }[];
  milestones: string[];
  oscHints: { speciesIndex: number | null };
}

export interface AvatarProfile {
  id: string;
  name: string | null;
  autoName?: boolean;
  species: string | null;
  kind: BodyKind | null;
  hideBelly: boolean | null;
  bellyScale: number | null;
  fetalSounds: boolean | null;
  pregnancyId: string | null;
  followGlobal: boolean;
  /** The group it's in (avatars in a group share one pregnancy). */
  groupId: string | null;
  paramNames: Record<string, string>;
  sps: { custom: boolean; sockets: string[]; types: Record<string, HoleType> };
  lastSeenMs: number;
  params: string[];
}

export interface JournalEntry {
  id: string;
  timeMs: number;
  pregnancyId: string | null;
  kind: "note" | "milestone" | "reveal" | "event" | string;
  title: string;
  text: string;
  image: string | null;
}

export interface Keepsake {
  id: string;
  pregnancyId: string;
  babyIndex: number;
  traitKey: string;
  value: string;
  week: number;
  timeMs: number;
}

export interface InstanceTracker {
  local: Person | null;
  worldId: string | null;
  worldName: string | null;
  instance: string | null;
  joinedMs: number | null;
  present: Person[];
  history: Person[];
}

export interface PendingConception {
  timeMs: number;
  revealMs: number;
  chance: number;
  cycleDay: number;
  /** Secret until the reveal: the UI never shows it. */
  positive: boolean;
  babyCount: number;
  partner: Partner;
  avatarId: string | null;
  worldName: string | null;
}

export interface RevealResult {
  timeMs: number;
  eventMs: number;
  positive: boolean;
  seen: boolean;
}

export interface FunEvent {
  timeMs: number;
  partnerName: string | null;
  partnerUserId: string | null;
  avatarId: string | null;
  worldName: string | null;
}

export interface AvatarGroup {
  id: string;
  name: string;
  pregnancyId: string | null;
}

export interface AppData {
  version: number;
  settings: Settings;
  pregnancies: Pregnancy[];
  activePregnancyId: string | null;
  avatars: Record<string, AvatarProfile>;
  groups: Record<string, AvatarGroup>;
  speciesAnswers: Record<string, string>;
  communitySpecies: Record<string, string>;
  journal: JournalEntry[];
  keepsakes: Keepsake[];
  instance: InstanceTracker;
  currentAvatarId: string | null;
  waiting: PendingConception[];
  lastReveal: RevealResult | null;
  funLog: FunEvent[];
  syncValues: Record<string, number>;
  achievements: Record<string, number>;
}

/** A newer version on GitHub, ready to install. */
export interface UpdateInfo {
  version: string;
  notes: string | null;
}

export interface Status {
  vrchatRunning: boolean;
  steamvrRunning: boolean;
  oscActive: boolean;
  oscquery: boolean;
  vrchatFound: boolean;
  sendTarget: string | null;
  recvPort: number | null;
  backupPort: number | null;
  queryPort: number | null;
  vrchatQuery: string | null;
  lastHeardMs: number | null;
  avatarId: string | null;
  avatarParams: string[] | null;
  /** Switching avatars: the new one is still loading. */
  avatarLoading: boolean;
  /** The Hide Belly Sync parameter's name (it can be renamed). */
  hideBelly: string | null;
  lastStage: number | null;
  lastExtra: number | null;
  logDir: string | null;
  logFile: string | null;
  worldName: string | null;
  others: Person[];
  note: string | null;
  trayAvailable: boolean;
  spsFound: string[];
  spsLive: { name: string; channel: string; value: number; holeType: HoleType; counts: boolean }[];
  spsProgress: number;
  mappingPath: string;
  dataDir: string;
}

export interface NewPregnancy {
  name?: string | null;
  startMs?: number | null;
  mode?: DurationMode | null;
  speed?: number | null;
  customMs?: number | null;
  babyCount?: number | null;
  kind?: BodyKind | null;
  motherSpecies?: string | null;
  source?: PregnancySource;
  avatarId?: string | null;
  makeCurrent?: boolean | null;
}

export interface ParamDef {
  name: string;
  type: "float" | "int" | "bool";
  synced?: boolean;
  required?: boolean;
  optional?: boolean;
  aliases?: string[];
}

export type SyncRole = "hideBelly" | "fetalSounds";

export interface Mapping {
  version: number;
  comment?: string;
  sendIntervalMs: number;
  minStageChange: number;
  heartbeat: { intervalMs: number; timeoutMs: number };
  overdue: { maxOverdueFraction: number };
  parameters: Record<string, ParamDef>;
  /** `role`: a built-in entry (renamable, never removed, always on/off). */
  sync: { name: string; type: "float" | "int" | "bool"; role?: SyncRole }[];
  kicks: { weak: number; strong: number; extreme: number; holdMs: number };
}

export interface AvatarPreview {
  name: string | null;
  author: string | null;
  picture: string | null;
  public: boolean;
}

export interface MappingInfo {
  mapping: Mapping;
  path: string;
  syncedBits: number;
  syncedBitsAll: number;
}
