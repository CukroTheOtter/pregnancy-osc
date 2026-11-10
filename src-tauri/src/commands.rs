//! Commands the web UI calls. Mirrored by the browser mock in `src/lib/api.ts`.

use crate::state::{EngineCmd, Shared, Status};
use pregsys_core::actions::{self, NewPregnancy};
use pregsys_core::instance::Person;
use pregsys_core::mapping::Mapping;
use pregsys_core::model::*;
use pregsys_core::now_ms;
use pregsys_core::storage;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

type S<'a> = State<'a, Arc<Shared>>;
type R<T> = Result<T, String>;

fn changed(app: &AppHandle, shared: &Shared, resync: bool) {
    shared.mark_dirty();
    if resync {
        shared.send(EngineCmd::Resync);
    }
    let _ = app.emit("data-changed", ());
}

#[tauri::command]
pub fn get_data(s: S) -> AppData {
    s.data.lock().unwrap().clone()
}

#[tauri::command]
pub fn get_status(s: S) -> Status {
    s.status.lock().unwrap().clone()
}

/// Sets a Sync value from the app (like changing it in-game): saved and sent now.
#[tauri::command]
pub fn set_sync_value(app: AppHandle, s: S, name: String, value: f64) {
    s.data.lock().unwrap().sync_values.insert(name.clone(), value);
    s.mark_dirty();
    s.send(EngineCmd::SyncSet(name));
    let _ = app.emit("data-changed", ());
}

/// Shows the belly in this instance only (Settings > Safety hides it here).
#[tauri::command]
pub fn show_belly_here(s: S) {
    s.send(EngineCmd::ShowHere);
}

#[tauri::command]
pub fn take_load_warning(s: S) -> Option<String> {
    s.load_warning.lock().unwrap().take()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, s: S, settings: Settings) -> R<Option<String>> {
    let old = {
        let mut d = s.data.lock().unwrap();
        let old = std::mem::replace(&mut d.settings, settings.clone());
        // Turning debug mode off removes Test Buddy and everything they touched.
        if old.debug && !settings.debug {
            pregsys_core::testbuddy::remove_all(&mut d, now_ms());
        }
        // A wait to find out follows a new pregnancy length (or wait setting).
        actions::retime_waits(&mut d, &old);
        old
    };
    changed(&app, &s, true);
    if old.osc != settings.osc {
        s.send(EngineCmd::Restart);
    }
    let mut note = None;
    // --no-save: start-up settings live outside the data folder, so leave them be.
    let saving = !s.store.is_read_only();
    if saving && old.start_with_os != settings.start_with_os {
        crate::set_os_autostart(&app, settings.start_with_os)?;
    }
    if saving && old.start_with_steamvr != settings.start_with_steamvr {
        let running = s.status.lock().unwrap().steamvr_running;
        note = Some(crate::system::set_steamvr_autostart(&s.data_dir, settings.start_with_steamvr, running)?);
    }
    Ok(note)
}

#[tauri::command]
pub fn create_pregnancy(app: AppHandle, s: S, opts: NewPregnancy) -> Pregnancy {
    let p = {
        let mut d = s.data.lock().unwrap();
        let id = actions::start_pregnancy(&mut d, opts, now_ms());
        d.pregnancy(&id).cloned().unwrap()
    };
    changed(&app, &s, true);
    p
}

/// Saves a pregnancy edited in the UI (timeline, babies, traits, reveals...).
#[tauri::command]
pub fn save_pregnancy(app: AppHandle, s: S, pregnancy: Pregnancy) -> R<()> {
    {
        let mut d = s.data.lock().unwrap();
        let mut p = pregnancy;
        p.sync_babies();
        actions::ensure_genders(&mut p);
        match d.pregnancy_mut(&p.id) {
            Some(slot) => *slot = p,
            None => return Err("pregnancy not found".into()),
        }
    }
    changed(&app, &s, true);
    Ok(())
}

#[tauri::command]
pub fn end_pregnancy(app: AppHandle, s: S, id: String) {
    actions::end_pregnancy(&mut s.data.lock().unwrap(), &id, now_ms());
    changed(&app, &s, true);
}

#[tauri::command]
pub fn give_birth(app: AppHandle, s: S, id: String) {
    actions::give_birth(&mut s.data.lock().unwrap(), &id, now_ms());
    changed(&app, &s, true);
}

