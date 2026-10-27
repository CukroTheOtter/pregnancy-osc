# Pregnancy OSC (app/)

Desktop app that drives the avatar's pregnancy over OSC. Tauri 2 + Rust + Svelte 5.

- Work inside `app/`; edit the SDK package only when the user asks (see the root `CLAUDE.md`).
- Keep it simple and cute. Build what the user asks for; propose (don't build) extras.
- Layout: `crates/pregsys-core` (pure logic + tests), `crates/pregsys-osc` (OSC, OSCQuery, bridge), `crates/belly-cli`, `src-tauri` (engine, commands), `src` (UI), `later/` (parked).
- Docs: `README.md`, `docs/osc-parameters.md` (source of truth for parameters), `docs/plan.md`.
- Parameters all use the `Pregnancy/` prefix (mapping in `config/osc-parameters.json`, versioned + upgraded in `mapping.rs`). App parameters vs the Sync list; app parameters can't be synced.
- Every Rust command has a twin in `src/lib/mock.ts` (browser preview). Change both.
- Wording: short and plain; "WooHoo" for in-game fun; in-app dialogs (`confirmAsk`), never `confirm()`.
- UI: tabs Home / Me (stats + achievements) / Partners / Avatars / Settings. Settings cards use `components/Section.svelte` (collapsible). Achievements: logic in `pregsys-core/src/achievements.rs`, display list in `src/lib/achievements.ts` (keep ids/titles in step).
- Debug mode: tap the title 5 times.

## VRChat facts learned the hard way
- VRChat looks for OSCQuery apps only at its start-up or when OSC is toggled in its Action Menu. So the app also listens on 9001 (when free) and polls OSCQuery for the worn avatar (1 s until found, 5 s after, 2 s after a log "Switching" line).
- The bridge (OSCQuery service + the 9001 listener, always 9001, never the fixed-mode listen port) starts with the app and stays up while VRChat is closed, so a VRChat launched at the same time (VRCX auto-launch) still finds it.
- mdns-sd only announces an address on the adapter whose subnet holds it, so 127.0.0.1 never left loopback and VRChat (listening on the real adapters) never found the app. `announce.rs` is our own small responder: 127.0.0.1 on every adapter, laid out like VRChat's own answers (PTR as the answer; TXT, SRV, A as additional records). Confirmed: a fresh VRChat logs "Found new OSC Service: PregnancyOSC-<pid>" and sends to the app's own port. Its "no SRV Records found" debug lines are noise (from both PCs, even for well-formed packets).
- Other PCs' VRChat shows up over mDNS too (its OSCQuery also says 127.0.0.1). Only trust the one whose `?HOST_INFO` on 127.0.0.1 answers with its own name.
- OSC off in VRChat's Action Menu: OSCQuery and port 9000 stay up and nothing is logged, but `/avatar` is empty (no avatar, no values). That's what "not linked" means. Toggling it on reloads the avatar in the log; "Found new OSC Service: …" only appears for apps VRChat hasn't seen yet.
- Announce services at 127.0.0.1 with one host per type (`<name>.oscjson.local.`, `<name>.osc.local.`); HTTP on loopback. Never restart the bridge to change listen addresses (VRChat won't rediscover it); update the tree in place.
- Fixed-port mode (user runs an OSC router, e.g. send 9005 / listen 9006) still uses OSCQuery to READ the avatar, but always sends to the fixed port.
- Switching avatars, VRChat reports the NEW avatar's saved parameter values just BEFORE `/avatar/change` (seen live). Sync values are guarded by the log "Switching" line (its own timestamp) and a 2 s hold dropped on avatar change (`driver.rs`).
- After an avatar change VRChat isn't ready yet: the driver sends nothing for 3 s, then everything, again when the params show up, and again at 10 s. When the avatar finishes loading, VRChat reports its DEFAULTS for every parameter: Sync values aren't remembered while it loads: until 1.5 s after its params show up (OSCQuery polled every 1 s while loading; the old avatar's identical list in the first 5 s doesn't count), capped at 30 s.
- Avatar names: VRChat's `LocalLow\VRChat\VRChat\OSC\usr_*\Avatars\<avtr_id>.json` has the exact name; the log only has names, logged when loading STARTS. `auto_name` marks names the app picked (user-typed names are never changed).
- The log writes "Entering Room" BEFORE "Joining wrld_…" (see `instance.rs`).
- "Belly not changing" is often Hide Belly on (Home shows a 🙈 note). `belly-cli --query` shows live values.

## Commands (bash needs `~/.cargo/bin` and `/c/Program Files/nodejs` on PATH)
- `cargo test --workspace` · `npm test` · `npx svelte-check --tsconfig ./tsconfig.json`
- Release: `powershell -ExecutionPolicy Bypass -File scripts\release.ps1 -Version X.Y.Z -Notes "..."` (bumps, tests, commit, pushes an annotated tag; `.github/workflows/release.yml` builds, signs and publishes with the repo secrets). Run it with `run_in_terminal`: only the user's real shell has `gh`/git signed in to the repo's account (the Bash/PowerShell tools can't push). Never handle the signing key or its password. Never name the user's other accounts in the repo. Updater: `src-tauri/src/updates.rs`, Settings > Updates.
- `npx tauri build --config src-tauri/tauri.local.json` (local build, no update signing) → `target/release/bundle/nsis/Pregnancy OSC_*_x64-setup.exe` (rebuild after changes; the user reinstalls)
- Preview: `preview_start` with "app-preview" (root `.claude/launch.json`). Clear `pregsys-mock-data` / `pregsys-open-sections` from localStorage and stop it afterwards.
- `target/debug/belly-cli.exe --query` (add `--no-oscquery` for fixed ports) reads the worn avatar; `--listen 10` prints what VRChat sends. Both read-only.

Don't launch the installed app or touch its data folder unless asked.
