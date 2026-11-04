//! Debug: pretend partners ("Test Buddy" and friends) to test with, so testing
//! never mixes into your real partner stats. Each is on or off by itself.
//! Everything they touch is found by their user ID and removed again (also
//! when debug mode is turned off).
//! Mirrored in `src/lib/testbuddy.ts` (the names, IDs and pictures).

use crate::actions::{self, NewPregnancy};
use crate::logparse::LogEvent;
use crate::model::{AppData, FunEvent, Partner, PartnerStatus, PregnancySource};
use crate::timeline::{DurationMode, DAY_MS};

pub struct Buddy {
    pub name: &'static str,
    /// A made-up user ID ("7e57" = test). Never a real VRChat user.
    pub user_id: &'static str,
    pub avatar: &'static str,
}

pub const BUDDIES: [Buddy; 3] = [
    Buddy { name: "Test Buddy", user_id: "usr_7e57b0d1-7e57-4e57-8e57-7e57b0d17e57", avatar: "Test Bunny" },
    Buddy { name: "Test Fox", user_id: "usr_7e57b0d2-7e57-4e57-8e57-7e57b0d27e57", avatar: "Test Kit" },
    Buddy { name: "Test Wolf", user_id: "usr_7e57b0d3-7e57-4e57-8e57-7e57b0d37e57", avatar: "Test Pup" },
];

pub fn is_test(user_id: Option<&str>) -> bool {
    BUDDIES.iter().any(|b| user_id == Some(b.user_id))
}

/// The test buddies in the instance right now, by user ID.
pub fn here(data: &AppData) -> Vec<String> {
    data.instance.others().iter().filter_map(|p| p.user_id.clone().filter(|id| is_test(Some(id)))).collect()
}

/// Saves from when there was one Test Buddy: `test_buddy_since` meant them.
pub fn upgrade(data: &mut AppData) {
    if data.test_buddy_since.is_some() && data.test_buddies.is_empty() {
        data.test_buddies.push(BUDDIES[0].user_id.to_string());
    }
}

/// Adds a buddy: in your instance (long enough to count as a settled 1-on-1,
/// if they're the only one), with a few past WooHoos and one past baby, so
/// the Partners tab has something to show.
pub fn add(data: &mut AppData, index: usize, now_ms: i64) {
    let Some(b) = BUDDIES.get(index) else { return };
    if data.test_buddies.iter().any(|id| id == b.user_id) {
        return;
    }
    data.test_buddies.push(b.user_id.to_string());
    data.test_buddy_since.get_or_insert(now_ms);
    // In the instance since before the settle time.
    let at = now_ms - (data.settings.settle_minutes as i64 + 1) * 60_000;
    let inst = &mut data.instance;
    if inst.joined_ms.is_none_or(|j| j > at) {
        inst.joined_ms = Some(at);
    }
    if inst.world_name.is_none() {
        inst.world_name = Some("Debug world".into());
    }
    inst.apply(&LogEvent::PlayerJoined { name: b.name.into(), user_id: Some(b.user_id.into()) }, at);

    // Past WooHoos (WooHoos are found by their time, so never the same one).
    for days in [9, 6, 2] {
        data.fun_log.push(FunEvent {
            time_ms: now_ms - days * DAY_MS - index as i64 * 60_000,
            partner_name: Some(b.name.into()),
            partner_user_id: Some(b.user_id.into()),
            avatar_id: data.current_avatar_id.clone(),
            world_name: Some("Debug world".into()),
            ..Default::default()
        });
    }
    data.fun_log.sort_by_key(|f| f.time_ms);

    // A past baby together (a one-week pregnancy that ended a month ago).
    let partner = Partner { status: PartnerStatus::Confirmed, name: Some(b.name.into()), user_id: Some(b.user_id.into()), avatar_name: Some(b.avatar.into()), ..Default::default() };
    let current = data.active_pregnancy_id.clone();
    let id = actions::start_pregnancy(
        data,
        NewPregnancy {
            name: Some("Test baby".into()),
            start_ms: Some(now_ms - 37 * DAY_MS),
            mode: Some(DurationMode::Custom),
            custom_ms: Some(7 * DAY_MS),
            baby_count: Some(1),
            source: PregnancySource::Conception,
            make_current: Some(false),
            partner: Some(partner),
            ..Default::default()
        },
        now_ms,
    );
    actions::give_birth(data, &id, now_ms - 30 * DAY_MS);
    // Never replaces the pregnancy you have.
    data.active_pregnancy_id = current;
}