#[tauri::command]
pub fn start_labor(app: AppHandle, s: S, id: String) {
    pregsys_core::labor::start(&mut s.data.lock().unwrap(), &id, now_ms());
    changed(&app, &s, true);
}

#[tauri::command]
pub fn delete_pregnancy(app: AppHandle, s: S, id: String) {
    actions::delete_pregnancy(&mut s.data.lock().unwrap(), &id, now_ms());
    changed(&app, &s, true);
}

#[tauri::command]
pub fn set_current_pregnancy(app: AppHandle, s: S, avatar_id: Option<String>, id: Option<String>) {
    s.data.lock().unwrap().set_current(avatar_id.as_deref(), id);
    changed(&app, &s, true);
}

#[tauri::command]
pub fn set_partner(app: AppHandle, s: S, pregnancy_id: String, person: Option<Person>, name: Option<String>, random: Option<bool>) {
    let mut d = s.data.lock().unwrap();
    if random == Some(true) {
        actions::random_partner(&mut d, &pregnancy_id, pregsys_core::fertility::roll());
    } else {
        actions::set_partner(&mut d, &pregnancy_id, person, name);
    }
    drop(d);
    changed(&app, &s, true);
}

/// Who it was, while waiting to find out (the WooHoo by its time).
#[tauri::command]
pub fn set_waiting_partner(app: AppHandle, s: S, time_ms: i64, person: Option<Person>, name: Option<String>, random: Option<bool>) {
    let mut d = s.data.lock().unwrap();
    if random == Some(true) {
        actions::random_waiting_partner(&mut d, time_ms, pregsys_core::fertility::roll());
    } else {
        actions::set_waiting_partner(&mut d, time_ms, person, name);
    }
    drop(d);
    changed(&app, &s, false);
}

#[tauri::command]
pub fn save_avatar(app: AppHandle, s: S, profile: AvatarProfile) {
    s.data.lock().unwrap().avatars.insert(profile.id.clone(), profile);
    changed(&app, &s, true);
}

#[tauri::command]
pub fn delete_avatar(app: AppHandle, s: S, id: String) {
    actions::delete_avatar(&mut s.data.lock().unwrap(), &id);
    changed(&app, &s, true);
}

#[tauri::command]
pub fn delete_avatars(app: AppHandle, s: S, ids: Vec<String>) {
    actions::delete_avatars(&mut s.data.lock().unwrap(), &ids);
    changed(&app, &s, true);
}

#[tauri::command]
pub fn create_group(app: AppHandle, s: S, name: String, avatar_ids: Vec<String>) -> R<String> {
    let r = actions::create_group(&mut s.data.lock().unwrap(), &name, &avatar_ids);
    changed(&app, &s, true);
    r
}

#[tauri::command]
pub fn set_avatar_group(app: AppHandle, s: S, avatar_id: String, group_id: Option<String>) -> R<()> {
    actions::set_avatar_group(&mut s.data.lock().unwrap(), &avatar_id, group_id.as_deref())?;
    changed(&app, &s, true);
    Ok(())
}

#[tauri::command]
pub fn rename_group(app: AppHandle, s: S, id: String, name: String) {
    actions::rename_group(&mut s.data.lock().unwrap(), &id, &name);
    changed(&app, &s, false);
}

#[tauri::command]
pub fn delete_group(app: AppHandle, s: S, id: String) {
    actions::delete_group(&mut s.data.lock().unwrap(), &id);
    changed(&app, &s, true);
}

/// Remembers "this avatar name / person is a fox".
#[tauri::command]
pub fn set_species_answer(app: AppHandle, s: S, key: String, species: String) {
    // User IDs stay as-is; avatar names are matched case-insensitively.
    let key = key.trim();
    let key = if key.starts_with("usr_") { key.to_string() } else { key.to_lowercase() };
    s.data.lock().unwrap().species_answers.insert(key, species);
    changed(&app, &s, false);
}

#[tauri::command]
pub fn forget_person(app: AppHandle, s: S, name: String, user_id: Option<String>) {
    actions::forget_person(&mut s.data.lock().unwrap(), &name, user_id.as_deref());
    changed(&app, &s, false);
}

fn check_env(d: &AppData) -> pregsys_core::check::Env {
    pregsys_core::check::Env { log_dir_missing: d.settings.log_dir.as_deref().is_some_and(|p| !std::path::Path::new(p).is_dir()) }
}

