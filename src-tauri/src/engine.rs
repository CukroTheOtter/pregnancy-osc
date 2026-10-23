//! Background loop: VRChat/SteamVR detection, the OSC bridge, the log
//! reader, the driver tick, autosave and UI updates.

use crate::state::{EngineCmd, Shared};
use crate::system::Procs;
use pregsys_core::actions;
use pregsys_core::chatbox::Vars;
use pregsys_core::driver::{Driver, ParamValue, Send, SyncReport};
use pregsys_core::logwatch::{find_log_dir, LogWatcher};
use pregsys_core::mapping::{Mapping, OSC_PARAM_PREFIX};
use pregsys_core::now_ms;
use pregsys_osc::{Bridge, BridgeConfig, OscValue};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const TICK: Duration = Duration::from_millis(200);

pub fn spawn(app: AppHandle, shared: Arc<Shared>) {
    let (tx, rx) = mpsc::channel();
    *shared.engine.lock().unwrap() = Some(tx);
    std::thread::Builder::new()
        .name("engine".into())
        .spawn(move || Engine::new(app, shared).run(rx))
        .expect("engine thread");
}

struct Engine {
    app: AppHandle,
    shared: Arc<Shared>,
    driver: Driver,
    bridge: Option<Bridge>,
    bridge_error: Option<String>,
    bridge_error_at: Instant,
    procs: Procs,
    vrchat: bool,
    steamvr: bool,
    steamvr_seen: bool,
    logs: Option<LogWatcher>,
    last_proc: Instant,
    last_log: Instant,
    last_status: Instant,
    last_save: Instant,
    last_avatar_change: Option<Instant>,
    /// The avatar name from the log's last "Switching you to avatar X" line, and
    /// when it was seen. VRChat logs it when loading starts, before it says
    /// which avatar ID it is, so it waits here for the matching change.
    log_avatar_name: Option<(String, Instant)>,
    pending_params_fetch: Option<Instant>,
    /// Check the worn avatar more often until then (the log said you're switching).
    fast_poll_until: Option<Instant>,
    /// In-game App Controls buttons.
    controls: pregsys_core::controls::Controls,
    sps: pregsys_core::sps::Detector,
    /// SPS parameters VRChat is asked to send us (they change per avatar).
    sps_listen: Vec<String>,
    /// Chatbox: notices pause/resume/start/stop, and what it shows.
    chat_watch: pregsys_core::chatbox::Watch,
    chat_screen: pregsys_core::chatbox::Screen,
    /// When anything last arrived from VRChat (shown with the link details).
    last_heard_ms: Option<i64>,
    /// Debug chatbox: when the last "belly put back" note was shown.
    last_snap_note: Option<Instant>,
}

fn to_osc(v: ParamValue) -> OscValue {
    match v {
        ParamValue::Float(f) => OscValue::Float(f),
        ParamValue::Int(i) => OscValue::Int(i),
        ParamValue::Bool(b) => OscValue::Bool(b),
    }
}

impl Engine {
    fn new(app: AppHandle, shared: Arc<Shared>) -> Self {
        let mapping = shared.mapping.lock().unwrap().clone();
        let past = Instant::now() - Duration::from_secs(3600);
        Self {
            app,
            shared,
            driver: Driver::new(mapping),
            bridge: None,
            bridge_error: None,
            bridge_error_at: past,
            procs: Procs::new(),
            vrchat: false,
            steamvr: false,
            steamvr_seen: false,
            logs: None,
            last_proc: past,
            last_log: past,
            last_status: past,
            last_save: Instant::now(),
            last_avatar_change: None,
            log_avatar_name: None,
            pending_params_fetch: None,
            fast_poll_until: None,
            controls: Default::default(),
            sps: Default::default(),
            sps_listen: Vec::new(),
            chat_watch: Default::default(),
            chat_screen: Default::default(),
            last_heard_ms: None,
            last_snap_note: None,
        }
    }

