//! Journal moments from how you really play: lots of kicks with your partner
//! around, kicks in lots of worlds, late-night kicks. Each is found from the
//! current pregnancy week's kicks and saved to the journal once per pregnancy
//! (kicks get trimmed, a saved moment stays). The UI draws their picture.

use crate::model::{AppData, JournalEntry, new_id};
use crate::timeline::TERM_WEEKS;

/// The journal entry kind for these (its title is the moment's id).
pub const KIND: &str = "moment";

/// Kicks with your partner in the instance, in one week.
pub const PARTNER_KICKS: usize = 5;
/// Different worlds kicks happened in, in one week.
pub const WORLDS: usize = 4;
/// Kicks between midnight and 5 am, in one week.
pub const NIGHT_KICKS: usize = 5;

/// The hour on this PC's clock (0 to 23) at a time.
pub fn local_hour(t_ms: i64) -> u32 {
    use chrono::{Local, TimeZone, Timelike};
    Local.timestamp_millis_opt(t_ms).single().map_or(12, |dt| dt.hour())
}

/// Checks the current week's kicks and saves any new moment. `local_hour`
/// turns a time into the hour on the user's clock (0 to 23).
pub fn check(data: &mut AppData, preg_id: &str, now_ms: i64, local_hour: impl Fn(i64) -> u32) -> Vec<&'static str> {
    let Some(p) = data.pregnancy(preg_id) else { return vec![] };
    let week = p.timeline.week(now_ms).floor();
    let week_start = p.timeline.time_at_progress(now_ms, week / TERM_WEEKS);
    let this_week: Vec<_> = p.kicks.iter().filter(|k| k.time_ms >= week_start && k.time_ms <= now_ms).collect();
    let mut worlds: Vec<&str> = this_week.iter().filter_map(|k| k.world.as_deref()).collect();
    worlds.sort_unstable();
    worlds.dedup();
    let world_count = worlds.len();
    let found = [
        ("partner-kicks", this_week.iter().filter(|k| k.partner_here).count() >= PARTNER_KICKS),
        ("well-travelled", world_count >= WORLDS),
        ("night-owl", this_week.iter().filter(|k| local_hour(k.time_ms) < 5).count() >= NIGHT_KICKS),
    ];
    let mut added = vec![];
    for (id, yes) in found {
        let had = data.journal.iter().any(|j| j.kind == KIND && j.title == id && j.pregnancy_id.as_deref() == Some(preg_id));
        if yes && !had {
            data.journal.push(JournalEntry {
                id: new_id("j"),
                time_ms: now_ms,
                pregnancy_id: Some(preg_id.to_string()),
                kind: KIND.into(),
                title: id.into(),
                text: if id == "well-travelled" { world_count.to_string() } else { String::new() },
                image: None,
            });
            added.push(id);
        }
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{NewPregnancy, start_pregnancy};
    use crate::model::KickEvent;

    const T: i64 = 1_700_000_000_000;
    const HOUR: i64 = 3_600_000;

    fn kick(t: i64, partner: bool, world: &str) -> KickEvent {
        KickEvent { time_ms: t, partner_here: partner, world: Some(world.into()), ..Default::default() }
    }

    #[test]
    fn saves_each_moment_once() {
        let mut d = AppData::new();
        let id = start_pregnancy(&mut d, NewPregnancy::default(), T);
        let now = T + 10 * HOUR;
        let p = d.pregnancy_mut(&id).unwrap();
        for i in 0..5 {
            p.kicks.push(kick(now - i * 60_000, true, ["A", "B", "C", "D", "D"][i as usize]));
        }
        let day = |_: i64| 14;
        assert_eq!(check(&mut d, &id, now, day), vec!["partner-kicks", "well-travelled"]);
        assert!(check(&mut d, &id, now, day).is_empty(), "only once");
        let travel = d.journal.iter().find(|j| j.title == "well-travelled").unwrap();
        assert_eq!(travel.text, "4");
        assert_eq!(check(&mut d, &id, now, |_| 3), vec!["night-owl"]);
    }

    #[test]
    fn needs_enough_this_week() {
        let mut d = AppData::new();
        let id = start_pregnancy(&mut d, NewPregnancy::default(), T);
        let now = T + 10 * HOUR;
        let p = d.pregnancy_mut(&id).unwrap();
        for i in 0..4 {
            p.kicks.push(kick(now - i * 60_000, true, "A"));
        }
        assert!(check(&mut d, &id, now, |_| 2).is_empty());
    }
}