/// Settings > Troubleshooting: problems in the save and settings.
#[tauri::command]
pub fn check_problems(s: S) -> Vec<pregsys_core::check::Problem> {
    let d = s.data.lock().unwrap();
    pregsys_core::check::problems(&d, check_env(&d))
}

/// Applies one of a problem's fixes. False if it was already gone.
#[tauri::command]
pub fn fix_problem(app: AppHandle, s: S, id: String, fix: usize) -> bool {
    let done = {
        let mut d = s.data.lock().unwrap();
        let env = check_env(&d);
        pregsys_core::check::fix(&mut d, env, &id, fix)
    };
    if done {
        changed(&app, &s, true);
    }
    done
}

/// Adds someone to your 💞 partners, or takes them off.
#[tauri::command]
pub fn set_my_partner(app: AppHandle, s: S, name: String, user_id: Option<String>, on: bool) {
    actions::set_my_partner(&mut s.data.lock().unwrap(), &name, user_id.as_deref(), on, now_ms());
    changed(&app, &s, false);
}

/// Forgets your standing height (SteamVR learns it again while you stand).
#[tauri::command]
pub fn forget_standing_height(app: AppHandle, s: S) {
    s.data.lock().unwrap().vr_standing_m = None;
    changed(&app, &s, false);
}

/// "Set height now": measures your headset's height above the floor through
/// SteamVR and saves it as your normal height.
#[tauri::command]
pub async fn set_standing_height_now(app: AppHandle, s: S<'_>) -> R<f32> {
    let tracking = s.vr_posture.lock().unwrap().is_some_and(|v| now_ms() - v.at_ms < 3_000);
    if !tracking {
        return Err("Needs SteamVR running with your headset tracked (and \"Resting holds off labor\" and \"Use SteamVR tracking\" on).".into());
    }
    let (tx, rx) = std::sync::mpsc::channel();
    *s.vr_set_height.lock().unwrap() = Some(tx);
    let m = tauri::async_runtime::spawn_blocking(move || rx.recv_timeout(std::time::Duration::from_secs(2)))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|_| "SteamVR didn't answer. Try again.".to_string())??;
    changed(&app, &s, false);
    Ok(m)
}

/// "Send a test" (Settings > Notifications): goes where a real notification
/// would (headset, else desktop, see `crate::notify`). Returns where, by name.
#[tauri::command]
pub async fn test_notice(app: AppHandle, s: S<'_>) -> R<Vec<String>> {
    let targets = crate::vr_notify::targets(&s);
    if targets.is_empty() {
        if !s.data.lock().unwrap().settings.notifications {
            return Err("Nothing's on to show it. Turn on Desktop notifications, or one of the VR apps (with it running).".into());
        }
        crate::desktop_notify(&app, "Pregnancy OSC 💕", "This is how notifications look.");
        return Ok(vec!["your desktop".into()]);
    }
    let (title, body) = ("Pregnancy OSC 💕", "This is how notifications look in VR.");
    for &t in &targets {
        if t == VrTarget::Steamvr {
            crate::vr_notify::queue_steamvr(&s, title, body);
        } else {
            tauri::async_runtime::spawn_blocking(move || crate::vr_notify::send_to(t, title, body))
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| format!("{}: {e}", match t {
                    VrTarget::Xsoverlay => "XSOverlay",
                    _ => "OVR Toolkit",
                }))?;
        }
    }
    Ok(targets
        .iter()
        .map(|t| match t {
            VrTarget::Xsoverlay => "XSOverlay",
            VrTarget::OvrToolkit => "OVR Toolkit",
            VrTarget::Steamvr => "SteamVR",
        })
        .map(String::from)
        .collect())
}

#[tauri::command]
pub fn import_community_table(app: AppHandle, s: S, json: String) -> R<usize> {
    let table: std::collections::BTreeMap<String, String> =
        serde_json::from_str(&json).map_err(|e| format!("Expected a JSON object of \"avatar name\": \"species\" pairs ({e})"))?;
    let n = table.len();
    s.data.lock().unwrap().community_species = table.into_iter().map(|(k, v)| (k.trim().to_lowercase(), v)).collect();
    changed(&app, &s, false);
    Ok(n)
}

