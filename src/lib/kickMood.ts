// Babies' personalities, mirroring crates/pregsys-core/src/movement.rs.

import type { KickMood, Pregnancy } from "./types";

/** FNV-1a: a stable number from text (movement.rs `hash`). */
export function hashText(text: string): bigint {
  let h = 0xcbf29ce484222325n;
  for (const b of new TextEncoder().encode(text)) {
    h ^= BigInt(b);
    h = (h * 0x100000001b3n) & 0xffffffffffffffffn;
  }
  return h;
}

/** The personality a pregnancy gets when none was picked (movement.rs `mood_for`). */
export function moodFor(id: string): KickMood {
  const n = Number(hashText(id) % 100n);
  return n < 20 ? "sleepy" : n < 50 ? "calm" : n < 80 ? "wiggly" : "bouncy";
}

/** Picked by hand, or from the ID (model.rs `Pregnancy::kick_mood`). */
export const kickMood = (p: Pick<Pregnancy, "id" | "kickMood">): KickMood => p.kickMood ?? moodFor(p.id);

export const KICK_MOODS: { k: KickMood; emoji: string; name: string; hint: string }[] = [
  { k: "sleepy", emoji: "😴", name: "Sleepy", hint: "kicks now and then, gently" },
  { k: "calm", emoji: "🌙", name: "Calm", hint: "a bit quieter than most" },
  { k: "wiggly", emoji: "🐛", name: "Wiggly", hint: "a bit busier than most" },
  { k: "bouncy", emoji: "⚡", name: "Bouncy", hint: "always on the move, kicks hard" },
];

export const moodInfo = (k: KickMood) => KICK_MOODS.find((m) => m.k === k)!;
