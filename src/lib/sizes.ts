/** How big the baby is by week, as something from the kitchen. Each entry holds until the next. */
const SIZES: [week: number, emoji: string, what: string][] = [
  [4, "🌱", "poppy seed"],
  [5, "🌱", "sesame seed"],
  [6, "🫘", "lentil"],
  [7, "🫐", "blueberry"],
  [8, "🫒", "olive"],
  [9, "🍒", "cherry"],
  [10, "🍓", "strawberry"],
  [11, "🥝", "kiwi"],
  [12, "🍋", "lemon"],
  [13, "🍑", "peach"],
  [14, "🍎", "apple"],
  [15, "🍊", "orange"],
  [16, "🥑", "avocado"],
  [17, "🍐", "pear"],
  [18, "🫑", "bell pepper"],
  [19, "🥭", "mango"],
  [20, "🍌", "banana"],
  [21, "🥕", "carrot"],
  [22, "🍠", "sweet potato"],
  [23, "🍊", "grapefruit"],
  [24, "🌽", "ear of corn"],
  [25, "🥦", "head of broccoli"],
  [26, "🥬", "lettuce"],
  [27, "🥒", "cucumber"],
  [28, "🍆", "eggplant"],
  [29, "🥥", "coconut"],
  [31, "🍍", "pineapple"],
  [33, "🍈", "melon"],
  [36, "🎃", "pumpkin"],
  [39, "🍉", "watermelon"],
];

/** "🫐 About the size of a blueberry" (with `each` for twins and more). */
export function babySize(week: number, each = false): string {
  const at = [...SIZES].reverse().find(([w]) => week >= w);
  if (!at) return "✨ Just a tiny spark so far";
  const [, emoji, what] = at;
  const a = /^[aeiou]/.test(what) ? "an" : "a";
  return `${emoji} ${each ? "Each about" : "About"} the size of ${a} ${what}`;
}