#[tauri::command]
pub fn save_journal_entry(app: AppHandle, s: S, mut entry: JournalEntry) -> JournalEntry {
    {
        let mut d = s.data.lock().unwrap();
        if entry.id.is_empty() {
            entry.id = new_id("j");
        }
        if entry.time_ms == 0 {
            entry.time_ms = now_ms();
        }
        match d.journal.iter_mut().find(|j| j.id == entry.id) {
            Some(slot) => *slot = entry.clone(),
            None => d.journal.push(entry.clone()),
        }
    }
    changed(&app, &s, false);
    entry
}

#[tauri::command]
pub fn delete_journal_entry(app: AppHandle, s: S, id: String) {
    s.data.lock().unwrap().journal.retain(|j| j.id != id);
    changed(&app, &s, false);
}

#[tauri::command]
pub fn add_keepsake(app: AppHandle, s: S, mut keepsake: Keepsake) -> Keepsake {
    if keepsake.id.is_empty() {
        keepsake.id = new_id("k");
    }
    s.data.lock().unwrap().keepsakes.push(keepsake.clone());
    changed(&app, &s, false);
    keepsake
}

#[tauri::command]
pub fn pulse(s: S, key: String, ms: Option<i64>) {
    s.send(EngineCmd::Pulse(key, ms.unwrap_or(1000)));
}

#[tauri::command]
pub fn export_backup(s: S) -> R<String> {
    storage::to_json(&s.data.lock().unwrap()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reset_data(app: AppHandle, s: S, kind: actions::ResetKind) -> R<()> {
    // Keep a copy of what we had, just in case.
    s.save_if_dirty()?;
    let _ = s.store.daily_backup(&format!("before-reset-{}", now_ms()));
    actions::reset_data(&mut s.data.lock().unwrap(), kind);
    if kind == actions::ResetKind::Everything {
        std::fs::write(&s.mapping_path, pregsys_core::mapping::DEFAULT_MAPPING_JSON).map_err(|e| e.to_string())?;
        s.send(EngineCmd::Restart);
    }
    changed(&app, &s, true);
    Ok(())
}

#[tauri::command]
pub fn import_backup(app: AppHandle, s: S, json: String) -> R<()> {
    let data = storage::parse(&json).map_err(|e| e.to_string())?;
    // Keep a copy of what we had, just in case.
    let _ = s.store.daily_backup(&format!("before-import-{}", now_ms()));
    *s.data.lock().unwrap() = data;
    changed(&app, &s, true);
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingInfo {
    mapping: Mapping,
    path: String,
    synced_bits: u32,
    synced_bits_all: u32,
}

#[tauri::command]
pub fn get_mapping(s: S) -> MappingInfo {
    let m = s.mapping.lock().unwrap().clone();
    MappingInfo {
        synced_bits: m.synced_bits(false),
        synced_bits_all: m.synced_bits(true),
        mapping: m,
        path: s.mapping_path.to_string_lossy().to_string(),
    }
}

#[tauri::command]
pub fn save_mapping(s: S, mut mapping: Mapping) -> R<()> {
    let bad = mapping.protected_sync();
    if !bad.is_empty() {
        return Err(format!("{} is an app parameter, so it can't be in Sync.", bad.join(", ")));
    }
    // Hide Belly and Fetal Sounds can be renamed, never removed; menu buttons keep their names.
    mapping.apply_locks();
    let json = serde_json::to_string_pretty(&mapping).map_err(|e| e.to_string())?;
    std::fs::write(&s.mapping_path, json).map_err(|e| e.to_string())?;
    s.send(EngineCmd::Restart);
    Ok(())
}

#[tauri::command]
pub fn reset_mapping(s: S) -> R<()> {
    std::fs::write(&s.mapping_path, pregsys_core::mapping::DEFAULT_MAPPING_JSON).map_err(|e| e.to_string())?;
    s.send(EngineCmd::Restart);
    Ok(())
}

#[tauri::command]
pub fn reload_mapping(s: S) {
    s.send(EngineCmd::Restart);
}

#[tauri::command]
pub fn open_path(app: AppHandle, path: String) -> R<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_path(path, None::<&str>).map_err(|e| e.to_string())
}

/// The avatar system that works with the app (Home's "not set up" note).
const SYSTEM_PAGE: &str = "https://jinxxy.com/CukroTheOtt/VRChatPregnancySystem";
const TROUBLESHOOTING_PAGE: &str = "https://github.com/CukroTheOtter/pregnancy-osc/blob/main/docs/troubleshooting.md";

#[tauri::command]
pub fn open_system_page(app: AppHandle) -> R<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_url(SYSTEM_PAGE, None::<&str>).map_err(|e| e.to_string())
}

/// Settings > Troubleshooting > Copy details: a short report to paste when asking for help.
#[tauri::command]
pub fn copy_details(app: AppHandle, s: S) -> String {
    use pregsys_core::{logwatch, mapping::SyncRole, report};
    let st = s.status.lock().unwrap().clone();
    let d = s.data.lock().unwrap().clone();
    let mapping = s.mapping.lock().unwrap().clone();
    let profile = st.avatar_id.as_deref().and_then(|id| d.avatars.get(id));
    let dir = d.settings.log_dir.clone().map(std::path::PathBuf::from).or_else(logwatch::find_log_dir);
    let file = st.avatar_id.as_deref().zip(dir).and_then(|(id, dir)| logwatch::osc_avatar_params(&dir, id));
    let own = profile.map(|p| p.param_names.clone()).unwrap_or_default();
    let version = app.package_info().version.to_string();
    report::render(&report::Input {
        version: &version,
        vrchat_running: st.vrchat_running,
        linked: st.osc_active,
        oscquery: d.settings.osc.use_oscquery,
        read_logs: d.settings.read_logs,
        last_heard_secs: st.last_heard_ms.map(|t| (now_ms() - t) / 1000),
        avatar_id: st.avatar_id.as_deref(),
        avatar_name: profile.and_then(|p| p.name.as_deref()),
        loading: st.avatar_loading,
        live: st.avatar_params.as_deref(),
        file: file.as_deref(),
        saved: profile.map(|p| p.params.as_slice()),
        own_names: &own,
        mapping: &mapping,
        sps_sockets: st.sps_found.len(),
        instance_kind: st.instance_kind,
        hide_belly: mapping.sync_role(SyncRole::HideBelly).and_then(|r| d.sync_values.get(&r.name)).is_some_and(|&v| v > 0.5),
        auto_hidden: st.auto_hidden,
        shown_here: st.shown_here,
        labor_held: st.labor_held,
    })
}

/// Opens the troubleshooting guide on GitHub.
#[tauri::command]
pub fn open_troubleshooting(app: AppHandle) -> R<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_url(TROUBLESHOOTING_PAGE, None::<&str>).map_err(|e| e.to_string())
}

