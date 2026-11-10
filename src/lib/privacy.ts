// Privacy mode: for streaming and screenshots. People get a made-up name
// ("Sleepy Otter"), the same one every time so two partners still look
// different; their pictures are blurred (PersonPic), and your own name
// becomes "You". Avatar and world names stay.

import type { AppData } from "./types";
import { autoPrivacyReason } from "./autoPrivacy.svelte";

type Data = Pick<AppData, "settings"> | null | undefined;

/** On in Settings, or on by itself while a recording app runs (`autoPrivacy`). */
export const isPrivate = (d: Data): boolean => !!d?.settings.privacyMode || autoPrivacyReason() != null;

/** Privacy mode hides you too (unless "Still show me" is on). */
export const hidesMe = (d: Data): boolean => isPrivate(d) && !d?.settings.privacyShowMe;

const ADJECTIVES = [
  "Sleepy", "Fluffy", "Bouncy", "Cozy", "Sparkly", "Giggly", "Snuggly", "Wiggly",
  "Sunny", "Comfy", "Fuzzy", "Peppy", "Dreamy", "Silly", "Zippy", "Cuddly",
  "Squishy", "Toasty", "Wobbly", "Chirpy", "Dizzy", "Jolly", "Mellow", "Perky",
  "Sneaky", "Bubbly", "Dapper", "Frosty", "Glowy", "Happy", "Lucky", "Misty",
  "Nifty", "Plushy", "Rosy", "Shiny", "Spicy", "Sweet", "Tiny", "Witty",
  "Breezy", "Crispy", "Daring", "Gentle", "Humble", "Mighty", "Quirky", "Velvet",
];
const ANIMALS = [
  "Otter", "Bunny", "Fox", "Panda", "Kitten", "Puppy", "Raccoon", "Hedgehog",
  "Penguin", "Koala", "Duckling", "Hamster", "Seal", "Owl", "Lamb", "Fawn",
  "Ferret", "Squirrel", "Axolotl", "Capybara", "Corgi", "Dragon", "Moth", "Frog",
  "Goose", "Lemur", "Lynx", "Mole", "Narwhal", "Quokka", "Red Panda", "Sloth",
  "Tanuki", "Turtle", "Wombat", "Yak", "Badger", "Bat", "Bee", "Crow",
  "Deer", "Gecko", "Kiwi", "Llama", "Moose", "Mouse", "Pup", "Wolf",
];

/** A number from someone's ID or name: the same person, the same number. */
function hash(key: string): number {
  let h = 2166136261;
  for (let i = 0; i < key.length; i++) h = Math.imul(h ^ key.charCodeAt(i), 16777619);
  return h >>> 0;
}

/** A made-up name for someone, always the same for the same person. */
export function fakeName(key: string): string {
  const h = hash(key);
  return `${ADJECTIVES[h % ADJECTIVES.length]} ${ANIMALS[Math.floor(h / ADJECTIVES.length) % ANIMALS.length]}`;
}

/** Someone's name as shown: theirs, or a made-up one in privacy mode. */
export function shownName(d: Data, name: string, userId?: string | null): string;
export function shownName(d: Data, name: string | null, userId?: string | null): string | null;
export function shownName(d: Data, name: string | null, userId: string | null = null): string | null {
  if (!name || !isPrivate(d)) return name;
  return fakeName(userId ?? name.trim().toLowerCase());
}

/** Your own name as shown: yours, or "You" in privacy mode (unless it still shows you). */
export function myShownName(d: Data, name: string | null): string | null {
  return name && hidesMe(d) ? "You" : name;
}
