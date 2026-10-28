//! VRChat chatbox messages for app events, plus an optional always-on
//! status line. Each event has its own text (with {placeholders}) and can
//! be turned off on its own.

use crate::fertility::BABY_REVEAL_WEEK;
use crate::model::{AppData, Pregnancy};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

/// VRChat cuts chatbox messages at this many characters.
pub const MAX_CHARS: usize = 144;
/// A pregnancy counts as just started (not just switched to) for this long.
const NEW_MS: i64 = 60_000;
/// Time between chatbox messages (VRChat ignores them when sent faster).
pub const GAP_MS: i64 = 2_000;
/// The status line is sent again this often, so VRChat keeps showing it.
const KEEP_MS: i64 = 4_000;
const DEFAULT_STATUS: &str = "🤰 Week {week} of 40";
/// Shown for the partner when nobody is chosen yet.
const UNKNOWN: &str = "Unknown";

/// Debug-mode chatbox messages (Debug panel), for testing in VRChat.
pub const DEBUG_EVENTS: [&str; 7] = ["avatarChange", "avatarLoaded", "sync", "menuButtons", "bellySnap", "heard", "kicks"];

/// Event keys, in the order the settings show them.
pub const EVENTS: [&str; 10] = ["started", "paused", "resumed", "stopped", "labor", "waterBroke", "born", "woohoo", "notPregnant", "achievement"];

fn default_text(event: &str) -> &'static str {
    match event {
        "started" => "🤰 A little one is on the way!",
        "paused" => "⏸️ Pregnancy paused (week {week})",
        "resumed" => "▶️ Pregnancy resumed (week {week})",
        "stopped" => "Pregnancy stopped",
        "labor" => "🤰 Labor has started!",
        "waterBroke" => "💦 Water broke!",
        "born" => "🍼 The baby is here!",
        "woohoo" => "💕 WooHoo!",
        "notPregnant" => "Not pregnant this time",
        "achievement" => "🏆 {achievement}",
        _ => "",
    }
}