    fn run(mut self, rx: Receiver<EngineCmd>) {
        loop {
            match rx.recv_timeout(TICK) {
                Ok(EngineCmd::Resync) => self.driver.resync(),
                Ok(EngineCmd::Restart) => self.restart(),
                Ok(EngineCmd::Pulse(key, ms)) => self.driver.pulse(&key, now_ms(), ms),
                Ok(EngineCmd::DebugWoohoo(roll)) => self.woohoo(now_ms(), roll),
                Ok(EngineCmd::DebugKick(s)) => self.driver.kick_now(s),
                Ok(EngineCmd::Shutdown(reply)) => {
                    self.handoff();
                    let _ = self.shared.save_if_dirty();
                    let _ = reply.send(());
                    return;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            self.step();
        }
    }

    fn restart(&mut self) {
        if let Ok(m) = Mapping::load_or_create(&self.shared.mapping_path) {
            *self.shared.mapping.lock().unwrap() = m.clone();
            self.driver.set_mapping(m);
        }
        self.handoff();
        self.bridge = None;
        self.bridge_error = None;
        self.logs = None;
        self.driver.set_connected(false);
    }

    fn handoff(&mut self) {
        let sends = {
            let data = self.shared.data.lock().unwrap();
            self.driver.handoff(now_ms(), &data)
        };
        self.send_all(&sends);
    }

    fn send_all(&self, sends: &[Send]) {
        if let Some(b) = &self.bridge {
            for s in sends {
                b.send(&format!("{OSC_PARAM_PREFIX}{}", s.name), to_osc(s.value));
            }
        }
    }

    fn step(&mut self) {
        let now = now_ms();
        if self.last_proc.elapsed() >= Duration::from_secs(3) {
            self.last_proc = Instant::now();
            self.check_processes();
        }
        self.ensure_bridge();
        self.drain_osc(now);
        self.save_held_sync(now);
        if let Some(c) = self.controls.tick(now) {
            self.apply_control(c, now);
        }
        self.fetch_params_if_due();

        let sends = {
            let data = self.shared.data.lock().unwrap();
            self.driver.tick(now, &data)
        };
        let last = |key: &str| match sends.iter().rev().find(|s| s.key == key).map(|s| s.value) {
            Some(ParamValue::Float(f)) => Some(f),
            _ => None,
        };
        let (stage, extra) = (last("stage"), last("bellyExtra"));
        if stage.is_some() || extra.is_some() {
            let mut st = self.shared.status.lock().unwrap();
            if stage.is_some() {
                st.last_stage = stage;
            }
            if extra.is_some() {
                st.last_extra = extra;
            }
        }
        self.send_all(&sends);
        self.log_kicks(now);
        self.tick_chatbox(now);

        // SPS sockets: enough time inside counts as a WooHoo.
        let (sps_settings, avatar_sps) = {
            let d = self.shared.data.lock().unwrap();
            let a = self.driver.avatar_id().and_then(|id| d.avatars.get(id)).map(|p| p.sps.clone());
            (d.settings.sps.clone(), a)
        };
        if self.sps.tick(now, &sps_settings, avatar_sps.as_ref()) {
            self.woohoo(now, None);
        }

        if self.last_log.elapsed() >= Duration::from_secs(1) {
            self.last_log = Instant::now();
            self.read_logs(now);
            self.check_reveal(now);
        }
        if self.last_save.elapsed() >= Duration::from_secs(2) {
            self.check_achievements(now);
            self.last_save = Instant::now();
            if let Err(e) = self.shared.save_if_dirty() {
                self.shared.status.lock().unwrap().note = Some(format!("Could not save: {e}"));
            }
        }
        if self.last_status.elapsed() >= Duration::from_secs(1) {
            self.last_status = Instant::now();
            self.publish_status();
        }
    }

    fn check_processes(&mut self) {
        let r = self.procs.check();
        if self.vrchat && !r.vrchat {
            // VRChat closed: release the avatar and go quiet. The bridge stays
            // up so the next VRChat finds it at start-up.
            self.handoff();
            self.driver.set_connected(false);
            self.shared.data.lock().unwrap().instance.vrchat_closed(pregsys_core::now_ms());
            self.shared.mark_dirty();
            let _ = self.app.emit("instance-changed", ());
        } else if !self.vrchat && r.vrchat && self.bridge.is_some() {
            self.driver.set_connected(true);
            self.pending_params_fetch = Some(Instant::now() + Duration::from_secs(3));
        }
        self.vrchat = r.vrchat;
        self.steamvr = r.steamvr;
        if r.steamvr {
            self.steamvr_seen = true;
        } else if self.steamvr_seen {
            self.steamvr_seen = false;
            let exit = self.shared.data.lock().unwrap().settings.exit_with_steamvr;
            if exit {
                let _ = self.app.emit("steamvr-closed", ());
                crate::request_quit(&self.app);
            }
        }
    }

    fn ensure_bridge(&mut self) {
        let osc = self.shared.data.lock().unwrap().settings.osc.clone();
        // Started even before VRChat runs: VRChat only looks for OSCQuery apps
        // at its start-up, so the app must already be there (e.g. VRCX
        // launching both at once).
        if !osc.enabled {
            if self.bridge.is_some() {
                self.handoff();
                self.bridge = None;
                self.driver.set_connected(false);
            }
            return;
        }
        if self.bridge.is_some() {
            return;
        }
        // After a failure (e.g. port in use), retry every 15 seconds.
        if self.bridge_error.is_some() && self.bridge_error_at.elapsed() < Duration::from_secs(15) {
            return;
        }
        let listen = self.listen_addresses();
        let cfg = BridgeConfig {
            use_oscquery: osc.use_oscquery,
            send_host: osc.send_host,
            send_port: osc.send_port,
            recv_port: osc.recv_port,
            listen_addresses: listen,
            ..Default::default()
        };
        match Bridge::start(&cfg) {
            Ok(b) => {
                self.bridge = Some(b);
                self.bridge_error = None;
                self.driver.set_connected(self.vrchat);
                if self.vrchat {
                    self.pending_params_fetch = Some(Instant::now() + Duration::from_secs(3));
                }
            }
            Err(e) => {
                self.bridge_error = Some(e);
                self.bridge_error_at = Instant::now();
            }
        }
    }

    /// Addresses VRChat is asked to send us over OSCQuery.
    fn listen_addresses(&self) -> Vec<String> {
        let mapping = self.driver.mapping();
        let mut listen = vec!["/avatar/change".to_string()];
        listen.extend(mapping.parameters.values().filter(|p| !p.name.is_empty()).map(|p| format!("{OSC_PARAM_PREFIX}{}", p.name)));
        // SPS sockets on this avatar, for detecting WooHoos.
        listen.extend(self.sps_listen.iter().map(|n| format!("{OSC_PARAM_PREFIX}{n}")));
        // Sync parameters too, so their in-game changes reach the app.
        listen.extend(mapping.sync.iter().filter(|p| !p.name.is_empty()).map(|p| format!("{OSC_PARAM_PREFIX}{}", p.name)));
        listen
    }

    fn drain_osc(&mut self, now: i64) {
        let Some(b) = &self.bridge else { return };
        b.refresh_target();
        let msgs = match b.drain() {
            Ok(m) => m,
            Err(e) => {
                self.bridge_error = Some(e);
                self.bridge_error_at = Instant::now();
                self.bridge = None;
                return;
            }
        };
        if !msgs.is_empty() {
            if self.last_heard_ms.is_none_or(|t| now - t > 60_000) {
                self.debug_chat("heard", "Hearing VRChat (OSC is coming in)".into());
            }
            self.last_heard_ms = Some(now);
        }
        for m in msgs {
            if m.addr == "/avatar/change" {
                if let OscValue::Str(id) = &m.value {
                    // It can arrive twice (OSCQuery and port 9001): one change is enough.
                    let repeat = self.driver.avatar_id() == Some(id.as_str())
                        && self.last_avatar_change.is_some_and(|t| t.elapsed() < Duration::from_secs(2));
                    if !repeat {
                        self.on_avatar_change(id, now);
                    }
                }
                continue;
            }
            let Some(name) = m.addr.strip_prefix(OSC_PARAM_PREFIX) else { continue };
            let Some(v) = m.value.as_f32() else { continue };
            // App Controls buttons from the avatar's menu.
            if let Some(c) = self.driver.mapping().key_for_name(name).and_then(pregsys_core::controls::Control::from_key) {
                if v > 0.5 {
                    self.debug_chat("menuButtons", format!("Menu button: {c:?}"));
                }
                if let Some(c) = self.controls.on_param(c, v, now) {
                    self.apply_control(c, now);
                }
                continue;
            }
            if self.sps.on_param(name, v) {
                continue;
            }
            match self.driver.on_sync_param(name, v, now) {
                SyncReport::NotSync => {}
                // Remembered a moment later (save_held_sync), unless the avatar changes first.
                SyncReport::Held => continue,
                SyncReport::Ignored => {
                    let saved = self.shared.data.lock().unwrap().sync_values.get(name).copied();
                    // Only worth a note when it differs from what's saved (the avatar's default).
                    if saved.is_some_and(|s| (s as f32 - v).abs() > 0.01) {
                        self.debug_chat("sync", format!("Sync ignored while loading: {name} = {v}"));
                    }
                    continue;
                }
            }
            let (pregnant, hold_flat) = {
                let d = self.shared.data.lock().unwrap();
                let pregnant = d.current_pregnancy(self.driver.avatar_id()).is_some();
                (pregnant, !pregnant && d.pending_for(self.driver.avatar_id()).is_some())
            };
            self.driver.on_param(name, v, pregnant, hold_flat);
        }
        // One note per few seconds, even while a menu slider is being dragged.
        if let Some((game, app)) = self.driver.take_snap_back() {
            if self.last_snap_note.is_none_or(|t| t.elapsed() >= Duration::from_secs(5)) {
                self.last_snap_note = Some(Instant::now());
                self.debug_chat("bellySnap", format!("Belly put back: game {:.0}% → app {:.0}%", game * 100.0, app * 100.0));
            }
        }
    }

    /// Remembers in-game sync changes that weren't followed by an avatar switch.
    fn save_held_sync(&mut self, now: i64) {
        for name in self.driver.take_sync_dropped() {
            self.debug_chat("sync", format!("Sync not saved (avatar switch): {name}"));
        }
        let hold = self.shared.data.lock().unwrap().settings.sync_wait_ms as i64;
        for (name, v) in self.driver.take_sync_saves(now, hold) {
            let mut d = self.shared.data.lock().unwrap();
            if d.sync_values.get(&name) == Some(&v) {
                continue;
            }
            d.sync_values.insert(name.clone(), v);
            drop(d);
            self.shared.mark_dirty();
            let _ = self.app.emit("data-changed", ());
            self.debug_chat("sync", format!("Sync saved: {name} = {v}"));
        }
    }

    fn on_avatar_change(&mut self, id: &str, now: i64) {
        self.driver.on_avatar_change(id, now);
        self.sps.reset();
        self.last_avatar_change = Some(Instant::now());
        if !self.name_avatar_from_vrchat() {
            self.name_avatar_from_log();
        }
        {
            let mut d = self.shared.data.lock().unwrap();
            actions::touch_avatar(&mut d, id, now);
        }
        self.shared.mark_dirty();
        let name = self.shared.data.lock().unwrap().avatars.get(id).and_then(|a| a.name.clone());
        self.debug_chat("avatarChange", format!("Avatar change: {}", name.unwrap_or_else(|| id.to_string())));
        // VRChat needs a moment before OSCQuery shows the new avatar.
        self.pending_params_fetch = Some(Instant::now() + Duration::from_millis(1500));
        let _ = self.app.emit("avatar-changed", id);
        let _ = self.app.emit("data-changed", ());
    }

    fn fetch_params_if_due(&mut self) {
        let Some(at) = self.pending_params_fetch else { return };
        if !self.vrchat || Instant::now() < at {
            return;
        }
        self.pending_params_fetch = None;
        let Some(info) = self.bridge.as_ref().and_then(|b| b.avatar_info()) else {
            // VRChat not found yet (it can take a few seconds): try again soon.
            if self.bridge.is_some() && self.driver.avatar_param_names().is_none() {
                self.pending_params_fetch = Some(Instant::now() + Duration::from_secs(5));
            }
            return;
        };
        // Keep checking which avatar is worn, in case VRChat's /avatar/change never reached us.
        let every = if info.avatar_id.is_none() || self.driver.avatar_id().is_none() || self.driver.loading() {
            Duration::from_secs(1) // still finding your avatar (e.g. the app started while VRChat was running)
        } else if self.fast_poll_until.is_some_and(|t| Instant::now() < t) {
            Duration::from_secs(2)
        } else {
            Duration::from_secs(5)
        };
        self.pending_params_fetch = Some(Instant::now() + every);
        // Already wearing an avatar when the app linked up: pick it up now
        // instead of waiting for the next avatar switch.
        if let Some(id) = info.avatar_id.as_deref() {
            if self.driver.avatar_id() != Some(id) {
                self.on_avatar_change(id, now_ms());
                // on_avatar_change schedules a fresh parameter read for the new avatar.
                return;
            }
        }
        let params = info.params;
        // Ask VRChat to send this avatar's SPS socket values.
        let mut sps_names: Vec<String> = params
            .iter()
            .filter(|p| pregsys_core::sps::split(&p.name).is_some_and(|(_, c)| pregsys_core::sps::is_others_channel(c)))
            .map(|p| p.name.clone())
            .collect();
        sps_names.sort();
        // Switch VRCFury's haptics channels on (until it's on, every socket reads 0).
        let sps_on = self.shared.data.lock().unwrap().settings.sps.enabled;
        let enable = pregsys_core::sps::ENABLE_PARAM;
        let off = params.iter().any(|p| p.name == enable && p.value.as_ref().and_then(|v| v.as_f32()) != Some(1.0));
        if sps_on && !sps_names.is_empty() && off {
            if let Some(b) = &self.bridge {
                b.send(&format!("{OSC_PARAM_PREFIX}{enable}"), OscValue::Bool(true));
            }
        }
        if sps_names != self.sps_listen {
            self.sps_listen = sps_names;
            let listen = self.listen_addresses();
            if let Some(b) = &self.bridge {
                b.set_listen_addresses(&listen);
            }
        }
        let list: Vec<(String, Option<f32>)> = params.iter().map(|p| (p.name.clone(), p.value.as_ref().and_then(|v| v.as_f32()))).collect();
        let names: Vec<String> = list.iter().map(|(n, _)| n.clone()).collect();
        if self.driver.avatar_param_names() == Some(&names[..]) {
            return; // nothing new
        }
        let mapping = self.driver.mapping();
        let found = names.iter().filter(|n| mapping.key_for_name(n).is_some()).count();
        let belly = names.iter().any(|n| mapping.key_for_name(n) == Some("stage"));
        let note = format!("Avatar loaded: {} params, {found} for the app, belly {}", names.len(), if belly { "✓" } else { "missing" });
        if !self.driver.set_avatar_params(list, now_ms()) {
            return; // still the old avatar's list: check again soon
        }
        self.debug_chat("avatarLoaded", note);
        self.name_avatar_from_vrchat();
        if let Some(aid) = self.driver.avatar_id().map(str::to_string) {
            let mut d = self.shared.data.lock().unwrap();
            if let Some(p) = d.avatars.get_mut(&aid) {
                p.params = names;
            }
            drop(d);
            self.shared.mark_dirty();
        }
    }

    /// Records kicks the driver sent on the current pregnancy and tells the UI.
    fn log_kicks(&mut self, now: i64) {
        let kicks = self.driver.take_kicks();
        if kicks.is_empty() {
            return;
        }
        for k in &kicks {
            self.debug_chat("kicks", format!("Kick: {k:?}"));
        }
        {
            let mut d = self.shared.data.lock().unwrap();
            let Some(id) = d.current_pregnancy_id(self.driver.avatar_id()) else { return };
            if let Some(p) = d.pregnancy_mut(&id) {
                for s in &kicks {
                    p.kicks.push(pregsys_core::model::KickEvent { time_ms: now, strength: *s });
                }
                let extra = p.kicks.len().saturating_sub(pregsys_core::model::KICKS_KEPT);
                p.kicks.drain(..extra);
            }
        }
        self.shared.mark_dirty();
        let _ = self.app.emit("kick", kicks.last().copied());
        let _ = self.app.emit("data-changed", ());
    }

    /// Names the current avatar from VRChat's OSC config (exact, by ID). Fixes a
    /// name the app guessed earlier; never touches one the user typed.
    /// Returns true when the avatar has VRChat's name.
    fn name_avatar_from_vrchat(&mut self) -> bool {
        let Some(aid) = self.driver.avatar_id().map(str::to_string) else { return false };
        let dir = {
            let d = self.shared.data.lock().unwrap();
            d.settings.log_dir.clone().map(PathBuf::from).or_else(find_log_dir)
        };
        let Some(name) = dir.and_then(|dir| pregsys_core::logwatch::osc_avatar_name(&dir, &aid)) else { return false };
        let mut d = self.shared.data.lock().unwrap();
        let Some(p) = d.avatars.get_mut(&aid) else { return false };
        let ours = p.name.is_none() || p.auto_name;
        if ours && p.name.as_deref() != Some(name.as_str()) {
            p.name = Some(name);
            p.auto_name = true;
            drop(d);
            self.shared.mark_dirty();
            let _ = self.app.emit("data-changed", ());
        }
        true
    }

    /// Names the avatar you just switched into from the log's "Switching you to
    /// avatar X" line, when the line and the avatar change are close together
    /// (either can come first). Never renames an avatar that already has a name.
    fn name_avatar_from_log(&mut self) {
        let close = Duration::from_secs(60);
        let Some((name, seen)) = self.log_avatar_name.clone() else { return };
        let Some(changed) = self.last_avatar_change else { return };
        let gap = if seen > changed { seen - changed } else { changed - seen };
        if gap > close {
            return;
        }
        let Some(aid) = self.driver.avatar_id().map(str::to_string) else { return };
        let mut d = self.shared.data.lock().unwrap();
        if let Some(p) = d.avatars.get_mut(&aid) {
            if p.name.as_deref().is_none_or(|n| n.trim().is_empty()) {
                p.name = Some(name);
                p.auto_name = true;
                drop(d);
                self.log_avatar_name = None;
                self.shared.mark_dirty();
                let _ = self.app.emit("data-changed", ());
            }
        }
    }

    /// Backup for when VRChat hasn't told us the avatar yet (OSC not reaching
    /// us, or the app started late): the log has the avatar's name, so pick the
    /// one known avatar with exactly that name.
    fn avatar_from_log_name(&mut self, now: i64) {
        if self.driver.avatar_id().is_some() {
            return;
        }
        let Some((name, _)) = self.log_avatar_name.clone() else { return };
        let id = {
            let d = self.shared.data.lock().unwrap();
            let mut hits = d.avatars.values().filter(|p| p.name.as_deref() == Some(name.as_str()));
            match (hits.next(), hits.next()) {
                (Some(p), None) => p.id.clone(),
                _ => return, // unknown, or two avatars share the name
            }
        };
        self.on_avatar_change(&id, now);
    }

    /// Unlocks newly earned achievements and says so (once, even if several unlock together).
    fn check_achievements(&mut self, now: i64) {
        let new = pregsys_core::achievements::check(&mut self.shared.data.lock().unwrap(), now);
        if new.is_empty() {
            return;
        }
        self.shared.mark_dirty();
        let body = match new.as_slice() {
            [one] => format!("🏆 {}", pregsys_core::achievements::title(one).unwrap_or(one)),
            many => format!("🏆 You unlocked {} achievements", many.len()),
        };
        crate::notify(&self.app, "Achievement unlocked!", &body);
        for id in &new {
            let achievement = pregsys_core::achievements::title(id).unwrap_or(id).to_string();
            self.chat("achievement", Vars { achievement, ..Default::default() });
        }
        let _ = self.app.emit("achievements", new.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        let _ = self.app.emit("data-changed", ());
    }

    /// A button in the avatar's App Controls menu.
    fn apply_control(&mut self, c: pregsys_core::controls::Control, now: i64) {
        let msg = {
            let mut d = self.shared.data.lock().unwrap();
            pregsys_core::controls::apply(&mut d, self.driver.avatar_id(), c, now)
        };
        let Some(msg) = msg else { return };
        self.shared.mark_dirty();
        self.driver.resync();
        let _ = self.app.emit("data-changed", ());
        crate::notify(&self.app, "Pregnancy OSC", msg);
    }

    /// A WooHoo (from SPS sockets or debug). `roll` forces the outcome.
    fn woohoo(&mut self, now: i64, roll: Option<f64>) {
        let outcome = {
            let mut d = self.shared.data.lock().unwrap();
            actions::record_fun(&mut d, now, roll.unwrap_or_else(pregsys_core::fertility::roll))
        };
        self.shared.mark_dirty();
        self.driver.resync();
        let _ = self.app.emit("data-changed", ());
        if outcome != actions::FunOutcome::Ignored {
            self.chat("woohoo", Vars::default());
        }
        match outcome {
            actions::FunOutcome::Started => {
                let _ = self.app.emit("conception", ());
                crate::notify(&self.app, "A new little bean!", "Conception detected in-game. Your pregnancy has started.");
            }
            actions::FunOutcome::Pending => {
                let _ = self.app.emit("fun", ());
                crate::notify(&self.app, "💕 WooHoo!", "You'll find out later if a little one is on the way.");
            }
            actions::FunOutcome::Ignored => {}
        }
    }

    /// Ends the grace period when it's time (also right after start-up,
    /// if it ended while the app was closed).
    fn check_reveal(&mut self, now: i64) {
        let result = actions::resolve_due(&mut self.shared.data.lock().unwrap(), now);
        let Some(positive) = result else { return };
        self.shared.mark_dirty();
        self.driver.resync();
        let _ = self.app.emit("data-changed", ());
        let _ = self.app.emit("reveal", positive);
        if positive {
            crate::notify(&self.app, "You're pregnant! 💕", "Something did come of it.");
        } else {
            crate::notify(&self.app, "Not this time", "No little one from that WooHoo.");
            self.chat("notPregnant", Vars::default());
        }
    }

    /// Queues a chatbox message for an event (if the chatbox and that event are on).
    fn chat(&mut self, event: &str, vars: Vars) {
        if self.bridge.is_none() || !self.vrchat {
            return;
        }
        let msg = self.shared.data.lock().unwrap().settings.chatbox.message(event, &vars);
        if let Some(m) = msg {
            self.chat_screen.push(m);
        }
    }

    /// Debug mode: a testing note in the chatbox, if that one is on (works
    /// even with the chatbox off).
    fn debug_chat(&mut self, key: &str, text: String) {
        if self.bridge.is_none() || !self.vrchat {
            return;
        }
        let on = {
            let d = self.shared.data.lock().unwrap();
            d.settings.debug && d.settings.chatbox.debug_on(key)
        };
        if on {
            self.chat_screen.push(format!("🔧 {text}"));
        }
    }

    /// Spots pause/resume/start/stop, then shows event messages (cleared
    /// after a while) and the status line.
    fn tick_chatbox(&mut self, now: i64) {
        let events = {
            let d = self.shared.data.lock().unwrap();
            self.chat_watch.check(&d, self.driver.avatar_id(), now)
        };
        for (e, v) in events {
            self.chat(e, v);
        }
        let Some(b) = self.bridge.as_ref().filter(|_| self.vrchat) else {
            self.chat_screen.reset();
            return;
        };
        let (status, show_seconds, sound) = {
            let d = self.shared.data.lock().unwrap();
            let c = &d.settings.chatbox;
            (c.status_line(&d, self.driver.avatar_id(), now), c.show_seconds, c.sound)
        };
        if let Some(text) = self.chat_screen.tick(now, show_seconds, status.clone()) {
            // The sound only for events, not the status or clearing.
            let event = !text.is_empty() && Some(&text) != status.as_ref();
            b.chatbox(&text, sound && event);
        }
    }

    fn read_logs(&mut self, now: i64) {
        let (enabled, dir) = {
            let d = self.shared.data.lock().unwrap();
            (d.settings.read_logs, d.settings.log_dir.clone())
        };
        if !enabled {
            self.logs = None;
            return;
        }
        let want: Option<PathBuf> = dir.map(PathBuf::from).or_else(find_log_dir);
        let Some(want) = want else { return };
        if self.logs.as_ref().map(|l| l.dir()) != Some(want.as_path()) {
            self.logs = Some(LogWatcher::new(want));
        }
        let Some(w) = self.logs.as_mut() else { return };
        let Ok(poll) = w.poll() else { return };
        if poll.lines.is_empty() && !poll.new_file {
            return;
        }
        let mut d = self.shared.data.lock().unwrap();
        if poll.new_file {
            // Rebuild "who is here" from the start of the new log, keep history.
            let history = std::mem::take(&mut d.instance.history);
            let local = d.instance.local.clone();
            d.instance = Default::default();
            d.instance.history = history;
            d.instance.local = local;
        }
        let mut local_avatar: Option<String> = None; // newest switch line in this batch
        for l in &poll.lines {
            // Replayed lines keep their own time; live lines use now.
            let at = if poll.new_file { l.time_ms.unwrap_or(now) } else { now };
            d.instance.apply(&l.event, at);
            if let pregsys_core::logparse::LogEvent::AvatarSwitch { player, avatar_name } = &l.event {
                if d.instance.local.as_ref().is_some_and(|me| &me.name == player) {
                    local_avatar = Some(avatar_name.clone());
                    if !poll.new_file {
                        // Values VRChat reports from now on are the next avatar's, not yours.
                        self.driver.on_switch_start(l.time_ms.unwrap_or(now - 1_500), now);
                        self.log_avatar_name = Some((avatar_name.clone(), Instant::now()));
                        // You're switching: the new avatar is still loading, so watch closely for it.
                        self.fast_poll_until = Some(Instant::now() + Duration::from_secs(60));
                        self.pending_params_fetch = Some(Instant::now() + Duration::from_secs(2));
                    }
                }
            }
        }
        drop(d);
        // A new log file replays from the start: its last switch line is the avatar you're wearing now.
        if poll.new_file {
            if let Some(name) = local_avatar {
                self.log_avatar_name = Some((name, Instant::now()));
            }
        }
        // The log line can also arrive after the avatar change.
        if !self.name_avatar_from_vrchat() {
            self.name_avatar_from_log();
        }
        self.avatar_from_log_name(now);
        self.shared.mark_dirty();
        let _ = self.app.emit("instance-changed", ());
    }

    fn publish_status(&mut self) {
        let mut st = self.shared.status.lock().unwrap();
        st.vrchat_running = self.vrchat;
        st.last_heard_ms = self.last_heard_ms;
        st.steamvr_running = self.steamvr;
        st.osc_active = self.bridge.is_some() && self.vrchat;
        st.avatar_id = self.driver.avatar_id().map(str::to_string);
        st.avatar_params = self.driver.avatar_param_names().map(|p| p.to_vec());
        st.tray_available = self.shared.tray_available.load(std::sync::atomic::Ordering::SeqCst);
        {
            let (sps_settings, avatar_sps) = {
                let d = self.shared.data.lock().unwrap();
                let a = self.driver.avatar_id().and_then(|id| d.avatars.get(id)).map(|p| p.sps.clone());
                (d.settings.sps.clone(), a)
            };
            st.sps_found = self
                .driver
                .avatar_param_names()
                .map(|p| pregsys_core::sps::sockets(p).into_keys().collect())
                .unwrap_or_default();
            st.sps_live = self.sps.live(&sps_settings, avatar_sps.as_ref());
            st.sps_progress = self.sps.progress(&sps_settings);
        }
        match &self.bridge {
            Some(b) => {
                let bs = b.status();
                st.oscquery = bs.oscquery;
                st.vrchat_found = bs.vrchat_found;
                st.send_target = Some(bs.send_target);
                st.recv_port = Some(bs.recv_port);
                st.backup_port = bs.backup_port;
                st.query_port = bs.query_port;
                st.vrchat_query = bs.vrchat_query;
                st.note = bs.note.or_else(|| self.bridge_error.clone());
            }
            None => {
                st.oscquery = false;
                st.vrchat_found = false;
                st.send_target = None;
                st.recv_port = None;
                st.backup_port = None;
                st.query_port = None;
                st.vrchat_query = None;
                st.note = self.bridge_error.clone();
            }
        }
        st.log_dir = self.logs.as_ref().map(|l| l.dir().to_string_lossy().to_string());
        st.log_file = self.logs.as_ref().and_then(|l| l.current_file()).map(|f| f.to_string_lossy().to_string());
        {
            let d = self.shared.data.lock().unwrap();
            st.world_name = d.instance.world_name.clone();
            st.others = d.instance.others().into_iter().cloned().collect();
        }
        let snapshot = st.clone();
        drop(st);
        let _ = self.app.emit("status", snapshot);
    }
}
