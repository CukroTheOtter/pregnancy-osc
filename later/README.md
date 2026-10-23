# Parked features

These screens were built but taken out of the app to keep it simple. They
still work with the current data model, so each can be brought back on its own:
move the files back under `src/` (same folder names) and add a nav entry.

- `views/Home.svelte` + `components/XrayBelly.svelte`, `Baby.svelte`: x-ray belly with babies
- `views/GeneLab.svelte`, `views/Nursery.svelte` + `lib/genetics.ts`, `lib/species.ts`, `lib/look.ts`: species, traits, hybrids
- `views/Scans.svelte` + `lib/reveals.ts`, `lib/sound.ts`, `lib/share.ts`: ultrasound reveals and keepsakes
- `views/Journal.svelte`: notes, milestones, share card
- `views/Avatars.svelte`: per-avatar profiles (the engine still keeps profiles)
- `views/Pregnancies.svelte`: several pregnancies at once
- `components/Onboarding.svelte`, `NewPregnancy.svelte`, `PartnerCard.svelte`, `views/People.svelte`, `views/Settings.svelte`: the old versions

`lib/genetics.test.ts` has the genetics tests (run them again once genetics is back in `src/lib`).
