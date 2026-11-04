# Pregnancy OSC: the plan

Agreed layout and features. Built in the order at the bottom.

## Tabs
**Home · Partners · Avatars · Settings**. The header shows the avatar you're wearing and the link status. Tapping the avatar name lets you view another avatar.

## Home (one avatar, or a linked group)
| State | Shows |
|---|---|
| Not pregnant | **Cycle card**: ring "Day 12 of 28" with the real time left in the cycle, chance now (Low / Medium / High / Peak), "next fertile window in …". Length card. Small "start manually". |
| Waiting | "💕 You WooHoo'd with **Ember** at Cozy Cabin." Countdown "You'll find out in 1 day 4 h" (real time, with the date and time). No hint of the outcome. Belly held flat. |
| Any state | **💕 WooHoo history** (folds away, only with a result on this avatar or its group): one row per wait, since a wait has one result. Its first WooHoo's time and world, the partner ("Partner unknown" if nobody was picked), how many WooHoos, and ⏳ Waiting / 🤰 Pregnant / 🤍 Not this time. WooHoos from before results were saved aren't listed (the totals still count them). |
| Reveal (once) | "You're pregnant! 💕" + **estimated due date**, or "Not this time 🤍". OK button. |
| Pregnant | Progress ring (week of 40, plus the real time left: weeks are scaled to the pregnancy length, hours and days are always real), **babies: "Too early to tell" → "It's twins! 👶👶"** (from about week 12), due date, Length, Partner (with "3rd time together · 2 kids"). At the due date: **"Welcome your baby 🍼"**. Small "End early". Shared avatars listed. |

## Me
Stats and achievements (1 Common to 5 Legendary; surprises show a vague hint until unlocked). **📤 Share** draws a picture on your PC, in your app theme: your name and picture, counts, your 3 rarest as rows, and the rest as small icons. Copy or download it. Achievements unlocked while debug mode is on get a 🛠 and stay out of the count and the picture (Debug can clear them). Name, picture and WooHoo count can be left off; partners, worlds and avatars are never on it.

## Partners (new tab)
Everyone you've been with, with fun stats: WooHoos, times pregnant by them, kids together, first/last time. Profile picture if turned on.

## Avatars
- Switch: **same pregnancy on every avatar** / **each avatar has its own**.
- Search box on top. **Groups** (per-avatar mode): one card per group, its avatars share one pregnancy; add avatars with a search picker. Then avatars on their own, then folds for **Older** (not worn in 2 weeks) and **Without the Pregnancy System** (no `Pregnancy/` parameters). Worn or set-up avatars never fold away. Status like "Week 12 · with Rex", "Waiting…", "Not pregnant".
- Avatar page: rename, **group** (or make a new one), which parameters it has (✓/✗), **custom parameter names** (optional, only if this avatar differs), forget.

## Settings
- **VRChat**: control the belly; start on in-game conception.
- **Detecting WooHoos**: **from SPS sockets** (pick sockets, how deep, how long). The avatar's own fertility trigger isn't used.
- **Fertility**: on/off, chance at peak, **multiples** (Realistic / Playful / Lots), start a new cycle today.
- **Avatar parameters** (shared names): each with name, on/off and a one-line explanation.
- **Safety**: hide the belly / hold labor in picked instance types (default Public, Group Public), never changing your own Hide Belly.
- **Look**: a palette, or Custom: an accent and (optionally) a background, picked on a color wheel (ring for the color, square for paler or vivid, brighter or darker). The rest of the theme is worked out from them (`src/lib/theme.ts`), and your own background decides light or dark by itself.
- Partner guess (logs, pictures), Starting up, Backup.

## Avatar parameters
Everything uses the `Pregnancy/` prefix (old names still work as backups). Full list: [osc-parameters.md](osc-parameters.md).

- **App parameters** (the app drives or reads them): `Pregnancy/Stage`, `Pregnancy/BellyExtra`, `Pregnancy/AppActive`, `Pregnancy/AppHeartbeat`, `Pregnancy/FertileLevel`, `Pregnancy/Waiting`, `Pregnancy/BabyCount`.
- **Sync** (follow you onto every avatar, anyone can add): starts with `Pregnancy/HideBelly`, `Pregnancy/FetalSounds`. App parameters can't be added here.
- SPS socket params (read): for detecting WooHoos (names found on the avatar).

## Decisions
- Cycle = 1/10 of the pregnancy length; grace = 1/20; peak chance 60% (setting).
- A pregnancy counts from the moment of the WooHoo.
- Baby genders (Random or Off) are rolled at conception and shown from week 20.
- Waiting card shows who you were with.
- Baby count rolled at conception, revealed about week 12; odds set by the user.
- Kids count when you press "Welcome your baby".

## Build order
1. Avatars + linking (+ per-avatar parameter names).
2. Home states: cycle, waiting, reveal, baby count, due date, birth.
3. Partners tab + stats.
4. Settings: parameters section, fertility, multiples.
5. SPS socket detection (needs testing on a real avatar).
