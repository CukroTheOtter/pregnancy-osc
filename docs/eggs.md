# Eggs

For egg-laying sonas: an avatar can lay eggs instead of giving birth. The
eggs then keep warm for a while and hatch.

## Turn it on

**Avatars**, open the avatar, turn on **🥚 Eggs**. It's per avatar, so your
other avatars still give birth as usual.

- A pregnancy on that avatar becomes an egg pregnancy. If one is already
  going (labor too), it changes with the switch, as long as the eggs aren't
  laid yet.
- Turning it off doesn't change eggs that are already laid: they still hatch.
- Avatars in a group share one pregnancy: it starts as eggs if the avatar it
  starts on has Eggs on, and the switch on any of them changes it.

## The pregnancy

Everything works like a normal pregnancy (length, the partner, scans).
What's different:

- Kicks work as usual if the avatar has `FetalMovement` (something wiggling
  in the egg). An avatar without it never kicks, and Home has no Movement
  card for it (same for any avatar, eggs or not).
- Home says **Eggs** instead of Babies: "One egg", "3 eggs!".
- The journal's scans show eggs, each with a little one curled up inside,
  and the screen counts "2 EGGS". The week 8 scan has a little egg forming,
  and week 36 says the shells are getting hard.
- Girl or boy (with genders on) is a surprise until the eggs hatch: Home says
  "when they hatch", scans never show it, the calendar has no week 20 "girl or
  boy" day, and One Of Each only counts once they've hatched.
- Rare scans: only "Camera shy" and "A heart on the screen". Nobody can wave,
  yawn, hiccup or high-five inside a shell (`EGG_SCANS` in `src/lib/journal.ts`).

## Laying

Labor works as usual (Settings > Labor and birth), but the water never breaks:

| | Normal | Eggs |
|---|---|---|
| Phases | early 50%, water broke 30%, pushing 20% | early 60%, laying 40% |
| `Pregnancy/Labor` | 1, 2, 3, then 4 when born | 1, 3, then 4 when laid (**never 2**) |
| Water breaks first | about 15% of the time | never |
| Home button | Welcome your baby 🍼 | Lay your eggs 🥚 |

So an avatar needs nothing new: `Pregnancy/Labor` = 3 means laying, and 4
means the eggs are out (the belly goes down as usual). If your avatar has a
"water broke" effect on 2, it simply never plays.

Contractions, *Resting holds off labor* ("laying won't start until you're
up"), Safety and *Labor waits for the other parent* all work the same.

## Hatching

Laid eggs hatch after **Settings > Labor and birth > Eggs hatch after**:
1 hour, 1 day, **3 days** (default) or 1 week, in real time.

- Home shows a card for each clutch, with how long is left and **Hatch now**.
- They hatch even if the app was closed: next time it runs, they hatch at the
  time they were due (the journal and calendar use that time).
- When they hatch you get a notification, and a chatbox message if that's on.
- Hatching happens in the app only. There's no avatar parameter for it.

## Journal, calendar and stats

- Journal: "Laying started", "2 eggs laid!", then "🐣 Hatched!" (with girls
  and boys, if genders are on).
- Calendar: 🥚 Laid on the day they're laid, and 🐣 Hatch on the day they hatch
  (before then it's when they're due to). Both are under the "Born & hatched"
  filter.
- Stats: laid eggs count as babies born.

## Chatbox

Two new event messages (Settings > Chatbox): **Eggs laid** ("🥚 The eggs are
laid!") and **Eggs hatched** ("🐣 The eggs hatched!"). For an egg pregnancy
they're sent instead of **Baby born**.

## Achievements

Laying counts as a birth, so Welcome Home, Full House, Right On Time, Early
Bird, Speedrun and the rest unlock as usual. Three are just for eggs (all
surprises until unlocked):

| | Achievement | How |
|---|---|---|
| 🥚 | Egg-cellent! | Lay your first eggs |
| 🐣 | Hatch Day | Your eggs hatch |
| 🪺 | Best Of Both | A live birth and a clutch of eggs |

*Splash!* (the water breaking first) can't happen with eggs.

## For developers

- `BodyKind::Egg` on the avatar profile (`kind`) is the switch. Each pregnancy
  copies it when it starts (`Pregnancy::eggs`), so history stays right if the
  switch changes later. `actions::set_avatar_eggs` changes the profile and its
  pregnancy (not laid yet), labor too (`Labor::eggs`).
- `labor.rs`: `EGG_SPLIT` (no active phase), `water_first` is never true for
  eggs, and `Labor::for_pregnancy` makes the labor.
- `actions::give_birth` sets `hatch_ms`. `actions::take_hatched` (engine,
  every second) sets `hatched_ms`, and `actions::hatch_now` moves the time up.
  Eggs laid before hatch times existed hatch at the time they were laid.
- Settings: `labor.hatch_hours` (default 72).
- UI: `isEggs`, `kidWord`, `birthTitle` in `src/lib/fertility.ts`,
  `components/EggsCard.svelte`, and `ScanMeta.eggs` in `Ultrasound.svelte`.
  The mock (`src/lib/mock.ts`) has `hatch_eggs` and `set_avatar_eggs`.
