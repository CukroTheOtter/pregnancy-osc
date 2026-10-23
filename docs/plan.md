# Pregnancy OSC: the plan

Agreed layout and features. Built in the order at the bottom.

## Tabs
**Home · Partners · Avatars · Settings**. The header shows the avatar you're wearing and the link status. Tapping the avatar name lets you view another avatar.

## Home (one avatar, or a linked group)
| State | Shows |
|---|---|
| Not pregnant | **Cycle card**: ring "Day 12 of 28", chance now (Low / Medium / High / Peak), "next fertile window in …". Length card. Small "start manually". |
| Waiting | "💕 You WooHoo'd with **Ember** at Cozy Cabin." Countdown "You'll find out in 1 day 4 h". No hint of the outcome. Belly held flat. |
| Reveal (once) | "You're pregnant! 💕" + **estimated due date**, or "Not this time 🤍". OK button. |
| Pregnant | Progress ring, **babies: "Too early to tell" → "It's twins! 👶👶"** (from about week 12), due date, Length, Partner (with "3rd time together · 2 kids"). At the due date: **"Welcome your baby 🍼"**. Small "End early". Shared avatars listed. |

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
- Partner guess (logs, pictures), Starting up, Look, Backup.

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
