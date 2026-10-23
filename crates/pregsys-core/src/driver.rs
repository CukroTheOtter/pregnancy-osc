//! Decides what to send to the avatar, tick by tick. Pure logic: the app
//! feeds it what VRChat says and sends whatever it returns.

use crate::mapping::{Mapping, ParamType};
use crate::fertility;
use crate::movement::{self, KickStrength};
use crate::model::AppData;
use crate::timeline::TERM_WEEKS;
use std::collections::BTreeMap;
use crate::stage::{extra_value, stage_value, ExtraInput, StageInput};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParamValue {
    Float(f32),
    Int(i32),
    Bool(bool),
}

/// One parameter write: full avatar parameter name and value.
#[derive(Debug, Clone, PartialEq)]
pub struct Send {
    pub key: String,
    pub name: String,
    pub value: ParamValue,
}

/// Belly value drift (from the avatar's own clock or the menu) that makes
/// the app snap back.
const DRIFT: f32 = 0.002;
/// Re-send the belly at least this often even if unchanged, so a size set
/// in-game (menu or the avatar's own clock) is put back within a second.
const REFRESH_MS: i64 = 1_000;
/// After an avatar change, nothing is sent for this long: VRChat says the
/// avatar changed before it has finished loading, and values sent too early
/// are lost. Then everything is sent (and the remembered sync values applied).
const SYNC_SETTLE_MS: i64 = 3_000;
/// Everything is sent once more this long after an avatar change, in case it
/// was still loading the first time.
const RESEND_AFTER_MS: i64 = 10_000;
/// While a new avatar loads, VRChat reports its own defaults, which aren't
/// your choice: sync values reported then are ignored, until this long after
/// its parameters show up over OSCQuery...
const SYNC_IGNORE_AFTER_LOAD_MS: i64 = 1_500;
/// ...or this long after the change, if they never do.
const LOAD_CAP_MS: i64 = 30_000;
/// The same parameter list as the last avatar this soon after a change is
/// probably still the old avatar's, so it doesn't count as loaded yet.
const SAME_LIST_MS: i64 = 5_000;
/// A sync value reported in-game is held this long before it's remembered.
/// When you switch, VRChat reports the next avatar's own values a moment
/// BEFORE it says the avatar changed. The log's "Switching" line (written
/// when the switch starts) is the main guard; this hold is the backup for
/// when the log isn't read. The default; Settings > Sync can change it.
pub const SYNC_HOLD_MS: i64 = 2_000;

/// What happened to a reported parameter value (see [`Driver::on_sync_param`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncReport {
    /// Not a sync parameter.
    NotSync,
    /// A new avatar is loading: its own value, not your choice.
    Ignored,
    /// Held, remembered after the hold time unless the avatar changes.
    Held,
}

pub struct Driver {
    mapping: Mapping,
    connected: bool,
    avatar_id: Option<String>,
    /// Parameter names on the current avatar (None = not known, send anyway).
    avatar_params: Option<Vec<String>>,
    last_sent: HashMap<String, ParamValue>,
    last_stage_ms: i64,
    last_rx_stage: Option<f32>,
    need_snap: bool,
    need_profile: bool,
    beat: bool,
    last_beat_ms: i64,
    /// key -> (until_ms, true already sent)
    pulses: HashMap<String, (i64, bool)>,
    /// Custom parameter names for the current avatar (key -> name).
    overrides: BTreeMap<String, String>,
    /// After an avatar change: ignore the new avatar's own sync values until
    /// this time, then send the remembered ones.
    sync_until_ms: Option<i64>,
    /// After an avatar change: in-game sync values aren't remembered until then.
    sync_ignore_until_ms: Option<i64>,
    /// After an avatar change: when it happened, the old avatar's parameter
    /// list, and whether the new one has loaded (its parameters showed up).
    change_ms: i64,
    prev_params: Option<Vec<String>>,
    loading: bool,
    /// Sync values reported in-game, waiting to be remembered: name -> (value, when).
    held_sync: BTreeMap<String, (f64, i64)>,
    /// Held sync values dropped by an avatar change (for the debug note).
    dropped_sync: Vec<String>,
    /// Held sync values from before a switch started: remembered right away.
    ready_sync: Vec<(String, f64)>,
    need_sync: bool,
    /// After an avatar change: when the second full send is due.
    resend_at_ms: Option<i64>,
    /// A belly above 0 was sent for a pregnancy (survives resyncs), so it's
    /// flattened when the pregnancy ends.
    belly_up: bool,
    /// The pregnancy is paused: the belly and extra size are left alone
    /// (set them in-game), and sent again on resume.
    paused: bool,
    /// The belly was changed in-game and is being put back: (game, app).
    snap_back: Option<(f32, f32)>,
    /// Belly movement: when the next kick is due, when FetalMovement goes back to 0.
    next_kick_ms: Option<i64>,
    kick_reset_ms: Option<i64>,
    /// Kicks sent since the engine last asked (for the kick log).
    kicks_sent: Vec<KickStrength>,
    /// Debug: a kick to send on the next tick.
    forced_kick: Option<KickStrength>,
    /// Sends produced by helpers during a tick.
    extra: Vec<Send>,
}

impl Driver {
    pub fn new(mapping: Mapping) -> Self {
        Self {
            mapping,
            connected: false,
            avatar_id: None,
            avatar_params: None,
            last_sent: HashMap::new(),
            last_stage_ms: i64::MIN / 2,
            last_rx_stage: None,
            need_snap: true,
            need_profile: true,
            beat: false,
            last_beat_ms: i64::MIN / 2,
            pulses: HashMap::new(),
            overrides: BTreeMap::new(),
            sync_until_ms: None,
            sync_ignore_until_ms: None,
            change_ms: 0,
            prev_params: None,
            loading: false,
            held_sync: BTreeMap::new(),
            dropped_sync: Vec::new(),
            ready_sync: Vec::new(),
            need_sync: true,
            resend_at_ms: None,
            belly_up: false,
            paused: false,
            snap_back: None,
            next_kick_ms: None,
            kick_reset_ms: None,
            kicks_sent: Vec::new(),
            forced_kick: None,
            extra: Vec::new(),
        }
    }

