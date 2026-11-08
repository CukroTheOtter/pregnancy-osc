// Achievements as shown in the app. Unlocking happens in the Rust core
// (crates/pregsys-core/src/achievements.rs); keep the ids and titles in step.

/** How rare and hard it is: 1 Common to 5 Legendary. Picked by hand, from the odds and the effort. */
export type Tier = 1 | 2 | 3 | 4 | 5;

export const TIERS: Record<Tier, string> = { 1: "Common", 2: "Uncommon", 3: "Rare", 4: "Epic", 5: "Legendary" };

export interface AchievementInfo {
  id: string;
  tier: Tier;
  emoji: string;
  title: string;
  hint: string;
  /** A surprise: shown instead of the hint until it's unlocked. */
  secret?: string;
}

const LIST: AchievementInfo[] = [
  { id: "first_woohoo", tier: 1, emoji: "💕", title: "First WooHoo", hint: "WooHoo for the first time" },
  { id: "busy_bee", tier: 2, emoji: "🐝", title: "Busy Bee", hint: "25 WooHoos" },
  { id: "little_bean", tier: 1, emoji: "🫘", title: "Little Bean", hint: "Your first pregnancy" },
  { id: "double_trouble", tier: 3, emoji: "👯", title: "Double Trouble", hint: "Carry twins" },
  { id: "triple_treat", tier: 4, emoji: "🎉", title: "Triple Treat", hint: "Carry triplets or more" },
  { id: "welcome_home", tier: 1, emoji: "🍼", title: "Welcome Home", hint: "Welcome your first baby" },
  { id: "full_house", tier: 3, emoji: "🏡", title: "Full House", hint: "5 babies born" },
  { id: "fashionably_late", tier: 1, emoji: "⏰", title: "Fashionably Late", hint: "Go past your due date" },
  { id: "splash", tier: 3, emoji: "💦", title: "Splash!", hint: "Your water breaks before labor gets going", secret: "Sometimes things start with a surprise…" },
  { id: "right_on_time", tier: 3, emoji: "📅", title: "Right On Time", hint: "A birth on the due date" },
  { id: "early_bird", tier: 3, emoji: "🐣", title: "Early Bird", hint: "A birth before week 37" },
  { id: "right_here_with_me", tier: 2, emoji: "🤝", title: "Right Here With Me", hint: "The other parent is there for the birth" },
  { id: "speedrun", tier: 2, emoji: "⚡", title: "Speedrun", hint: "A birth within a day of conception" },
  { id: "long_haul", tier: 5, emoji: "🐢", title: "The Long Haul", hint: "A birth after 9 real months" },
  { id: "social_butterfly", tier: 2, emoji: "🦋", title: "Social Butterfly", hint: "WooHoos or pregnancies with 5 different people" },
  { id: "soulmates", tier: 3, emoji: "💞", title: "Soulmates", hint: "3 pregnancies with the same person" },
  { id: "kick_counter", tier: 2, emoji: "👣", title: "Kick Counter", hint: "Feel 100 kicks" },
  { id: "wardrobe", tier: 2, emoji: "👗", title: "Wardrobe Change", hint: "Wear 5 different Pregnancy System avatars" },
  { id: "shape_shifter", tier: 2, emoji: "🔗", title: "Shape Shifter", hint: "Link 3 avatars together" },
  { id: "first_try", tier: 3, emoji: "🎯", title: "First Try", hint: "Your first WooHoo with a saved result leads to a pregnancy", secret: "Sometimes you just get lucky…" },
  { id: "never_give_up", tier: 3, emoji: "🌱", title: "Never Give Up", hint: "A pregnancy after 5 \"Not this time\" results", secret: "Keep trying…" },
  { id: "cant_stop", tier: 2, emoji: "😅", title: "Can't Stop", hint: "5 WooHoos in one wait", secret: "Waiting is hard. Maybe you got impatient…" },
  { id: "whos_your_daddy", tier: 1, emoji: "🎲", title: "Who's Your Daddy?", hint: "A pregnancy where the other parent was picked at random", secret: "Who could it have been?…" },
  { id: "century_club", tier: 4, emoji: "💯", title: "Century Club", hint: "100 WooHoos" },
  { id: "four_leaf_clover", tier: 5, emoji: "🍀", title: "Four Leaf Clover", hint: "Carry quadruplets", secret: "A very, very full house…" },
  { id: "one_of_each", tier: 4, emoji: "👫", title: "One Of Each", hint: "Twins or more with both a boy and a girl", secret: "A little bit of both…" },
  { id: "baby_boom", tier: 4, emoji: "🏟️", title: "Baby Boom", hint: "10 babies born" },
  { id: "little_athlete", tier: 4, emoji: "⚽", title: "Little Athlete", hint: "Feel 500 kicks" },
  { id: "globetrotter", tier: 3, emoji: "🌍", title: "Globetrotter", hint: "WooHoos in 10 different worlds" },
  { id: "favorite_spot", tier: 3, emoji: "🏠", title: "Favorite Spot", hint: "10 WooHoos in the same world" },
  { id: "taking_a_break", tier: 1, emoji: "☕", title: "Taking A Break", hint: "A birth after pausing that pregnancy", secret: "Everyone needs a rest sometimes…" },
];

/** Easiest first. */
export const ACHIEVEMENTS = [...LIST].sort((a, b) => a.tier - b.tier);