/// Debug: pretend a WooHoo happened. `outcome`: None = roll with your
/// fertility, Some(true) = pregnant, Some(false) = not this time.
#[tauri::command]
pub fn debug_woohoo(s: S, outcome: Option<bool>) {
    let roll = outcome.map(|yes| if yes { 0.0 } else { 0.999_999 });
    s.send(EngineCmd::DebugWoohoo(roll));
}

fn test_person(name: &str, user_id: Option<String>) -> R<(String, Option<String>)> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Give the test partner a name.".into());
    }
    let uid = user_id.map(|u| u.trim().to_string()).filter(|u| !u.is_empty());
    if let Some(u) = &uid {
        if !crate::pictures::valid_user_id(u) {
            return Err("That isn't a user ID. It looks like usr_ followed by letters and numbers.".into());
        }
    }
    Ok((name.to_string(), uid))
}

/// Debug: pretend someone joined your instance a while ago (so a WooHoo
/// right after counts as a settled 1-on-1).
#[tauri::command]
pub fn debug_add_person(app: AppHandle, s: S, name: String, user_id: Option<String>) -> R<()> {
    let (name, uid) = test_person(&name, user_id)?;
    {
        let mut d = s.data.lock().unwrap();
        let now = now_ms();
        let at = now - (d.settings.settle_minutes as i64 + 1) * 60_000;
        let inst = &mut d.instance;
        if inst.joined_ms.is_none_or(|j| j > at) {
            inst.joined_ms = Some(at);
        }
        if inst.world_name.is_none() {
            inst.world_name = Some("Debug world".into());
        }
        inst.apply(&pregsys_core::logparse::LogEvent::PlayerJoined { name, user_id: uid }, at);
    }
    changed(&app, &s, false);
    let _ = app.emit("instance-changed", ());
    Ok(())
}

/// Debug: add a test buddy (a pretend partner with a few stats), or remove
/// them and everything they touched.
#[tauri::command]
pub fn debug_test_buddy(app: AppHandle, s: S, index: usize, on: bool) {
    {
        let mut d = s.data.lock().unwrap();
        if on {
            pregsys_core::testbuddy::add(&mut d, index, now_ms());
        } else {
            pregsys_core::testbuddy::remove(&mut d, index, now_ms());
        }
    }
    changed(&app, &s, true);
    let _ = app.emit("instance-changed", ());
}