/// Removes a buddy and everything that came from them: WooHoos (theirs, and
/// any with nobody named while they were here), pregnancies (with their
/// journal and keepsakes), grace periods, history, and achievements they
/// unlocked that you no longer have without them.
pub fn remove(data: &mut AppData, index: usize, now_ms: i64) {
    let Some(b) = BUDDIES.get(index) else { return };
    let mine = |id: Option<&str>| id == Some(b.user_id);
    let around = |list: &[crate::instance::Person]| list.iter().any(|c| mine(c.user_id.as_deref()));
    let ids: Vec<String> = data
        .pregnancies
        .iter()
        .filter(|p| mine(p.partner.user_id.as_deref()) || (p.partner.name.is_none() && around(&p.partner.candidates)))
        .map(|p| p.id.clone())
        .collect();
    for id in ids {
        actions::delete_pregnancy(data, &id);
    }
    let gone: Vec<i64> = data
        .fun_log
        .iter()
        .filter(|f| mine(f.partner_user_id.as_deref()) || (f.partner_name.is_none() && f.test_buddies_here.iter().any(|id| id == b.user_id)))
        .map(|f| f.time_ms)
        .collect();
    data.fun_log.retain(|f| !gone.contains(&f.time_ms));
    data.waiting.retain(|w| !mine(w.partner.user_id.as_deref()));
    for w in &mut data.waiting {
        // It only took with one of those, so it didn't take after all.
        if w.later.as_ref().is_some_and(|l| mine(l.partner.user_id.as_deref()) || gone.contains(&l.time_ms)) || (w.later.is_none() && gone.contains(&w.time_ms)) {
            w.later = None;
            w.positive = false;
        }
        w.fun_times.retain(|t| !gone.contains(t));
        // The card moves on to the first WooHoo left.
        if gone.contains(&w.time_ms) {
            w.time_ms = w.fun_times.first().copied().unwrap_or(w.time_ms);
        }
    }
    // Nothing left in the wait: it goes too.
    data.waiting.retain(|w| !(gone.contains(&w.time_ms) && w.fun_times.is_empty()));
    data.instance.present.retain(|p| !mine(p.user_id.as_deref()));
    actions::forget_person(data, b.name, Some(b.user_id));
    data.test_buddies.retain(|id| id != b.user_id);
    // Achievements unlocked while testing that are no longer earned.
    if let Some(since) = data.test_buddy_since {
        let gone: Vec<String> = data
            .achievements
            .iter()
            .filter(|(id, at)| **at >= since && !crate::achievements::earned(data, id, now_ms))
            .map(|(id, _)| id.clone())
            .collect();
        for id in gone {
            data.achievements.remove(&id);
            data.test_achievements.retain(|t| *t != id);
        }
    }
    if data.test_buddies.is_empty() {
        data.test_buddy_since = None;
    }
}

