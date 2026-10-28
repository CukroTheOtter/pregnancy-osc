// Factories for tests and the browser preview.

import type { AppData, Baby, Pregnancy, Settings } from "./types";
import { newTimeline } from "./timeline";

export function defaultSettings(): Settings {
  return {
    syncScope: "global",
    syncWaitMs: 2000,
    userSpecies: "human",
    hybridChance: 0.05,
    spoilerMode: "surprise",
    theme: { palette: "strawberry", mode: "system", accent: "#f59ac0", customAccent: false },
    startWithSteamvr: false,
    startWithOs: false,
    startInTray: false,
    exitWithSteamvr: false,
    heartbeatSound: false,
    soundVolume: 0.5,
    revealPulse: true,
    kickEvents: true,
    notifications: true,
    defaultMode: "custom",
    defaultSpeed: 1,
    defaultCustomMs: 30 * 86_400_000,
    settleMinutes: 10,
    hideBelly: false,
    bellyScale: 1,
    fetalSounds: null,
    osc: { enabled: true, useOscquery: true, sendHost: "127.0.0.1", sendPort: 9000, recvPort: 9001 },
    chatbox: {
      enabled: false,
      sound: false,
      showSeconds: 8,
      status: false,
      statusText: "🤰 Week {week} of 40",
      statusPartner: true,
      debug: {},
      messages: {
        started: { on: true, text: "🤰 A little one is on the way!" },
        paused: { on: true, text: "⏸️ Pregnancy paused (week {week})" },
        resumed: { on: true, text: "▶️ Pregnancy resumed (week {week})" },
        stopped: { on: true, text: "Pregnancy stopped" },
        labor: { on: false, text: "🤰 Labor has started!" },
        waterBroke: { on: false, text: "💦 Water broke!" },
        born: { on: true, text: "🍼 The baby is here!" },
        woohoo: { on: true, text: "💕 WooHoo!" },
        notPregnant: { on: true, text: "Not pregnant this time" },
        achievement: { on: true, text: "🏆 {achievement}" },
      },
    },
    logDir: null,
    readLogs: true,
    showPictures: false,
    fertility: true,
    fertilityPeak: 0.6,
    cycleAnchorMs: null,
    multiples: "playful",
    biggerForMultiples: true,
    growWhenOverdue: true,
    genderMode: "random",
    debug: false,
    kicks: true,
    kickFrequency: "normal",
    allowedKicks: { weak: true, strong: true, extreme: true },
    kicksWithPartner: false,
    autoAddAvatars: true,
    closeToTray: true,
    labor: { start: "random", induceWeek: null, minutes: 20, waitsForPartner: false, woohooStarts: false, quietEarly: false, restingHolds: false },
    sps: { enabled: true, types: { vaginal: true, anal: false, oral: false, other: false }, depth: 0.3, seconds: 20, gapSeconds: 120 },
    onboarded: false,
  };
}

export function emptyData(): AppData {
  return {
    version: 1,
    settings: defaultSettings(),
    pregnancies: [],
    activePregnancyId: null,
    avatars: {},
    groups: {},
    speciesAnswers: {},
    communitySpecies: {},
    journal: [],
    keepsakes: [],
    instance: { local: null, worldId: null, worldName: null, instance: null, joinedMs: null, present: [], history: [] },
    currentAvatarId: null,
    waiting: [],
    lastReveal: null,
    funLog: [],
    syncValues: {},
    achievements: {},
  };
}

export function baby(index: number, extra: Partial<Baby> = {}): Baby {
  return { index, name: null, mode: "rolled", manualSpecies: null, locks: {}, reroll: 0, ...extra };
}

export function pregnancy(extra: Partial<Pregnancy> = {}): Pregnancy {
  const createdMs = extra.createdMs ?? 1_700_000_000_000;
  const count = extra.babyCount ?? 1;
  return {
    id: "p1",
    name: "Little Bean",
    createdMs,
    timeline: newTimeline(createdMs, "accelerated", 7),
    babyCount: count,
    kind: "human",
    motherSpecies: "human",
    partner: { status: "unknown", name: null, userId: null, avatarName: null, species: null, speciesSource: "fallback", candidates: [] },
    babies: Array.from({ length: count }, (_, i) => baby(i)),
    reveals: {},
    source: "manual",
    avatarId: null,
    worldName: null,
    endedMs: null,
    born: false,
    conceivedMs: null,
    genders: [],
    kicks: [],
    milestones: [],
    oscHints: { speciesIndex: null },
    ...extra,
  };
}
