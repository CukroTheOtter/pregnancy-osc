// Achievements as shown in the app. Unlocking happens in the Rust core
// (crates/pregsys-core/src/achievements.rs); keep the ids and titles in step.

export interface AchievementInfo {
  id: string;
  emoji: string;
  title: string;
  hint: string;
}

export const ACHIEVEMENTS: AchievementInfo[] = [
  { id: "first_woohoo", emoji: "💕", title: "First WooHoo", hint: "WooHoo for the first time" },
  { id: "busy_bee", emoji: "🐝", title: "Busy Bee", hint: "25 WooHoos" },
  { id: "little_bean", emoji: "🫘", title: "Little Bean", hint: "Your first pregnancy" },
  { id: "double_trouble", emoji: "👯", title: "Double Trouble", hint: "Carry twins" },
  { id: "triple_treat", emoji: "🎉", title: "Triple Treat", hint: "Carry triplets or more" },
  { id: "welcome_home", emoji: "🍼", title: "Welcome Home", hint: "Welcome your first baby" },
  { id: "full_house", emoji: "🏡", title: "Full House", hint: "5 babies born" },
  { id: "fashionably_late", emoji: "⏰", title: "Fashionably Late", hint: "Go past your due date" },
  { id: "speedrun", emoji: "⚡", title: "Speedrun", hint: "A birth within a day of conception" },
  { id: "long_haul", emoji: "🐢", title: "The Long Haul", hint: "A birth after 9 real months" },
  { id: "social_butterfly", emoji: "🦋", title: "Social Butterfly", hint: "5 different partners" },
  { id: "soulmates", emoji: "💞", title: "Soulmates", hint: "3 pregnancies with the same partner" },
  { id: "kick_counter", emoji: "👣", title: "Kick Counter", hint: "Feel 100 kicks" },
  { id: "wardrobe", emoji: "👗", title: "Wardrobe Change", hint: "Wear 5 different Pregnancy System avatars" },
  { id: "shape_shifter", emoji: "🔗", title: "Shape Shifter", hint: "Link 3 avatars together" },
];