    pub fn mapping(&self) -> &Mapping {
        &self.mapping
    }

    pub fn set_mapping(&mut self, m: Mapping) {
        self.mapping = m;
        self.resync();
    }

    pub fn avatar_id(&self) -> Option<&str> {
        self.avatar_id.as_deref()
    }

    pub fn connected(&self) -> bool {
        self.connected
    }


    /// Force a full re-send on the next tick (settings changed, etc.).
    pub fn resync(&mut self) {
        self.need_snap = true;
        self.need_profile = true;
        self.need_sync = true;
        self.last_sent.clear();
    }

    /// A sync parameter changed in-game. It's held for a moment, then
    /// remembered (see [`Driver::take_sync_saves`]).
    pub fn on_sync_param(&mut self, name: &str, v: f32, now_ms: i64) -> SyncReport {
        // Never treat an app parameter (or this avatar's custom name for one) as sync.
        if self.overrides.values().any(|n| n == name) {
            return SyncReport::NotSync;
        }
        let Some(def) = self.mapping.sync_def(name) else { return SyncReport::NotSync };
        if self.sync_ignored(now_ms) {
            return SyncReport::Ignored;
        }
        let value = match def.kind {
            ParamType::Bool => (v > 0.5) as i32 as f64,
            ParamType::Int => v.round() as f64,
            ParamType::Float => v as f64,
        };
        self.held_sync.insert(name.to_string(), (value, now_ms));
        SyncReport::Held
    }

    /// Held sync values old enough to be your choice: remember these.
    pub fn take_sync_saves(&mut self, now_ms: i64, hold_ms: i64) -> Vec<(String, f64)> {
        let ready: Vec<String> = self.held_sync.iter().filter(|(_, (_, t))| now_ms - t >= hold_ms).map(|(n, _)| n.clone()).collect();
        let mut out = std::mem::take(&mut self.ready_sync);
        out.extend(ready.into_iter().filter_map(|n| self.held_sync.remove(&n).map(|(v, _)| (n, v))));
        out
    }

    /// The log says you started switching avatars at `at_ms`. Values reported
    /// before that are yours (remembered now); from then until the new avatar
    /// has loaded, reported values are its own, however long it takes.
    pub fn on_switch_start(&mut self, at_ms: i64, now_ms: i64) {
        if self.change_ms >= at_ms {
            return; // the avatar change already came in
        }
        for (name, (v, t)) in std::mem::take(&mut self.held_sync) {
            if t < at_ms {
                self.ready_sync.push((name, v));
            } else {
                self.dropped_sync.push(name);
            }
        }
        self.loading = true;
        self.sync_ignore_until_ms = Some(now_ms + LOAD_CAP_MS);
    }

    /// Sync values dropped by the last avatar change (they were its values, not yours).
    pub fn take_sync_dropped(&mut self) -> Vec<String> {
        std::mem::take(&mut self.dropped_sync)
    }

    pub fn set_connected(&mut self, c: bool) {
        if c && !self.connected {
            self.resync();
        }
        self.connected = c;
    }

    pub fn on_avatar_change(&mut self, id: &str, now_ms: i64) {
        self.sync_until_ms = Some(now_ms + SYNC_SETTLE_MS);
        self.sync_ignore_until_ms = Some(now_ms + LOAD_CAP_MS);
        self.dropped_sync.extend(std::mem::take(&mut self.held_sync).into_keys());
        // The app never set a belly on this avatar, so there's nothing to flatten
        // (an avatar without a pregnancy keeps its own belly).
        self.belly_up = false;
        self.change_ms = now_ms;
        self.loading = true;
        self.resend_at_ms = Some(now_ms + RESEND_AFTER_MS);
        self.avatar_id = Some(id.to_string());
        self.prev_params = self.avatar_params.take().or(self.prev_params.take());
        self.last_rx_stage = None;
        self.connected = true;
        self.resync();
    }

    /// Avatar parameter list from OSCQuery, with current values when known.
    /// Returns false while it still looks like the old avatar's list.
    pub fn set_avatar_params(&mut self, params: Vec<(String, Option<f32>)>, now_ms: i64) -> bool {
        let names: Vec<String> = params.iter().map(|(n, _)| n.clone()).collect();
        if self.loading {
            if self.prev_params.as_ref() == Some(&names) && now_ms - self.change_ms < SAME_LIST_MS {
                return false;
            }
            // Loaded: its defaults are reported around now, so keep ignoring them
            // a moment longer, then send everything once more.
            self.loading = false;
            let until = now_ms + SYNC_IGNORE_AFTER_LOAD_MS;
            self.sync_ignore_until_ms = Some(until);
            self.resend_at_ms = self.resend_at_ms.max(Some(until));
        }
        for (name, v) in &params {
            if let (Some("stage"), Some(v)) = (self.mapping.key_for_name(name), v) {
                self.last_rx_stage = Some(*v);
            }
        }
        self.avatar_params = Some(names);
        // VRChat now lists this avatar's parameters, so it's (nearly) loaded: send everything again.
        self.resync();
        true
    }

    /// A new avatar is loading (its parameters haven't shown up yet).
    pub fn loading(&self) -> bool {
        self.loading
    }

    /// In-game sync values are being ignored right now (a new avatar is loading).
    pub fn sync_ignored(&self, now_ms: i64) -> bool {
        self.sync_until_ms.is_some_and(|t| now_ms < t) || self.sync_ignore_until_ms.is_some_and(|t| now_ms < t)
    }

