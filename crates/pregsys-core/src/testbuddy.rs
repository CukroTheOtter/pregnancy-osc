//! Debug: a pretend partner ("Test Buddy") to test with, so testing never
//! mixes into your real partner stats. Everything they touch is found by
//! their user ID and removed again (also when debug mode is turned off).
//! Mirrored in `src/lib/testbuddy.ts` (the ID and picture).

use crate::actions::{self, NewPregnancy};
use crate::logparse::LogEvent;
use crate::model::{AppData, FunEvent, Partner, PartnerStatus, PregnancySource};
use crate::timeline::{DurationMode, DAY_MS};

pub const NAME: &str = "Test Buddy";
/// A made-up user ID ("7e57" = test). Never a real VRChat user.
pub const USER_ID: &str = "usr_7e57b0d1-7e57-4e57-8e57-7e57b0d17e57";

pub fn is_test(user_id: Option<&str>) -> bool {
    user_id == Some(USER_ID)
}

/// Adds Test Buddy: in your instance (long enough to count as a settled
/// 1-on-1), with a few past WooHoos and one past baby, so the Partners tab
/// has something to show.
pub fn add(data: &mut AppData, now_ms: i64) {
    if data.test_buddy_since.is_some() {
        return;
    }
    data.test_buddy_since = Some(now_ms);
    // In the instance since before the settle time.
    let at = now_ms - (data.settings.settle_minutes as i64 + 1) * 60_000;
    let inst = &mut data.instance;
    if inst.joined_ms.is_none_or(|j| j > at) {
        inst.joined_ms = Some(at);
    }
    if inst.world_name.is_none() {
        inst.world_name = Some("Debug world".into());
    }
    inst.apply(&LogEvent::PlayerJoined { name: NAME.into(), user_id: Some(USER_ID.into()) }, at);

    // Past WooHoos.
    for days in [9, 6, 2] {
        data.fun_log.push(FunEvent {
            time_ms: now_ms - days * DAY_MS,
            partner_name: Some(NAME.into()),
            partner_user_id: Some(USER_ID.into()),
            avatar_id: data.current_avatar_id.clone(),
            world_name: Some("Debug world".into()),
        });
    }
    data.fun_log.sort_by_key(|f| f.time_ms);

    // A past baby together (a one-week pregnancy that ended a month ago).
    let partner = Partner { status: PartnerStatus::Confirmed, name: Some(NAME.into()), user_id: Some(USER_ID.into()), avatar_name: Some("Test Bunny".into()), ..Default::default() };
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

/// Removes Test Buddy and everything that came from them: WooHoos,
/// pregnancies (with their journal and keepsakes), grace periods, history,
/// and achievements they unlocked that you no longer have without them.
pub fn remove(data: &mut AppData, now_ms: i64) {
    let since = data.test_buddy_since.take();
    let ids: Vec<String> = data.pregnancies.iter().filter(|p| is_test(p.partner.user_id.as_deref())).map(|p| p.id.clone()).collect();
    for id in ids {
        actions::delete_pregnancy(data, &id);
    }
    data.waiting.retain(|w| !is_test(w.partner.user_id.as_deref()));
    data.instance.present.retain(|p| !is_test(p.user_id.as_deref()));
    actions::forget_person(data, NAME, Some(USER_ID));
    // Achievements unlocked while testing that are no longer earned.
    if let Some(since) = since {
        let gone: Vec<String> = data
            .achievements
            .iter()
            .filter(|(id, at)| **at >= since && !crate::achievements::earned(data, id, now_ms))
            .map(|(id, _)| id.clone())
            .collect();
        for id in gone {
            data.achievements.remove(&id);
        }
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

        add(&mut d, NOW);
        add(&mut d, NOW + 1);
        assert_eq!(d.fun_log.len(), 4, "added once");
        assert!(d.instance.others().iter().any(|p| is_test(p.user_id.as_deref())));
        let baby = d.pregnancies.iter().find(|p| is_test(p.partner.user_id.as_deref())).unwrap();
        assert!(baby.born && d.active_pregnancy_id.is_none());
        crate::achievements::check(&mut d, NOW + 10);
        assert!(d.achievements.contains_key("welcome_home"));

        // A real pregnancy started with Test Buddy while testing goes too.
        let p = Partner { name: Some(NAME.into()), user_id: Some(USER_ID.into()), ..Default::default() };
        actions::start_pregnancy(&mut d, NewPregnancy { partner: Some(p), ..Default::default() }, NOW + 20);

        remove(&mut d, NOW + 30);
        assert_eq!(d.fun_log, before.fun_log);
        assert!(d.pregnancies.is_empty() && d.active_pregnancy_id.is_none());
        assert!(d.instance.present.is_empty() && d.instance.history.is_empty());
        assert_eq!(d.achievements, before.achievements, "only the real one is kept");
        assert!(d.test_buddy_since.is_none());
    }
}
