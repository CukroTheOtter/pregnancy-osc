//! In-game menu buttons (`Pregnancy/Control/*`): start, pause and stop a
//! pregnancy from the avatar's Settings > App Controls menu.

use crate::actions::{self, NewPregnancy};
use crate::model::{AppData, PregnancySource};
use std::collections::HashMap;

/// Stop has to be held this long, so a stray tap never ends a pregnancy.
pub const STOP_HOLD_MS: i64 = 2_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Start,
    Pause,
    Stop,
}

impl Control {
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "controlStart" => Some(Self::Start),
            "controlPause" => Some(Self::Pause),
            "controlStop" => Some(Self::Stop),
            _ => None,
        }
    }
}

/// Turns button values into presses: Start and Pause act when pressed, Stop
/// once it has been held for [`STOP_HOLD_MS`].
#[derive(Debug, Default)]
pub struct Controls {
    /// Button -> when it was pressed (None once it has acted).
    held: HashMap<&'static str, Option<i64>>,
}

impl Controls {
    pub fn on_param(&mut self, c: Control, value: f32, now_ms: i64) -> Option<Control> {
        let key = name(c);
        let down = value > 0.5;
        let was_down = self.held.contains_key(key);
        if !down {
            self.held.remove(key);
            return None;
        }
        if was_down {
            return None;
        }
        self.held.insert(key, Some(now_ms));
        (c != Control::Stop).then_some(c)
    }

    /// Stop fires here once it has been held long enough.
    pub fn tick(&mut self, now_ms: i64) -> Option<Control> {
        let since = self.held.get_mut(name(Control::Stop))?;
        if since.is_some_and(|t| now_ms - t >= STOP_HOLD_MS) {
            *since = None;
            return Some(Control::Stop);
        }
        None
    }
}

fn name(c: Control) -> &'static str {
    match c {
        Control::Start => "start",
        Control::Pause => "pause",
        Control::Stop => "stop",
    }
}

/// Does what the button asks for the avatar you're wearing. Returns a short
/// message when something changed.
pub fn apply(data: &mut AppData, avatar_id: Option<&str>, c: Control, now_ms: i64) -> Option<&'static str> {
    let current = data.current_pregnancy_id(avatar_id);
    match (c, current) {
        (Control::Start, None) if data.pending_for(avatar_id).is_none() => {
            let np = NewPregnancy { source: PregnancySource::Manual, avatar_id: avatar_id.map(str::to_string), ..Default::default() };
            actions::start_pregnancy(data, np, now_ms);
            Some("Pregnancy started from the menu")
        }
        (Control::Pause, Some(id)) => {
            let p = data.pregnancy_mut(&id)?;
            if p.timeline.paused_at_ms.is_some() {
                p.timeline.resume(now_ms);
                Some("Pregnancy resumed")
            } else {
                p.timeline.pause(now_ms);
                Some("Pregnancy paused")
            }
        }
        (Control::Stop, Some(id)) => {
            actions::end_pregnancy(data, &id, now_ms);
            Some("Pregnancy stopped from the menu")
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_and_pause_act_once_per_press() {
        let mut c = Controls::default();
        assert_eq!(c.on_param(Control::Pause, 1.0, 0), Some(Control::Pause));
        assert_eq!(c.on_param(Control::Pause, 1.0, 50), None, "still held");
        assert_eq!(c.on_param(Control::Pause, 0.0, 100), None);
        assert_eq!(c.on_param(Control::Pause, 1.0, 200), Some(Control::Pause));
    }

    #[test]
    fn stop_needs_a_hold() {
        let mut c = Controls::default();
        assert_eq!(c.on_param(Control::Stop, 1.0, 0), None);
        assert_eq!(c.tick(STOP_HOLD_MS - 1), None);
        assert_eq!(c.tick(STOP_HOLD_MS), Some(Control::Stop));
        assert_eq!(c.tick(STOP_HOLD_MS + 500), None, "only once per hold");
        // A short tap does nothing.
        c.on_param(Control::Stop, 0.0, 3_000);
        c.on_param(Control::Stop, 1.0, 4_000);
        c.on_param(Control::Stop, 0.0, 4_300);
        assert_eq!(c.tick(10_000), None);
    }

    #[test]
    fn buttons_start_pause_and_stop() {
        let mut d = AppData::new();
        assert_eq!(apply(&mut d, Some("avtr_a"), Control::Pause, 0), None, "nothing to pause");
        assert!(apply(&mut d, Some("avtr_a"), Control::Start, 0).is_some());
        assert_eq!(apply(&mut d, Some("avtr_a"), Control::Start, 1), None, "already pregnant");
        assert_eq!(apply(&mut d, Some("avtr_a"), Control::Pause, 10), Some("Pregnancy paused"));
        assert!(d.current_pregnancy(Some("avtr_a")).unwrap().timeline.paused_at_ms.is_some());
        assert_eq!(apply(&mut d, Some("avtr_a"), Control::Pause, 20), Some("Pregnancy resumed"));
        assert!(apply(&mut d, Some("avtr_a"), Control::Stop, 30).is_some());
        assert!(d.current_pregnancy(Some("avtr_a")).is_none());
    }
}
