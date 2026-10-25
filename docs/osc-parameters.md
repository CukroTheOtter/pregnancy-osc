# OSC Parameters

What Pregnancy OSC reads and writes on your avatar, and what your avatar needs to provide.

The app never hardcodes parameter names. Every name lives in the mapping file **`config/osc-parameters.json`**. The app copies it to its settings folder on first run (Settings > OSC > "Open mapping file"). Change a `name` to match your avatar, or set it to `""` to turn that parameter off. No code changes needed.

- OSC address = `/avatar/parameters/<name>`, e.g. `/avatar/parameters/Pregnancy/Stage`.
- Each entry can list `aliases`: other names the same parameter may have (the app picks whichever the avatar has, using OSCQuery).
- The defaults are placeholders that match a standard Pregnancy System install.

---

## 1. Ports and discovery

| What | Value |
|---|---|
| Preferred | **OSCQuery**: the app advertises itself over mDNS, VRChat finds it and sends to it. The app finds VRChat the same way and reads the avatar's parameter list. No ports to set. |
| Fallback send | UDP `127.0.0.1:9000` (VRChat's input) |
| Fallback receive | UDP `9001` (VRChat's output). Only one app can hold this port, which is why OSCQuery is preferred. |
| Also used | `/avatar/change` (VRChat sends the avatar ID on every avatar switch) |

Turn on OSC in VRChat: Action Menu > Options > OSC > **Enabled**.

---

## 2. Parameters

Every parameter uses the **`Pregnancy/`** prefix. Bits: synced bool = 1, synced int/float = 8, local = 0. VRChat's limit is 256.

### 2a. App parameters

What the app drives or reads. Rename them in **Settings > Advanced** (or per avatar under **Avatars**); set a name to empty to turn one off.

| Name | Type | Synced | Bits | What it does |
|---|---|---|---|---|
| `Pregnancy/Stage` | float | yes | 8 | **Belly size**, 0 to 1. Written by the app. **Required.** |
| `Pregnancy/BellyExtra` | float | yes | 8 | Optional. **Extra size**, 0 to 1, for the SDK's Bigger Belly (adds Belly_Bigger on top, scaled by Stage). Twins 0.35, triplets 0.6, 4+ 0.8; past the due date it grows to 1. Each part can be turned off under Settings > Bigger belly. |
| `Pregnancy/AppActive` | bool | **no** | 0 | True while the app runs. False on exit. **Required.** |
| `Pregnancy/AppHeartbeat` | bool | **no** | 0 | Flips every second while the app runs. **Required.** |
| `Pregnancy/Control/Start` | bool | **no** | 0 | Optional, name fixed (Start, Pause and Stop are made for this app, so they can't be renamed or given per-avatar names). Read. Button in the avatar's Settings > App Controls menu: starts a pregnancy with the saved length. |
| `Pregnancy/Control/Pause` | bool | **no** | 0 | Read. Button: pauses, or resumes if paused. |
| `Pregnancy/Control/Stop` | bool | **no** | 0 | Read. Button: held for 2 seconds, ends the pregnancy (not a birth). |
| `Pregnancy/FertileLevel` | int | yes | 8 | Optional. 0 low, 1 medium, 2 high, 3 peak. 0 while pregnant or waiting. |
| `Pregnancy/Waiting` | bool | yes | 1 | Optional. True during the grace period after a WooHoo. |
| `Pregnancy/BabyCount` | int | yes | 8 | Optional. 0 until the babies can be seen (week 12), then 1 to 4. |
| `Pregnancy/FetalMovement` | int | yes | 8 | Optional. **Kicks:** the app sets it to the little / strong / big kick code (default 1 / 2 / 3, editable under Settings > Advanced > Kick values), then back to 0 after 1.5 s. The SDK's kick layer plays a different kick for each code, and only rolls its own kicks while `Pregnancy/AppActive` is off. Kicks start at week 16; strong from 24, big from 32. |

Older names still work as backups: `Pregnancy/Active`, `Pregnancy/Heartbeat` and the `PregnancyOSC/...` names. Parameter files from earlier versions are upgraded to the new names automatically (names you changed yourself are kept).

**App is running + Heartbeat (optional, only if your avatar has its own counter):**
1. Add `Pregnancy/AppActive` and `Pregnancy/AppHeartbeat` to the expression parameters: bool, **not synced**, **not saved**, default false.
2. In the counter's animator layer, only let it run when `AppActive` is false.
3. Add a small local-only watchdog layer: "Beat A" (`AppHeartbeat` true) and "Beat B" (false), each switching to the other on change. From each, a transition with exit time 5 s goes to "Timeout", which has a local parameter driver setting `AppActive = false`.

If your avatar makes its own random kicks, have its kick layer pause while `AppActive` is on too, so only the app's kicks play.

**Without these, everything still works.** The app writes the belly about once a second, which overwrites anything the avatar's counter adds in between.

### 2b. Sync

Parameters that **follow you onto every avatar**. The app remembers the last value it sees in-game and sends it again whenever you switch avatars (after giving the new avatar about 3 seconds to load, again when it has loaded, and again at 10 seconds). Values the new avatar reports (its own saved values) are never remembered: VRChat reports them as it loads, a moment BEFORE it says the avatar changed. From the log's "Switching" line (written when the switch starts) until 1.5 s after the new avatar's parameters show up over OSCQuery, reported values are ignored, however long loading takes (30 s cap if they never show up). Other changes are held for "Wait before remembering a change" (Settings > Advanced > Sync, default 2 s) and dropped if the avatar changes in that time (the backup when log reading is off). A saved Hide Belly that's on is sent every 0.5 s from the "Switching" line (or the avatar change) until 1.5 s after the new avatar loads, so its belly isn't seen while it loads. Values can also be set in the app (Settings > Follow you): saved and sent right away, replacing any in-game change still waiting. Add, rename or remove them in **Settings > Advanced > Follow you (Sync)**.

| Built-in entries | Type |
|---|---|
| `Pregnancy/HideBelly` (role `hideBelly`) | bool |
| `Pregnancy/FetalSounds` (role `fetalSounds`) | bool |

The built-in entries are always there and always bool: rename them if an avatar uses other names, but they can't be removed (one missing from the file comes back on load). The app finds them by their `role`, not their name, so a renamed Hide Belly keeps its special handling. Older files without roles: the entry with the default name, or a bool one ending in `HideBelly` / `FetalSounds`, gets the role.

Anything else can go here (any name, as bool, int or float) **except app parameters**: their names, backup names and per-avatar names are blocked, because the app already controls them. Sync entries don't count toward the app's bits; they're parameters your avatar already has.

### 2c. SPS sockets (read only, for detecting WooHoos)

VRCFury SPS sockets with haptics on expose `OGB/Orf/<socket>/<channel>` floats (the same ones OSC Goes Brrr reads). The app finds them on the avatar you're wearing and asks VRChat to send them.

- VRCFury keeps these at 0 until an OSC app sets `OGB_ENABLED` to true. While detection is on and the avatar has sockets, the app sends `OGB_ENABLED` true whenever it reads it as off (it resets on every avatar load). It never sets it false.

- Watched channel per socket (best available): `PenOthersNewRoot`, then `PenOthers`, then `PenOthersNewTip`. "Others" = another player.
- "Self" channels (your own touches) and plugs (`OGB/Pen/...`) are ignored.
- **Hole types (global):** each socket gets a type from its name: Vaginal (vagina, pussy, womb…), Anal (anal, ass, butt…), Oral (mouth, throat…) or Other. Settings chooses which types count on every avatar (default: Vaginal only).
- **Per avatar** (its page under Avatars): fix a socket's type if its name is unusual, or pick exactly which sockets count on that avatar (overrides the global types).
- A WooHoo counts when a socket that counts is past the depth you set for the total time you set (default 30% for 20 s). One per session; a session ends after a quiet gap (default 2 min), or when a WooHoo's result is revealed. A socket value that hasn't changed for a whole gap doesn't count (VRChat only sends changes, so a lost 0 can't keep a session open).
- Settings > Detecting WooHoos shows each socket with a live meter.

These are local parameters on the avatar, so they cost no synced bits.

### 2c. Budget

| Set | Synced bits |
|---|---|
| Belly only (minimum) | 8 |
| Belly + Extra size | 16 |
| App is running + Heartbeat | 0 (local) |
| Fertility, babies and kicks (optional) | +25 |
| **All app parameters** | **41** |

---

## 3. How the app drives the belly

| Situation | What the app does |
|---|---|
| Running, pregnant | Writes `stage` from the timeline about every `sendIntervalMs` (default 1 s) when it changed by at least `minStageChange`, and re-sends it every second anyway so in-game changes are put back. Plus Active and the heartbeat. |
| Running, pregnancy paused | Does not write `stage` or `bellyExtra`, so you can set them in-game. Both are sent again on resume. |
| Running, not pregnant | Does not write `stage` (so the in-game menu still works). Still holds Active and the heartbeat. |
| App starts or VRChat connects | **Snaps** `stage` to the app's timeline, ignoring whatever the avatar did while the app was closed. A visible jump is expected. |
| Avatar change | Loads that avatar's pregnancy (or its linked group's), snaps the belly, then re-sends the Sync parameters once the avatar has loaded. |
| A WooHoo (SPS sockets) | With Fertility on, the app starts a **grace period** (holds the belly at 0, sets `waiting`) and reveals the outcome later; with Fertility off, a pregnancy starts right away. The avatar's own fertility trigger is not used. |
| Chatbox (Settings > Chatbox, off by default) | Shows a message per event (started, paused, resumed, stopped, born, WooHoo, not pregnant, achievement) with `/chatbox/input`, at most one every 2 s, then clears it after "Show each message for" (default 8 s). Optional status line ("Always show pregnancy status", with or without the partner) is re-sent every 4 s while pregnant, and comes back after each event message. |
| Exit (or VRChat closes, or tracking paused) | Sends the final belly value, then Active false. The avatar's clock carries on from there. |
| Past the due date | `stage` stays at 1. `bellyExtra` grows to 1 over `overdue.maxOverdueFraction` of the term (default 5%, about 2 weeks at 9 months). |

---

## 4. Decisions so far

| Question | Decision |
|---|---|
| Avatar clock ran while the app was closed | Snap back to the app's timeline. |
| SDK changes | None. The app is self-contained; avatar-side pieces in 2b are optional. |
| Conception in-game | Start a pregnancy automatically. |

## 5. Open questions

1. **Speed on exit:** should the app set `progressionSpeed` on exit so the avatar keeps the app's pace? (It changes a saved menu setting, so it's off for now.)
2. **Overdue:** the app only reads `overdue`. Should it ever turn it on itself past the due date?