/// Debug: start labor now (`None`) or move it along, on the pregnancy you're wearing.
#[tauri::command]
pub fn debug_labor(app: AppHandle, s: S, skip: Option<pregsys_core::labor::Skip>) -> R<()> {
    let ok = {
        let mut d = s.data.lock().unwrap();
        let aid = d.current_avatar_id.clone();
        let id = d.current_pregnancy_id(aid.as_deref()).ok_or("Not pregnant on this avatar.")?;
        match skip {
            None => pregsys_core::labor::start(&mut d, &id, now_ms()),
            Some(how) => pregsys_core::labor::skip(&mut d, &id, how),
        }
    };
    if !ok {
        return Err(if skip.is_none() { "Already in labor." } else { "Not in labor yet." }.into());
    }
    changed(&app, &s, true);
    Ok(())
}

/// Debug: labor never happened on the pregnancy you're wearing (it goes on).
#[tauri::command]
pub fn debug_cancel_labor(app: AppHandle, s: S) -> R<()> {
    let ok = {
        let mut d = s.data.lock().unwrap();
        let aid = d.current_avatar_id.clone();
        let id = d.current_pregnancy_id(aid.as_deref()).ok_or("Not pregnant on this avatar.")?;
        pregsys_core::labor::cancel(&mut d, &id)
    };
    if !ok {
        return Err("Not in labor.".into());
    }
    changed(&app, &s, true);
    Ok(())
}

/// Debug: end the SPS WooHoo session, so the next one counts without the quiet gap.
#[tauri::command]
pub fn debug_new_woohoo_session(s: S) {
    s.send(EngineCmd::DebugNewSession);
}

/// Debug: how many babies (1 to 4), on the pregnancy you're wearing, or on
/// the grace period if you're still waiting to find out.
#[tauri::command]
pub fn debug_set_babies(app: AppHandle, s: S, count: u32) -> R<()> {
    let count = count.clamp(1, 4);
    {
        let mut d = s.data.lock().unwrap();
        let aid = d.current_avatar_id.clone();
        if let Some(id) = d.current_pregnancy_id(aid.as_deref()) {
            let p = d.pregnancy_mut(&id).ok_or("Pregnancy not found.")?;
            p.baby_count = count;
            p.sync_babies();
            actions::ensure_genders(p);
        } else {
            let global = d.settings.sync_scope == pregsys_core::model::SyncScope::Global;
            let owner = d.owner(aid.as_deref());
            let w = d.waiting.iter_mut().find(|w| global || w.avatar_id == owner).ok_or("Not pregnant or waiting on this avatar.")?;
            w.baby_count = count;
        }
    }
    changed(&app, &s, true);
    Ok(())
}

/// Debug: someone leaves the instance.
#[tauri::command]
pub fn debug_remove_person(app: AppHandle, s: S, name: String, user_id: Option<String>) {
    {
        let mut d = s.data.lock().unwrap();
        // Backdated, so a debug WooHoo right after still counts as a settled 1-on-1.
        let at = now_ms() - (d.settings.settle_minutes as i64 + 1) * 60_000;
        d.instance.apply(&pregsys_core::logparse::LogEvent::PlayerLeft { name, user_id }, at);
    }
    changed(&app, &s, false);
    let _ = app.emit("instance-changed", ());
}

/// Debug: fetch a profile picture even if pictures are off, and report why it fails.
#[tauri::command]
pub async fn debug_picture(s: S<'_>, user_id: String) -> R<Option<String>> {
    let (_, uid) = test_person("x", Some(user_id))?;
    let dir = s.data_dir.join("pictures");
    let uid = uid.unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || crate::pictures::get(&dir, &uid))
        .await
        .map_err(|e| e.to_string())?
}

/// Debug: kick right now (weak, strong or extreme).
#[tauri::command]
pub fn debug_kick(s: S, strength: pregsys_core::movement::KickStrength) {
    s.send(EngineCmd::DebugKick(strength));
}

/// Debug: end every grace period now (revealed within a second).
#[tauri::command]
pub fn debug_reveal_now(app: AppHandle, s: S) {
    let now = now_ms();
    for w in s.data.lock().unwrap().waiting.iter_mut() {
        w.reveal_ms = now;
    }
    changed(&app, &s, false);
}