    pub fn avatar_param_names(&self) -> Option<&[String]> {
        self.avatar_params.as_deref()
    }

    /// A parameter value VRChat reported. `pregnant` = the app currently has
    /// a pregnancy for this avatar; `hold_flat` = grace period, keep the belly at 0.
    pub fn on_param(&mut self, name: &str, v: f32, pregnant: bool, hold_flat: bool) {
        match self.mapping.key_for_name(name) {
            Some("stage") => {
                self.last_rx_stage = Some(v);
                if hold_flat && v.abs() > DRIFT {
                    // The avatar's own system started a belly: flatten it again.
                    self.need_snap = true;
                }
                if pregnant {
                    if let Some(ParamValue::Float(sent)) = self.last_sent.get("stage") {
                        if (v - sent).abs() > DRIFT {
                            self.need_snap = true;
                            if !self.paused {
                                self.snap_back = Some((v, *sent));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// The last in-game belly change being put back, if any: (game, app).
    pub fn take_snap_back(&mut self) -> Option<(f32, f32)> {
        self.snap_back.take()
    }

    /// Debug: send a kick of this strength right away.
    pub fn kick_now(&mut self, s: KickStrength) {
        self.forced_kick = Some(s);
    }

    /// Kicks sent since the last call (newest last).
    pub fn take_kicks(&mut self) -> Vec<KickStrength> {
        std::mem::take(&mut self.kicks_sent)
    }

    /// Starts a short true-then-false pulse on an event parameter.
    pub fn pulse(&mut self, key: &str, now_ms: i64, duration_ms: i64) {
        self.pulses.insert(key.to_string(), (now_ms + duration_ms, false));
    }

    /// Real parameter name to use for a key on this avatar, if any.
    fn name_for(&self, key: &str) -> Option<String> {
        // This avatar's own name for the parameter wins.
        if let Some(n) = self.overrides.get(key).filter(|n| !n.trim().is_empty()) {
            return Some(n.clone());
        }
        let def = self.mapping.get(key)?;
        match &self.avatar_params {
            Some(list) => self.mapping.resolve(key, list).map(str::to_string),
            // Parameter list unknown (no OSCQuery): VRChat ignores names the avatar lacks.
            None => Some(def.name.clone()),
        }
    }

    fn push(&mut self, out: &mut Vec<Send>, key: &str, value: ParamValue, force: bool) {
        if !force && self.last_sent.get(key) == Some(&value) {
            return;
        }
        if let Some(name) = self.name_for(key) {
            self.last_sent.insert(key.to_string(), value);
            out.push(Send { key: key.to_string(), name, value });
        }
    }

    fn stage_for(&self, data: &AppData, now_ms: i64) -> f32 {
        let aid = self.avatar_id.as_deref();
        let (_, scale, _) = data.belly_settings(aid);
        let progress = data.current_pregnancy(aid).map(|p| p.timeline.progress(now_ms));
        stage_value(StageInput { progress, belly_scale: scale })
    }

    pub fn tick(&mut self, now_ms: i64, data: &AppData) -> Vec<Send> {
        let mut out = Vec::new();
        if !self.connected {
            return out;
        }
        // A new avatar is still loading: wait, then send everything.
        if let Some(t) = self.sync_until_ms {
            if now_ms < t {
                return out;
            }
        }
        if self.resend_at_ms.is_some_and(|t| now_ms >= t) {
            self.resend_at_ms = None;
            self.resync();
        }
        let aid = self.avatar_id.clone();
        let preg = data.current_pregnancy(aid.as_deref());
        let paused = preg.is_some_and(|p| p.timeline.is_paused());
        if paused != self.paused {
            self.paused = paused;
            if !paused {
                self.resync();
            }
        }
        let overrides = aid.as_deref().and_then(|a| data.avatars.get(a)).map(|p| p.param_names.clone()).unwrap_or_default();
        if overrides != self.overrides {
            self.overrides = overrides;
            self.resync();
        }

        // App-active flag and heartbeat.
        if now_ms - self.last_beat_ms >= self.mapping.heartbeat.interval_ms as i64 {
            self.last_beat_ms = now_ms;
            self.beat = !self.beat;
            self.push(&mut out, "appActive", ParamValue::Bool(true), true);
            let b = self.beat;
            self.push(&mut out, "appHeartbeat", ParamValue::Bool(b), true);
        }

        // Sync parameters follow you onto every avatar, once it has loaded.
        if self.need_sync && self.sync_until_ms.is_none_or(|t| now_ms >= t) {
            self.need_sync = false;
            self.sync_until_ms = None;
            for def in self.mapping.sync.clone() {
                if self.mapping.is_app_param(&def.name) || self.overrides.values().any(|n| *n == def.name) {
                    continue;
                }
                let Some(&v) = data.sync_values.get(&def.name) else { continue };
                let value = match def.kind {
                    ParamType::Bool => ParamValue::Bool(v > 0.5),
                    ParamType::Int => ParamValue::Int(v.round() as i32),
                    ParamType::Float => ParamValue::Float(v as f32),
                };
                out.push(Send { key: format!("sync:{}", def.name), name: def.name.clone(), value });
            }
        }

        // Optional extras: only sent when changed.
        let force = self.need_profile;
        // Babies are only shown once they could be seen on a scan.
        let count = match preg {
            Some(p) if p.timeline.progress(now_ms) * TERM_WEEKS >= fertility::BABY_REVEAL_WEEK => p.baby_count as i32,
            _ => 0,
        };
        self.push(&mut out, "babyCount", ParamValue::Int(count), force);
        let waiting = preg.is_none() && data.pending_for(aid.as_deref()).is_some();
        self.push(&mut out, "waiting", ParamValue::Bool(waiting), force);
        let level = fertility::level_now(data, aid.as_deref(), now_ms) as i32;
        self.push(&mut out, "fertileLevel", ParamValue::Int(level), force);
        // Extra size: twins and more, and going overdue.
        let extra = preg.map_or(0.0, |p| {
            extra_value(ExtraInput {
                progress: p.timeline.progress(now_ms),
                babies: p.baby_count,
                multiples: data.settings.bigger_for_multiples,
                overdue: data.settings.grow_when_overdue,
                max_overdue_fraction: self.mapping.overdue.max_overdue_fraction,
            })
        });
        let last_extra = match self.last_sent.get("bellyExtra") {
            Some(ParamValue::Float(f)) => Some(*f),
            _ => None,
        };
        if paused {
            // Paused: leave the extra size alone.
        } else if force || last_extra.is_none_or(|l| (extra - l).abs() >= self.mapping.min_stage_change as f32 || (extra == 0.0 && l != 0.0)) {
            self.push(&mut out, "bellyExtra", ParamValue::Float(extra), force);
        }
        self.need_profile = false;

        // The belly.
        let stage = self.stage_for(data, now_ms);
        let last = match self.last_sent.get("stage") {
            Some(ParamValue::Float(f)) => Some(*f),
            _ => None,
        };
        if paused {
            // Paused: leave the belly alone.
        } else if preg.is_some() {
            let changed = last.is_none_or(|l| (stage - l).abs() >= self.mapping.min_stage_change as f32);
            let due = now_ms - self.last_stage_ms >= self.mapping.send_interval_ms as i64;
            let stale = now_ms - self.last_stage_ms >= REFRESH_MS;
            if self.need_snap || (changed && due) || stale {
                self.push(&mut out, "stage", ParamValue::Float(stage), true);
                self.last_stage_ms = now_ms;
                self.need_snap = false;
                self.belly_up = true;
            }
        } else if data.pending_for(aid.as_deref()).is_some() {
            // Grace period: hold the belly flat until the reveal.
            if self.need_snap || last != Some(0.0) || now_ms - self.last_stage_ms >= REFRESH_MS {
                self.push(&mut out, "stage", ParamValue::Float(0.0), true);
                self.last_stage_ms = now_ms;
                self.need_snap = false;
            }
        } else if self.belly_up || last.is_some_and(|l| l > 0.0) {
            // Pregnancy ended (birth) or removed: flatten once, then leave the belly alone.
            self.push(&mut out, "stage", ParamValue::Float(0.0), true);
            self.last_stage_ms = now_ms;
            self.belly_up = false;
        }

        // Belly movement.
        self.tick_kicks(now_ms, data, preg);

        out.append(&mut self.extra);

        // Event pulses.
        let keys: Vec<String> = self.pulses.keys().cloned().collect();
        for k in keys {
            let (until, sent) = self.pulses[&k];
            if !sent {
                self.push(&mut out, &k, ParamValue::Bool(true), true);
                self.pulses.insert(k.clone(), (until, true));
            } else if now_ms >= until {
                self.push(&mut out, &k, ParamValue::Bool(false), true);
                self.pulses.remove(&k);
            }
        }
        out
    }

    fn send_kick(&mut self, out_now_ms: i64, s: KickStrength) -> Option<Send> {
        let name = self.name_for("fetalMovement")?;
        let value = ParamValue::Int(self.mapping.kicks.value(s));
        self.kick_reset_ms = Some(out_now_ms + self.mapping.kicks.hold_ms.max(100));
        self.kicks_sent.push(s);
        Some(Send { key: "fetalMovement".into(), name, value })
    }

    fn tick_kicks(&mut self, now_ms: i64, data: &AppData, preg: Option<&crate::model::Pregnancy>) -> Option<()> {
        // A pending reset back to 0 comes first.
        if let Some(t) = self.kick_reset_ms {
            if now_ms >= t {
                self.kick_reset_ms = None;
                let name = self.name_for("fetalMovement")?;
                self.extra.push(Send { key: "fetalMovement".into(), name, value: ParamValue::Int(0) });
            }
            return Some(());
        }
        if let Some(s) = self.forced_kick.take() {
            if let Some(k) = self.send_kick(now_ms, s) {
                self.extra.push(k);
            }
            return Some(());
        }
        let s = &data.settings;
        let p = preg?;
        if !s.kicks || p.timeline.is_paused() {
            self.next_kick_ms = None;
            return None;
        }
        let week = p.timeline.progress(now_ms) * TERM_WEEKS;
        if week < movement::FIRST_KICK_WEEK {
            self.next_kick_ms = None;
            return None;
        }
        let due = *self.next_kick_ms.get_or_insert_with(|| now_ms + movement::next_kick_delay_ms(s.kick_frequency, p.baby_count, fertility::roll()));
        if now_ms < due {
            return Some(());
        }
        self.next_kick_ms = Some(now_ms + movement::next_kick_delay_ms(s.kick_frequency, p.baby_count, fertility::roll()));
        let strength = movement::pick_strength(week, s.allowed_kicks, fertility::roll())?;
        if let Some(k) = self.send_kick(now_ms, strength) {
            self.extra.push(k);
        }
        Some(())
    }

    /// On exit: final belly value, then release the app-active flag.
    pub fn handoff(&mut self, now_ms: i64, data: &AppData) -> Vec<Send> {
        let mut out = Vec::new();
        if !self.connected {
            return out;
        }
        if data.current_pregnancy(self.avatar_id.as_deref()).is_some() {
            let s = self.stage_for(data, now_ms);
            self.push(&mut out, "stage", ParamValue::Float(s), true);
        }
        for (k, _) in std::mem::take(&mut self.pulses) {
            self.push(&mut out, &k, ParamValue::Bool(false), true);
        }
        self.push(&mut out, "appActive", ParamValue::Bool(false), true);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Pregnancy, SyncScope};
    use crate::mapping::ParamDef;
    use crate::timeline::{DurationMode, Timeline, DAY_MS};

    const T0: i64 = 1_700_000_000_000;

    fn data_pregnant() -> AppData {
        let mut d = AppData::new();
        d.pregnancies.push(Pregnancy {
            id: "p".into(),
            timeline: Timeline::new(T0, DurationMode::Realtime, 1.0, None),
            ..Default::default()
        });
        d.active_pregnancy_id = Some("p".into());
        d
    }

    fn names(v: &[Send]) -> Vec<&str> {
        v.iter().map(|s| s.key.as_str()).collect()
    }

    fn stage_of(v: &[Send]) -> Option<f32> {
        v.iter().find(|s| s.key == "stage").and_then(|s| match s.value {
            ParamValue::Float(f) => Some(f),
            _ => None,
        })
    }

    #[test]
    fn idle_when_vrchat_absent() {
        let mut d = Driver::new(Mapping::default());
        assert!(d.tick(T0, &data_pregnant()).is_empty());
    }

    #[test]
    fn first_tick_snaps_and_beats() {
        let mut d = Driver::new(Mapping::default());
        d.set_connected(true);
        let out = d.tick(T0 + 140 * DAY_MS, &data_pregnant());
        let n = names(&out);
        assert!(n.contains(&"appActive") && n.contains(&"appHeartbeat"));
        assert!((stage_of(&out).unwrap() - 0.5).abs() < 1e-4);
    }

    #[test]
    fn stage_resent_every_second() {
        let mut d = Driver::new(Mapping::default());
        d.set_connected(true);
        let data = data_pregnant();
        d.tick(T0, &data);
        // Not on every tick...
        assert!(stage_of(&d.tick(T0 + 200, &data)).is_none());
        // ...but every second, so an in-game change is put back.
        assert!(stage_of(&d.tick(T0 + REFRESH_MS, &data)).is_some());
    }

    #[test]
    fn paused_lets_go_of_the_belly() {
        let mut d = Driver::new(Mapping::default());
        d.set_connected(true);
        let mut data = data_pregnant();
        assert!(stage_of(&d.tick(T0, &data)).is_some());
        data.pregnancies[0].timeline.pause(T0 + 100);
        let extra = |out: &[Send]| out.iter().any(|s| s.key == "bellyExtra");
        for i in 1..20 {
            let out = d.tick(T0 + i * 1000, &data);
            assert!(stage_of(&out).is_none() && !extra(&out), "hands off while paused");
        }
        d.on_param("Pregnancy/Stage", 1.0, true, false);
        assert!(stage_of(&d.tick(T0 + 25_000, &data)).is_none(), "in-game value kept");
        data.pregnancies[0].timeline.resume(T0 + 30_000);
        let out = d.tick(T0 + 30_000, &data);
        assert!(stage_of(&out).is_some() && extra(&out), "sent again on resume");
    }

    #[test]
    fn switching_to_an_avatar_without_a_pregnancy_leaves_its_belly_alone() {
        let mut d = Driver::new(Mapping::default());
        let mut data = AppData::new();
        data.settings.sync_scope = SyncScope::PerAvatar;
        crate::actions::start_pregnancy(&mut data, crate::actions::NewPregnancy { avatar_id: Some("avtr_a".into()), ..Default::default() }, T0);
        d.on_avatar_change("avtr_a", T0);
        assert!(stage_of(&d.tick(T0 + SYNC_SETTLE_MS, &data)).is_some(), "A gets its belly");
        d.on_avatar_change("avtr_b", T0 + 20_000);
        for i in 0..40 {
            assert!(stage_of(&d.tick(T0 + 20_000 + SYNC_SETTLE_MS + i * 500, &data)).is_none(), "B is never flattened");
        }
    }

    #[test]
    fn heartbeat_flips() {
        let mut d = Driver::new(Mapping::default());
        d.set_connected(true);
        let data = AppData::new();
        let beat = |out: Vec<Send>| out.into_iter().find(|s| s.key == "appHeartbeat").map(|s| s.value);
        let a = beat(d.tick(T0, &data)).unwrap();
        assert!(beat(d.tick(T0 + 200, &data)).is_none(), "not before the interval");
        let b = beat(d.tick(T0 + 1000, &data)).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn drift_from_avatar_clock_snaps_back() {
        let mut d = Driver::new(Mapping::default());
        d.set_connected(true);
        let data = data_pregnant();
        let now = T0 + 100 * DAY_MS;
        let s = stage_of(&d.tick(now, &data)).unwrap();
        assert!(stage_of(&d.tick(now + 200, &data)).is_none());
        d.on_param("Pregnancy/Stage", s + 0.01, true, false);
        assert!(stage_of(&d.tick(now + 400, &data)).is_some(), "snapped back right away");
    }

    #[test]
    fn not_pregnant_leaves_belly_alone() {
        let mut d = Driver::new(Mapping::default());
        d.set_connected(true);
        assert!(stage_of(&d.tick(T0, &AppData::new())).is_none());
    }

    #[test]
    fn ending_pregnancy_flattens_once() {
        let mut d = Driver::new(Mapping::default());
        d.set_connected(true);
        let mut data = data_pregnant();
        d.tick(T0 + 100 * DAY_MS, &data);
        data.pregnancies[0].ended_ms = Some(T0);
        assert_eq!(stage_of(&d.tick(T0 + 100 * DAY_MS + 1, &data)), Some(0.0));
        assert!(stage_of(&d.tick(T0 + 100 * DAY_MS + 2, &data)).is_none());
    }

    #[test]
    fn avatar_change_resends_profile_and_snaps() {
        let mut d = Driver::new(Mapping::default());
        d.set_connected(true);
        let data = data_pregnant();
        d.tick(T0, &data);
        d.on_avatar_change("avtr_2", T0);
        // Still loading: nothing yet, not even the heartbeat.
        assert!(d.tick(T0 + 10, &data).is_empty());
        assert!(stage_of(&d.tick(T0 + SYNC_SETTLE_MS, &data)).is_some());
        // Sent once more a little later, in case it was still loading.
        assert!(stage_of(&d.tick(T0 + SYNC_SETTLE_MS + 10, &data)).is_none());
        assert!(stage_of(&d.tick(T0 + RESEND_AFTER_MS, &data)).is_some());
    }

    #[test]
    fn birth_flattens_everything_even_after_a_resync() {
        let mut d = Driver::new(Mapping::default());
        d.set_connected(true);
        let mut data = data_pregnant();
        data.pregnancies[0].baby_count = 2;
        let t = T0 + 280 * DAY_MS;
        assert!(stage_of(&d.tick(t, &data)).unwrap() > 0.9);
        // The baby is born: the pregnancy ends, and the app resyncs.
        data.pregnancies.clear();
        d.resync();
        let out = d.tick(t + 10, &data);
        assert_eq!(stage_of(&out), Some(0.0));
        assert_eq!(out.iter().find(|s| s.key == "bellyExtra").map(|s| s.value), Some(ParamValue::Float(0.0)));
        // Then the belly is left alone (the in-game menu works again).
        assert_eq!(stage_of(&d.tick(t + 100_000, &data)), None);
    }

    #[test]
    fn avatar_parameters_showing_up_sends_again() {
        let mut d = Driver::new(Mapping::default());
        let data = data_pregnant();
        d.on_avatar_change("avtr_2", T0 - SYNC_SETTLE_MS);
        assert!(stage_of(&d.tick(T0, &data)).is_some());
        assert!(stage_of(&d.tick(T0 + 10, &data)).is_none());
        d.set_avatar_params(vec![("Pregnancy/Stage".into(), Some(0.0))], 0);
        assert!(stage_of(&d.tick(T0 + 20, &data)).is_some());
    }

    #[test]
    fn per_avatar_scope_uses_profile() {
        let mut d = Driver::new(Mapping::default());
        let mut data = data_pregnant();
        data.settings.sync_scope = SyncScope::PerAvatar;
        d.on_avatar_change("avtr_x", T0 - SYNC_SETTLE_MS);
        // No pregnancy linked to this avatar: belly left alone.
        assert!(stage_of(&d.tick(T0, &data)).is_none());
        data.set_current(Some("avtr_x"), Some("p".into()));
        d.resync();
        assert!(stage_of(&d.tick(T0 + 1, &data)).is_some());
    }

    #[test]
    fn optional_params_skipped_when_avatar_lacks_them() {
        let mut d = Driver::new(Mapping::default());
        d.on_avatar_change("avtr_1", T0 - SYNC_SETTLE_MS);
        d.set_avatar_params(vec![("Pregnancy/Stage".into(), Some(0.0))], 0);
        let out = d.tick(T0, &data_pregnant());
        let n = names(&out);
        assert!(n.contains(&"stage"));
        assert!(!n.contains(&"babyCount") && !n.contains(&"waiting") && !n.contains(&"appActive"));
    }

    #[test]
    fn aliases_are_used() {
        let mut d = Driver::new(Mapping::default());
        d.on_avatar_change("avtr_1", T0 - SYNC_SETTLE_MS);
        d.set_avatar_params(vec![("PregSys_Pregnancy_Stage".into(), Some(0.0))], 0);
        let out = d.tick(T0, &data_pregnant());
        assert_eq!(out.iter().find(|s| s.key == "stage").unwrap().name, "PregSys_Pregnancy_Stage");
    }

    #[test]
    fn twins_and_overdue_make_the_belly_bigger() {
        let extra = |out: &[Send]| out.iter().find(|s| s.key == "bellyExtra").map(|s| s.value);
        let mut d = Driver::new(Mapping::default());
        d.set_connected(true);
        let mut data = data_pregnant();
        assert_eq!(extra(&d.tick(T0 + 140 * DAY_MS, &data)), Some(ParamValue::Float(0.0)));
        data.pregnancies[0].baby_count = 2;
        assert_eq!(extra(&d.tick(T0 + 141 * DAY_MS, &data)), Some(ParamValue::Float(0.35)));
        // Long past the due date: full extra size.
        assert_eq!(extra(&d.tick(T0 + 400 * DAY_MS, &data)), Some(ParamValue::Float(1.0)));
        data.settings.bigger_for_multiples = false;
        data.settings.grow_when_overdue = false;
        assert_eq!(extra(&d.tick(T0 + 401 * DAY_MS, &data)), Some(ParamValue::Float(0.0)));
    }

    #[test]
    fn sync_values_follow_you_after_the_avatar_loads() {
        let mut d = Driver::new(Mapping::default());
        let mut data = AppData::new();
        data.sync_values.insert("Pregnancy/HideBelly".into(), 1.0);
        let now = crate::now_ms();
        d.on_avatar_change("avtr_new", now);
        // The new avatar reports its own default: ignored while it loads.
        assert_eq!(d.on_sync_param("Pregnancy/HideBelly", 0.0, now), SyncReport::Ignored);
        let sync = |out: &[Send]| out.iter().find(|s| s.name == "Pregnancy/HideBelly").map(|s| s.value);
        assert_eq!(sync(&d.tick(now, &data)), None, "not yet");
        assert_eq!(sync(&d.tick(now + SYNC_SETTLE_MS, &data)), Some(ParamValue::Bool(true)));
        assert_eq!(sync(&d.tick(now + SYNC_SETTLE_MS + 10, &data)), None, "sent once");
        // Still loading at 6 s: its default isn't your choice.
        assert_eq!(d.on_sync_param("Pregnancy/HideBelly", 0.0, now + 6_000), SyncReport::Ignored);
        // Loads at 6.5 s; its defaults may still come right after.
        assert!(d.set_avatar_params(vec![("Pregnancy/HideBelly".into(), Some(0.0))], now + 6_500));
        assert_eq!(d.on_sync_param("Pregnancy/HideBelly", 0.0, now + 7_000), SyncReport::Ignored);
        // Changes once it has loaded are held a moment, then remembered.
        let after = now + 6_500 + SYNC_IGNORE_AFTER_LOAD_MS;
        assert_eq!(d.on_sync_param("Pregnancy/HideBelly", 0.0, after), SyncReport::Held);
        assert!(d.take_sync_saves(after + 100, SYNC_HOLD_MS).is_empty(), "held");
        assert_eq!(d.take_sync_saves(after + SYNC_HOLD_MS, SYNC_HOLD_MS), vec![("Pregnancy/HideBelly".to_string(), 0.0)]);
        assert_eq!(d.on_sync_param("Not/Synced", 1.0, after), SyncReport::NotSync);
    }

    #[test]
    fn values_reported_just_before_a_switch_are_the_next_avatars() {
        let mut d = Driver::new(Mapping::default());
        d.on_avatar_change("avtr_a", 0);
        d.set_avatar_params(vec![("Pregnancy/HideBelly".into(), Some(0.0))], 2_000);
        // You unhide on A: remembered.
        assert_eq!(d.on_sync_param("Pregnancy/HideBelly", 0.0, 10_000), SyncReport::Held);
        assert_eq!(d.take_sync_saves(12_000, SYNC_HOLD_MS), vec![("Pregnancy/HideBelly".to_string(), 0.0)]);
        // Switching to B: VRChat reports B's own value, then says the avatar changed.
        assert_eq!(d.on_sync_param("Pregnancy/HideBelly", 1.0, 20_000), SyncReport::Held);
        d.on_avatar_change("avtr_b", 20_300);
        assert!(d.take_sync_saves(30_000, SYNC_HOLD_MS).is_empty(), "B's value never replaces your choice");
        assert_eq!(d.take_sync_dropped(), vec!["Pregnancy/HideBelly".to_string()]);
    }

    #[test]
    fn a_switch_in_the_log_guards_however_long_loading_takes() {
        let mut d = Driver::new(Mapping::default());
        d.on_avatar_change("avtr_a", 0);
        d.set_avatar_params(vec![("Pregnancy/HideBelly".into(), Some(0.0))], 2_000);
        // You unhide at 10 s, then switch at 11 s (read from the log at 11.8 s).
        assert_eq!(d.on_sync_param("Pregnancy/HideBelly", 0.0, 10_000), SyncReport::Held);
        d.on_switch_start(11_000, 11_800);
        assert_eq!(d.take_sync_saves(11_800, SYNC_HOLD_MS), vec![("Pregnancy/HideBelly".to_string(), 0.0)], "yours: saved right away");
        // B takes 40 s to download, then reports its own value well before the change.
        assert_eq!(d.on_sync_param("Pregnancy/HideBelly", 1.0, 25_000), SyncReport::Ignored);
        d.on_avatar_change("avtr_b", 51_000);
        assert_eq!(d.on_sync_param("Pregnancy/HideBelly", 1.0, 51_000), SyncReport::Ignored);
        assert!(d.take_sync_saves(60_000, SYNC_HOLD_MS).is_empty());
    }

    #[test]
    fn a_fast_switch_read_late_from_the_log() {
        let mut d = Driver::new(Mapping::default());
        d.on_avatar_change("avtr_a", 0);
        d.set_avatar_params(vec![("Pregnancy/HideBelly".into(), Some(0.0))], 2_000);
        // Cached avatar: switch logged at 16 s, B's value at 16.5 s, change at 16.5 s, log read at 17 s.
        assert_eq!(d.on_sync_param("Pregnancy/HideBelly", 1.0, 16_500), SyncReport::Held);
        d.on_avatar_change("avtr_b", 16_500);
        d.on_switch_start(16_000, 17_000);
        assert!(!d.loading() || d.sync_ignored(17_000), "the late log line changes nothing");
        assert!(d.take_sync_saves(20_000, SYNC_HOLD_MS).is_empty());
    }

    #[test]
    fn the_old_avatars_list_doesnt_count_as_loaded() {
        let mut d = Driver::new(Mapping::default());
        let list = || vec![("Pregnancy/HideBelly".to_string(), Some(0.0))];
        d.on_avatar_change("avtr_a", 0);
        assert!(d.set_avatar_params(list(), 4_000));
        d.on_avatar_change("avtr_b", 20_000);
        assert!(!d.set_avatar_params(list(), 21_500), "same list right away: still the old avatar");
        assert!(d.loading() && d.sync_ignored(30_000));
        assert!(d.set_avatar_params(list(), 26_000), "same list later: an avatar with the same parameters");
        assert!(!d.sync_ignored(26_000 + SYNC_IGNORE_AFTER_LOAD_MS));
    }

    #[test]
    fn never_listed_stops_ignoring_after_a_while() {
        let mut d = Driver::new(Mapping::default());
        d.on_avatar_change("avtr_a", 0);
        assert!(d.sync_ignored(LOAD_CAP_MS - 1));
        assert!(!d.sync_ignored(LOAD_CAP_MS));
    }

    #[test]
    fn slow_avatar_loads_keep_your_sync_values() {
        let mut d = Driver::new(Mapping::default());
        let mut data = AppData::new();
        data.sync_values.insert("Pregnancy/HideBelly".into(), 1.0);
        let now = crate::now_ms();
        d.on_avatar_change("avtr_big", now);
        d.tick(now + SYNC_SETTLE_MS, &data);
        // Loads at 15 s and reports its defaults right after.
        d.set_avatar_params(vec![("Pregnancy/HideBelly".into(), Some(0.0))], now + 15_000);
        assert_eq!(d.on_sync_param("Pregnancy/HideBelly", 0.0, now + 16_000), SyncReport::Ignored);
        let sync = |out: &[Send]| out.iter().find(|s| s.name == "Pregnancy/HideBelly").map(|s| s.value);
        assert_eq!(sync(&d.tick(now + 15_200, &data)), Some(ParamValue::Bool(true)), "sent when it loads");
        assert_eq!(sync(&d.tick(now + 17_000, &data)), Some(ParamValue::Bool(true)), "and once more after"); 
    }

    #[test]
    fn per_avatar_names_win() {
        let mut d = Driver::new(Mapping::default());
        let mut data = data_pregnant();
        crate::actions::touch_avatar(&mut data, "avtr_x", 0);
        data.avatars.get_mut("avtr_x").unwrap().param_names.insert("stage".into(), "Belly/Size".into());
        d.on_avatar_change("avtr_x", T0 - SYNC_SETTLE_MS);
        let out = d.tick(T0, &data);
        assert_eq!(out.iter().find(|s| s.key == "stage").unwrap().name, "Belly/Size");
    }

    #[test]
    fn baby_count_waits_for_week_12() {
        let mut d = Driver::new(Mapping::default());
        d.set_connected(true);
        let mut data = data_pregnant();
        data.pregnancies[0].baby_count = 2;
        let n = |out: &[Send]| out.iter().find(|s| s.key == "babyCount").map(|s| s.value);
        assert_eq!(n(&d.tick(T0 + 7 * crate::timeline::DAY_MS, &data)), Some(ParamValue::Int(0)));
        assert_eq!(n(&d.tick(T0 + 90 * crate::timeline::DAY_MS, &data)), Some(ParamValue::Int(2)));
    }

    #[test]
    fn kicks_start_at_week_16_and_reset_to_zero() {
        let mut d = Driver::new(Mapping::default());
        d.set_connected(true);
        let data = data_pregnant(); // realtime: week = days / 7
        let kick = |out: &[Send]| out.iter().find(|s| s.key == "fetalMovement").map(|s| s.value);
        // Week 10: never kicks.
        for i in 0..50 {
            assert_eq!(kick(&d.tick(T0 + 70 * DAY_MS + i * 60_000, &data)), None);
        }
        // Week 30: within an hour a kick comes, then goes back to 0.
        let start = T0 + 210 * DAY_MS;
        let mut sent = None;
        for i in 0..120 {
            if let Some(v) = kick(&d.tick(start + i * 30_000, &data)) {
                sent = Some((i, v));
                break;
            }
        }
        let (i, v) = sent.expect("a kick within an hour");
        assert_ne!(v, ParamValue::Int(0));
        assert_eq!(d.take_kicks().len(), 1);
        let back = (1..10).find_map(|j| kick(&d.tick(start + i * 30_000 + j * 1000, &data)));
        assert_eq!(back, Some(ParamValue::Int(0)));
    }

    #[test]
    fn kicks_off_or_paused_means_none() {
        let mut d = Driver::new(Mapping::default());
        d.set_connected(true);
        let mut data = data_pregnant();
        data.settings.kicks = false;
        let start = T0 + 210 * DAY_MS;
        assert!((0..200).all(|i| d.tick(start + i * 60_000, &data).iter().all(|s| s.key != "fetalMovement")));
    }

    #[test]
    fn debug_kick_is_sent_right_away() {
        let mut d = Driver::new(Mapping::default());
        d.set_connected(true);
        d.kick_now(KickStrength::Extreme);
        let out = d.tick(T0, &AppData::new());
        assert_eq!(out.iter().find(|s| s.key == "fetalMovement").map(|s| s.value), Some(ParamValue::Int(3)));
    }

    #[test]
    fn grace_period_holds_belly_flat() {
        let mut d = Driver::new(Mapping::default());
        d.set_connected(true);
        let mut data = AppData::new();
        data.waiting.push(crate::model::PendingConception::default());
        assert_eq!(stage_of(&d.tick(T0, &data)), Some(0.0));
        assert!(stage_of(&d.tick(T0 + 10, &data)).is_none());
        // The avatar's own system bumps the belly: it's snapped back to flat.
        d.on_param("Pregnancy/Stage", 0.01, false, true);
        assert_eq!(stage_of(&d.tick(T0 + 20, &data)), Some(0.0));
    }

    #[test]
    fn pulses_go_true_then_false() {
        let mut m = Mapping::default();
        let def = ParamDef { name: "Test/Pulse".into(), kind: ParamType::Bool, synced: false, required: false, optional: true, aliases: vec![] };
        m.parameters.insert("testPulse".into(), def);
        let mut d = Driver::new(m);
        d.set_connected(true);
        let data = AppData::new();
        d.tick(T0, &data);
        d.pulse("testPulse", T0, 1000);
        let v = |out: Vec<Send>| out.into_iter().find(|s| s.key == "testPulse").map(|s| s.value);
        assert_eq!(v(d.tick(T0 + 10, &data)), Some(ParamValue::Bool(true)));
        assert_eq!(v(d.tick(T0 + 500, &data)), None);
        assert_eq!(v(d.tick(T0 + 1100, &data)), Some(ParamValue::Bool(false)));
    }

    #[test]
    fn handoff_sends_final_stage_then_releases() {
        let mut d = Driver::new(Mapping::default());
        d.set_connected(true);
        let out = d.handoff(T0 + 140 * DAY_MS, &data_pregnant());
        assert_eq!(names(&out), vec!["stage", "appActive"]);
        assert_eq!(out[1].value, ParamValue::Bool(false));
    }
}