/// Events whose message is off until turned on (labor can be private).
fn on_by_default(event: &str) -> bool {
    !matches!(event, "labor" | "waterBroke")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ChatMessage {
    pub on: bool,
    pub text: String,
}

impl Default for ChatMessage {
    fn default() -> Self {
        Self { on: true, text: String::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ChatboxSettings {
    pub enabled: bool,
    /// Play VRChat's chatbox sound.
    pub sound: bool,
    /// Seconds an event message stays up before it's cleared (or the
    /// status comes back). 0 = leave it to VRChat.
    pub show_seconds: u32,
    /// Always show the pregnancy status while pregnant.
    pub status: bool,
    pub status_text: String,
    /// Add the partner's name to the status.
    pub status_partner: bool,
    /// Event key -> message. Missing events use the default text.
    pub messages: BTreeMap<String, ChatMessage>,
    /// Debug-mode messages that are on (see [`DEBUG_EVENTS`]). They show
    /// even with the chatbox off, but only while debug mode is on.
    pub debug: BTreeMap<String, bool>,
}

impl Default for ChatboxSettings {
    fn default() -> Self {
        let messages = EVENTS.iter().map(|e| (e.to_string(), ChatMessage { on: on_by_default(e), text: default_text(e).into() })).collect();
        Self {
            enabled: false,
            sound: false,
            show_seconds: 8,
            status: false,
            status_text: DEFAULT_STATUS.into(),
            status_partner: true,
            messages,
            debug: BTreeMap::new(),
        }
    }
}

/// Values for the {placeholders}.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Vars {
    pub name: String,
    pub week: Option<u32>,
    pub babies: Option<u32>,
    pub partner: Option<String>,
    pub achievement: String,
}

impl Vars {
    pub fn of(p: &Pregnancy, now_ms: i64) -> Self {
        let week = p.timeline.week(now_ms);
        Self {
            name: p.name.clone(),
            week: Some(week.floor() as u32),
            // Twins and more stay a surprise until they'd show on a scan.
            babies: (week >= BABY_REVEAL_WEEK).then_some(p.baby_count),
            partner: p.partner.name.clone(),
            achievement: String::new(),
        }
    }

    fn fill(&self, text: &str) -> String {
        let out = text
            .replace("{name}", &self.name)
            .replace("{week}", &self.week.map(|w| w.to_string()).unwrap_or_default())
            .replace("{babies}", &self.babies.map(|b| b.to_string()).unwrap_or_else(|| "?".into()))
            .replace("{partner}", self.partner.as_deref().unwrap_or(UNKNOWN))
            .replace("{achievement}", &self.achievement);
        out.trim().chars().take(MAX_CHARS).collect()
    }
}

impl ChatboxSettings {
    /// The chatbox text for an event, or None if it's off.
    pub fn message(&self, event: &str, vars: &Vars) -> Option<String> {
        if !self.enabled {
            return None;
        }
        let m = self.messages.get(event);
        if !m.map_or(on_by_default(event), |m| m.on) {
            return None;
        }
        let text = m.map(|m| m.text.as_str()).filter(|t| !t.trim().is_empty()).unwrap_or(default_text(event));
        let out = vars.fill(text);
        (!out.is_empty()).then_some(out)
    }

    /// Whether a debug message is on (debug mode must be on too).
    pub fn debug_on(&self, key: &str) -> bool {
        self.debug.get(key).copied().unwrap_or(false)
    }

    /// The always-on status line for the current pregnancy, if it's on.
    pub fn status_line(&self, data: &AppData, avatar_id: Option<&str>, now_ms: i64) -> Option<String> {
        if !self.enabled || !self.status {
            return None;
        }
        let p = data.current_pregnancy(avatar_id)?;
        let vars = Vars::of(p, now_ms);
        let text = if self.status_text.trim().is_empty() { DEFAULT_STATUS } else { &self.status_text };
        let mut out = vars.fill(text);
        if self.status_partner && !text.contains("{partner}") {
            out = format!("{out} · 💕 {}", vars.partner.as_deref().unwrap_or(UNKNOWN));
        }
        if p.timeline.is_paused() {
            out = format!("{out} ⏸️");
        }
        let out: String = out.chars().take(MAX_CHARS).collect();
        (!out.is_empty()).then_some(out)
    }
}

/// Notices pause, resume, start and stop however they happen (the avatar's
/// menu or the app), by comparing the current pregnancy tick to tick.
#[derive(Debug, Default)]
pub struct Watch {
    /// (avatar, current pregnancy, paused)
    last: Option<(Option<String>, Option<String>, bool)>,
}

impl Watch {
    pub fn check(&mut self, data: &AppData, avatar_id: Option<&str>, now_ms: i64) -> Vec<(&'static str, Vars)> {
        let cur = data.current_pregnancy(avatar_id);
        let snap = (avatar_id.map(str::to_string), cur.map(|p| p.id.clone()), cur.is_some_and(|p| p.timeline.is_paused()));
        let Some(last) = self.last.replace(snap.clone()) else { return vec![] };
        // Switching avatars changes the current pregnancy without anything happening.
        if last.0 != snap.0 {
            return vec![];
        }
        let vars = |id: &str| data.pregnancies.iter().find(|p| p.id == id).map_or_else(Vars::default, |p| Vars::of(p, now_ms));
        match (&last.1, &snap.1) {
            (Some(a), Some(b)) if a == b => match (last.2, snap.2) {
                (false, true) => vec![("paused", vars(b))],
                (true, false) => vec![("resumed", vars(b))],
                _ => vec![],
            },
            (_, Some(b)) => {
                let new = cur.is_some_and(|p| now_ms - p.created_ms < NEW_MS);
                if new { vec![("started", vars(b))] } else { vec![] }
            }
            (Some(a), None) => match data.pregnancies.iter().find(|p| p.id == *a) {
                Some(p) if p.born => vec![("born", vars(a))],
                Some(p) if p.ended_ms.is_some() => vec![("stopped", vars(a))],
                _ => vec![], // deleted
            },
            (None, None) => vec![],
        }
    }
}

/// What the chatbox shows: queued event messages first (each for
/// `show_seconds`), then the status line kept alive, or nothing (cleared).
#[derive(Debug, Default)]
pub struct Screen {
    queue: VecDeque<String>,
    /// An event message is up until then.
    event_until: Option<i64>,
    last_sent_ms: Option<i64>,
    /// What's on screen ("" = cleared).
    shown: String,
}

impl Screen {
    pub fn push(&mut self, text: String) {
        if self.queue.len() < 5 {
            self.queue.push_back(text);
        }
    }

    /// VRChat closed: start over.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The text to send now, if any ("" clears the chatbox).
    pub fn tick(&mut self, now_ms: i64, show_seconds: u32, status: Option<String>) -> Option<String> {
        if self.last_sent_ms.is_some_and(|t| now_ms - t < GAP_MS) {
            return None;
        }
        let out = if let Some(m) = self.queue.pop_front() {
            self.event_until = Some(match show_seconds {
                // Left to VRChat, unless the status needs the chatbox back.
                0 if status.is_none() => i64::MAX,
                0 => now_ms + GAP_MS,
                s => now_ms + s as i64 * 1000,
            });
            m
        } else if self.event_until.is_some_and(|t| now_ms < t) {
            return None;
        } else {
            self.event_until = None;
            match status {
                Some(s) if s != self.shown || self.last_sent_ms.is_none_or(|t| now_ms - t >= KEEP_MS) => s,
                Some(_) => return None,
                None if !self.shown.is_empty() => String::new(),
                None => return None,
            }
        };
        self.last_sent_ms = Some(now_ms);
        self.shown = out.clone();
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{self, NewPregnancy};

    fn on() -> ChatboxSettings {
        ChatboxSettings { enabled: true, ..Default::default() }
    }

    #[test]
    fn messages_fill_placeholders_and_can_be_off() {
        let mut s = on();
        let v = Vars { week: Some(12), ..Default::default() };
        assert_eq!(s.message("paused", &v).unwrap(), "⏸️ Pregnancy paused (week 12)");
        s.messages.get_mut("paused").unwrap().on = false;
        assert_eq!(s.message("paused", &v), None);
        s.messages.get_mut("stopped").unwrap().text = "  ".into();
        assert_eq!(s.message("stopped", &v).unwrap(), "Pregnancy stopped", "blank text uses the default");
        s.messages.get_mut("born").unwrap().text = "x".repeat(200);
        assert_eq!(s.message("born", &v).unwrap().chars().count(), MAX_CHARS);
        assert_eq!(ChatboxSettings::default().message("born", &v), None, "chatbox off");
    }

    #[test]
    fn watch_sees_start_pause_resume_and_stop() {
        let mut d = AppData::new();
        let mut w = Watch::default();
        let a = Some("avtr_a");
        let ev = |w: &mut Watch, d: &AppData, t| w.check(d, a, t).into_iter().map(|(e, _)| e).collect::<Vec<_>>();
        assert!(ev(&mut w, &d, 0).is_empty());
        let id = actions::start_pregnancy(&mut d, NewPregnancy { avatar_id: a.map(str::to_string), ..Default::default() }, 1000);
        assert_eq!(ev(&mut w, &d, 1200), ["started"]);
        d.pregnancy_mut(&id).unwrap().timeline.pause(2000);
        assert_eq!(ev(&mut w, &d, 2000), ["paused"]);
        assert!(ev(&mut w, &d, 2200).is_empty(), "once");
        d.pregnancy_mut(&id).unwrap().timeline.resume(3000);
        assert_eq!(ev(&mut w, &d, 3000), ["resumed"]);
        actions::end_pregnancy(&mut d, &id, 4000);
        assert_eq!(ev(&mut w, &d, 4000), ["stopped"]);
    }

    #[test]
    fn events_clear_after_a_while_and_the_status_stays() {
        let mut sc = Screen::default();
        sc.push("💕 WooHoo!".into());
        assert_eq!(sc.tick(0, 8, None).as_deref(), Some("💕 WooHoo!"));
        assert_eq!(sc.tick(3_000, 8, None), None, "still showing");
        assert_eq!(sc.tick(8_000, 8, None).as_deref(), Some(""), "cleared");
        assert_eq!(sc.tick(12_000, 8, None), None, "cleared once");
        let st = || Some("🤰 Week 3 of 40".to_string());
        assert_eq!(sc.tick(14_000, 8, st()).as_deref(), Some("🤰 Week 3 of 40"));
        assert_eq!(sc.tick(16_000, 8, st()), None);
        assert_eq!(sc.tick(18_000, 8, st()).as_deref(), Some("🤰 Week 3 of 40"), "kept alive");
        sc.push("⏸️ Paused".into());
        assert_eq!(sc.tick(19_000, 8, st()), None, "gap between messages");
        assert_eq!(sc.tick(20_000, 8, st()).as_deref(), Some("⏸️ Paused"));
        assert_eq!(sc.tick(28_000, 8, st()).as_deref(), Some("🤰 Week 3 of 40"), "status comes back");
    }

    #[test]
    fn status_line_with_partner_and_pause() {
        let mut d = AppData::new();
        let id = actions::start_pregnancy(&mut d, NewPregnancy { avatar_id: Some("a".into()), ..Default::default() }, 0);
        let mut s = on();
        assert_eq!(s.status_line(&d, Some("a"), 0), None, "status off");
        s.status = true;
        assert_eq!(s.status_line(&d, Some("a"), 0).unwrap(), "🤰 Week 0 of 40 · 💕 Unknown");
        d.pregnancy_mut(&id).unwrap().partner.name = Some("Alex".into());
        assert_eq!(s.status_line(&d, Some("a"), 0).unwrap(), "🤰 Week 0 of 40 · 💕 Alex");
        s.status_partner = false;
        d.pregnancy_mut(&id).unwrap().timeline.pause(0);
        assert_eq!(s.status_line(&d, Some("a"), 0).unwrap(), "🤰 Week 0 of 40 ⏸️");
        actions::end_pregnancy(&mut d, &id, 10);
        assert_eq!(s.status_line(&d, Some("a"), 20), None, "not pregnant");
    }

    #[test]
    fn switching_avatars_says_nothing() {
        let mut d = AppData::new();
        let mut w = Watch::default();
        actions::start_pregnancy(&mut d, NewPregnancy { avatar_id: Some("avtr_a".into()), ..Default::default() }, 0);
        w.check(&d, Some("avtr_b"), 100);
        assert!(w.check(&d, Some("avtr_a"), 200).is_empty());
    }
}