/// "Find out now" on the waiting card (the engine reveals it within a second).
#[tauri::command]
pub fn find_out_now(app: AppHandle, s: S, time_ms: i64) {
    actions::find_out_now(&mut s.data.lock().unwrap(), time_ms, now_ms());
    changed(&app, &s, false);
}

/// "We used protection": calls the wait off.
#[tauri::command]
pub fn used_protection(app: AppHandle, s: S, time_ms: i64) {
    actions::used_protection(&mut s.data.lock().unwrap(), time_ms);
    changed(&app, &s, true);
}

/// Debug: takes back everything unlocked while debug mode was on.
#[tauri::command]
pub fn debug_clear_test_achievements(app: AppHandle, s: S) {
    pregsys_core::achievements::clear_test(&mut s.data.lock().unwrap());
    changed(&app, &s, false);
}

/// Debug: jump the fertility cycle to a day (1 to 28).
#[tauri::command]
pub fn debug_set_cycle_day(app: AppHandle, s: S, day: f64) {
    {
        let mut d = s.data.lock().unwrap();
        let cycle = pregsys_core::fertility::cycle_ms(&d.settings);
        let day = day.clamp(1.0, 28.99);
        d.settings.cycle_anchor_ms = Some(now_ms() - ((day - 1.0) / 28.0 * cycle as f64) as i64);
    }
    changed(&app, &s, true);
}

/// Hides the reveal card after the user has seen it.
#[tauri::command]
pub fn dismiss_reveal(app: AppHandle, s: S) {
    if let Some(r) = s.data.lock().unwrap().last_reveal.as_mut() {
        r.seen = true;
    }
    changed(&app, &s, false);
}

/// Starts a new cycle today (day 1).
#[tauri::command]
pub fn restart_cycle(app: AppHandle, s: S) {
    s.data.lock().unwrap().settings.cycle_anchor_ms = Some(now_ms());
    changed(&app, &s, false);
}

/// A partner's VRChat profile picture (data URL), if pictures are turned on.
#[tauri::command]
pub async fn partner_picture(s: S<'_>, user_id: String) -> R<Option<String>> {
    if !s.data.lock().unwrap().settings.show_pictures {
        return Ok(None);
    }
    let dir = s.data_dir.join("pictures");
    tauri::async_runtime::spawn_blocking(move || crate::pictures::get(&dir, &user_id))
        .await
        .map_err(|e| e.to_string())?
}

/// An avatar's public name and thumbnail, if pictures are turned on.
#[tauri::command]
pub async fn avatar_preview(s: S<'_>, avatar_id: String) -> R<Option<crate::pictures::AvatarPreview>> {
    if !s.data.lock().unwrap().settings.show_pictures {
        return Ok(None);
    }
    let dir = s.data_dir.join("pictures");
    tauri::async_runtime::spawn_blocking(move || crate::pictures::avatar(&dir, &avatar_id).map(Some))
        .await
        .map_err(|e| e.to_string())?
}

/// Adds an avatar you haven't worn yet, by its ID.
#[tauri::command]
pub fn add_avatar(app: AppHandle, s: S, avatar_id: String, name: Option<String>) -> R<AvatarProfile> {
    let id = avatar_id.trim().to_string();
    if !crate::pictures::valid_avatar_id(&id) {
        return Err("That isn't an avatar ID. It looks like avtr_ followed by letters and numbers.".into());
    }
    let profile = {
        let mut d = s.data.lock().unwrap();
        if d.avatars.contains_key(&id) {
            return Err("That avatar is already in your list.".into());
        }
        let p = AvatarProfile { id: id.clone(), name: name.filter(|n| !n.trim().is_empty()).map(|n| n.trim().to_string()), ..Default::default() };
        d.avatars.insert(id, p.clone());
        p
    };
    changed(&app, &s, false);
    Ok(profile)
}

/// Writes an export the user chose a location for in the save dialog.
#[tauri::command]
pub fn write_file(path: String, bytes: Vec<u8>) -> R<()> {
    std::fs::write(&path, bytes).map_err(|e| format!("Could not save {path}: {e}"))
}

#[tauri::command]
pub fn default_log_dirs() -> Vec<String> {
    pregsys_core::logwatch::default_log_dirs().into_iter().map(|p| p.to_string_lossy().to_string()).collect()
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::request_quit(&app);
}