/// Removes every buddy (debug mode was turned off).
pub fn remove_all(data: &mut AppData, now_ms: i64) {
    for i in 0..BUDDIES.len() {
        remove(data, i, now_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000_000;

    #[test]
    fn added_then_removed_without_a_trace() {
        let mut d = AppData::new();
        d.fun_log.push(FunEvent { time_ms: NOW - DAY_MS, partner_name: Some("Fox".into()), ..Default::default() });
        d.achievements.insert("first_woohoo".into(), NOW - DAY_MS);
        let before = d.clone();

        add(&mut d, 0, NOW);
        add(&mut d, 0, NOW + 1);
        assert_eq!(d.fun_log.len(), 4, "added once");
        assert!(d.instance.others().iter().any(|p| is_test(p.user_id.as_deref())));
        let baby = d.pregnancies.iter().find(|p| is_test(p.partner.user_id.as_deref())).unwrap();
        assert!(baby.born && d.active_pregnancy_id.is_none());
        crate::achievements::check(&mut d, NOW + 10);
        assert!(d.achievements.contains_key("welcome_home"));

        // A real pregnancy started with Test Buddy while testing goes too.
        let p = Partner { name: Some(BUDDIES[0].name.into()), user_id: Some(BUDDIES[0].user_id.into()), ..Default::default() };
        actions::start_pregnancy(&mut d, NewPregnancy { partner: Some(p), ..Default::default() }, NOW + 20);

        remove(&mut d, 0, NOW + 30);
        assert_eq!(d.fun_log, before.fun_log);
        assert!(d.pregnancies.is_empty() && d.active_pregnancy_id.is_none());
        assert!(d.instance.present.is_empty() && d.instance.history.is_empty());
        assert_eq!(d.achievements, before.achievements, "only the real one is kept");
        assert!(d.test_buddy_since.is_none());
    }

    #[test]
    fn each_buddy_comes_and_goes_alone() {
        let mut d = AppData::new();
        let names = |d: &AppData| d.instance.others().iter().map(|p| p.name.clone()).collect::<Vec<_>>();
        add(&mut d, 0, NOW);
        add(&mut d, 1, NOW);
        add(&mut d, 2, NOW);
        assert_eq!(names(&d), ["Test Buddy", "Test Fox", "Test Wolf"]);
        assert_eq!(d.fun_log.len(), 9);
        let mut times: Vec<i64> = d.fun_log.iter().map(|f| f.time_ms).collect();
        times.dedup();
        assert_eq!(times.len(), 9, "each WooHoo has its own time");
        assert_eq!(actions::guess_partner_now(&d, NOW).name, None, "three here: no guess");

        remove(&mut d, 1, NOW + 1);
        assert_eq!(names(&d), ["Test Buddy", "Test Wolf"]);
        assert_eq!(d.fun_log.len(), 6);
        assert_eq!(d.pregnancies.len(), 2);
        assert!(d.test_buddy_since.is_some(), "the others are still on");

        remove(&mut d, 2, NOW + 2);
        assert_eq!(actions::guess_partner_now(&d, NOW + 2).name.as_deref(), Some("Test Buddy"), "just one: a 1-on-1");

        remove_all(&mut d, NOW + 3);
        assert!(d.fun_log.is_empty() && d.pregnancies.is_empty() && d.test_buddies.is_empty() && d.test_buddy_since.is_none());
    }

    #[test]
    fn a_wait_that_only_took_with_a_buddy_didnt_take() {
        let mut d = AppData::new();
        let later = crate::model::LaterTake { partner: Partner { user_id: Some(BUDDIES[1].user_id.into()), ..Default::default() }, ..Default::default() };
        d.waiting.push(crate::model::PendingConception { positive: true, later: Some(later), ..Default::default() });
        add(&mut d, 1, NOW);
        remove(&mut d, 1, NOW);
        assert!(!d.waiting[0].positive && d.waiting[0].later.is_none());
    }

    #[test]
    fn three_buddies_pick_at_random_on_the_third_woohoo() {
        let names = |d: &AppData| d.fun_log.iter().filter(|f| f.time_ms >= NOW).map(|f| f.partner_name.is_some()).collect::<Vec<_>>();
        let mut d = AppData::new();
        (0..3).for_each(|i| add(&mut d, i, NOW));
        (0..3).for_each(|i| {
            actions::record_fun(&mut d, NOW + i, 0.999);
        });
        assert_eq!(names(&d), [false, false, true]);
        // A WooHoo from before they arrived, in the same instance, counts too.
        let mut d = AppData::new();
        d.instance.joined_ms = Some(NOW - 60_000);
        actions::record_fun(&mut d, NOW - 30_000, 0.999);
        (0..3).for_each(|i| add(&mut d, i, NOW));
        (0..2).for_each(|i| {
            actions::record_fun(&mut d, NOW + i, 0.999);
        });
        assert_eq!(names(&d), [false, true]);
    }

    #[test]
    fn woohoos_with_nobody_named_go_with_the_buddies() {
        let rows = |d: &AppData| d.fun_log.iter().filter(|f| f.time_ms >= NOW).count();
        let mut d = AppData::new();
        (0..3).for_each(|i| add(&mut d, i, NOW));
        actions::record_fun(&mut d, NOW, 0.999);
        actions::record_fun(&mut d, NOW + 1, 0.999);
        assert_eq!((rows(&d), d.waiting.len()), (2, 1));
        // Resetting one clears them, and the wait they were all of.
        remove(&mut d, 1, NOW + 2);
        add(&mut d, 1, NOW + 3);
        assert_eq!((rows(&d), d.waiting.len()), (0, 0));

        // A real named WooHoo stays, and so does its wait.
        let mut d = AppData::new();
        (0..3).for_each(|i| add(&mut d, i, NOW));
        actions::record_fun(&mut d, NOW, 0.999);
        d.fun_log.iter_mut().find(|f| f.time_ms == NOW).unwrap().partner_name = Some("Real".into());
        actions::record_fun(&mut d, NOW + 1, 0.999);
        remove(&mut d, 1, NOW + 2);
        assert_eq!(d.fun_log.iter().filter(|f| f.time_ms >= NOW).map(|f| f.time_ms).collect::<Vec<_>>(), [NOW]);
        assert_eq!(d.waiting[0].fun_times, [NOW]);

        // A pregnancy from one of those goes too, even with nobody named.
        let mut d = AppData::new();
        add(&mut d, 0, NOW);
        add(&mut d, 1, NOW);
        actions::record_fun(&mut d, NOW, 0.0);
        let reveal = d.waiting[0].reveal_ms;
        actions::resolve_due(&mut d, reveal);
        assert!(d.current_pregnancy(None).is_some());
        remove(&mut d, 1, reveal + 1);
        assert!(d.current_pregnancy(None).is_none());
        assert_eq!(rows(&d), 0);
    }

    #[test]
    fn old_saves_keep_their_test_buddy() {
        let mut d = AppData::new();
        d.test_buddy_since = Some(NOW);
        upgrade(&mut d);
        assert_eq!(d.test_buddies, [BUDDIES[0].user_id]);
    }
}
