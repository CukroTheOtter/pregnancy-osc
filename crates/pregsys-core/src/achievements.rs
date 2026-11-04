//! Achievements: little milestones. Each unlocks once and is kept with the
//! time it happened (`AppData::achievements`). Emoji and hints for the UI live
//! in `src/lib/achievements.ts`; keep the ids and titles in step.

use crate::model::{AppData, FunEvent, PartnerStatus};
use crate::timeline::{DAY_MS, TERM_WEEKS};
use std::collections::{BTreeSet, HashMap};

pub struct Achievement {
    pub id: &'static str,
    pub title: &'static str,
    earned: fn(&AppData, i64) -> bool,
}

/// Same person: user ID when there is one, else the name.
fn person_key(name: &str, user_id: Option<&str>) -> String {
    user_id.map(str::to_string).unwrap_or_else(|| format!("name:{}", name.to_lowercase()))
}

fn babies_born(d: &AppData) -> u32 {
    d.pregnancies.iter().filter(|p| p.born).map(|p| p.baby_count).sum()
}

/// Babies you know about: born, or seen on a scan (week 12).
fn known_babies(d: &AppData, now: i64, at_least: u32) -> bool {
    d.pregnancies.iter().any(|p| {
        let week = p.timeline.progress(p.ended_ms.unwrap_or(now)) * TERM_WEEKS;
        p.baby_count >= at_least && (p.born || week >= crate::fertility::BABY_REVEAL_WEEK)
    })
}

fn partners(d: &AppData) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for f in &d.fun_log {
        if let Some(n) = &f.partner_name {
            out.insert(person_key(n, f.partner_user_id.as_deref()));
        }
    }
    for p in &d.pregnancies {
        if let Some(n) = &p.partner.name {
            out.insert(person_key(n, p.partner.user_id.as_deref()));
        }
    }
    out
}

/// How far along each birth was (1 = the due date).
fn birth_progress(d: &AppData) -> impl Iterator<Item = f64> + '_ {
    d.pregnancies.iter().filter(|p| p.born).filter_map(|p| p.ended_ms.map(|e| p.timeline.progress(e)))
}

fn birth_lengths(d: &AppData) -> impl Iterator<Item = i64> + '_ {
    d.pregnancies.iter().filter(|p| p.born).filter_map(|p| p.ended_ms.map(|e| e - p.timeline.start_ms))
}

/// WooHoos with a saved result, oldest first.
fn with_results(d: &AppData) -> Vec<&FunEvent> {
    let mut out: Vec<&FunEvent> = d.fun_log.iter().filter(|f| f.took.is_some()).collect();
    out.sort_by_key(|f| f.time_ms);
    out
}

/// Pregnant after this many waits that didn't take. A wait is known by its
/// first WooHoo; a WooHoo saved without one counts by itself.
fn pregnant_after_misses(d: &AppData, misses: usize) -> bool {
    let results = with_results(d);
    let wait_of = |f: &FunEvent| f.wait_ms.unwrap_or(f.time_ms);
    let took: BTreeSet<i64> = results.iter().filter(|f| f.took == Some(true)).map(|f| wait_of(f)).collect();
    let missed: BTreeSet<i64> = results.iter().map(|f| wait_of(f)).filter(|w| !took.contains(w)).collect();
    took.iter().any(|t| missed.iter().filter(|m| *m < t).count() >= misses)
}

/// The most WooHoos in one wait, running or finished.
fn most_in_a_wait(d: &AppData) -> usize {
    let mut n: HashMap<i64, usize> = HashMap::new();
    for w in d.fun_log.iter().filter_map(|f| f.wait_ms) {
        *n.entry(w).or_default() += 1;
    }
    let finished = n.values().copied().max().unwrap_or(0);
    d.waiting.iter().map(|w| w.fun_times.len()).max().unwrap_or(0).max(finished)
}

/// How many WooHoos in each world.
fn worlds(d: &AppData) -> HashMap<&str, usize> {
    let mut n = HashMap::new();
    for w in d.fun_log.iter().filter_map(|f| f.world_name.as_deref()) {
        *n.entry(w).or_default() += 1;
    }
    n
}

/// Both a girl and a boy, once genders are known (a birth or week 20).
fn one_of_each(d: &AppData, now: i64) -> bool {
    d.pregnancies.iter().any(|p| {
        let week = p.timeline.progress(p.ended_ms.unwrap_or(now)) * TERM_WEEKS;
        let known = p.born || week >= crate::fertility::GENDER_REVEAL_WEEK;
        known && p.genders.iter().any(|g| g == "girl") && p.genders.iter().any(|g| g == "boy")
    })
}

pub const ALL: &[Achievement] = &[
    Achievement { id: "first_woohoo", title: "First WooHoo", earned: |d, _| !d.fun_log.is_empty() },
    Achievement { id: "busy_bee", title: "Busy Bee", earned: |d, _| d.fun_log.len() >= 25 },
    Achievement { id: "little_bean", title: "Little Bean", earned: |d, _| !d.pregnancies.is_empty() },
    Achievement { id: "double_trouble", title: "Double Trouble", earned: |d, now| known_babies(d, now, 2) },
    Achievement { id: "triple_treat", title: "Triple Treat", earned: |d, now| known_babies(d, now, 3) },
    Achievement { id: "welcome_home", title: "Welcome Home", earned: |d, _| d.pregnancies.iter().any(|p| p.born) },
    Achievement { id: "full_house", title: "Full House", earned: |d, _| babies_born(d) >= 5 },
    Achievement {
        id: "fashionably_late",
        title: "Fashionably Late",
        earned: |d, now| d.pregnancies.iter().any(|p| p.timeline.progress(p.ended_ms.unwrap_or(now)) > 1.01),
    },
    Achievement { id: "splash", title: "Splash!", earned: |d, _| d.pregnancies.iter().any(|p| p.born && p.labor.as_ref().is_some_and(|l| l.water_first)) },
    Achievement { id: "right_on_time", title: "Right On Time", earned: |d, _| birth_progress(d).any(|x| (x - 1.0).abs() <= 1.0 / 280.0) },
    Achievement { id: "early_bird", title: "Early Bird", earned: |d, _| birth_progress(d).any(|x| x * TERM_WEEKS < 37.0) },
    Achievement {
        id: "right_here_with_me",
        title: "Right Here With Me",
        earned: |d, _| d.pregnancies.iter().any(|p| p.born && p.labor.as_ref().is_some_and(|l| l.partner_at_birth)),
    },
    Achievement { id: "speedrun", title: "Speedrun", earned: |d, _| birth_lengths(d).any(|ms| ms <= DAY_MS) },
    Achievement { id: "long_haul", title: "The Long Haul", earned: |d, _| birth_lengths(d).any(|ms| ms >= 270 * DAY_MS) },
    Achievement { id: "social_butterfly", title: "Social Butterfly", earned: |d, _| partners(d).len() >= 5 },
    Achievement {
        id: "soulmates",
        title: "Soulmates",
        earned: |d, _| {
            let mut n: HashMap<String, u32> = HashMap::new();
            for p in &d.pregnancies {
                if let Some(name) = &p.partner.name {
                    *n.entry(person_key(name, p.partner.user_id.as_deref())).or_default() += 1;
                }
            }
            n.values().any(|&c| c >= 3)
        },
    },
    Achievement { id: "kick_counter", title: "Kick Counter", earned: |d, _| d.pregnancies.iter().map(|p| p.kicks.len()).sum::<usize>() >= 100 },
    Achievement { id: "wardrobe", title: "Wardrobe Change", earned: |d, _| d.avatars.values().filter(|a| a.has_system()).count() >= 5 },
    Achievement {
        id: "shape_shifter",
        title: "Shape Shifter",
        earned: |d, _| {
            let mut groups: HashMap<&str, usize> = HashMap::new();
            for a in d.avatars.values() {
                if let Some(g) = &a.group_id {
                    *groups.entry(g.as_str()).or_default() += 1;
                }
            }
            groups.values().any(|&n| n >= 3)
        },
    },
    Achievement { id: "first_try", title: "First Try", earned: |d, _| with_results(d).first().is_some_and(|f| f.took == Some(true)) },
    Achievement { id: "never_give_up", title: "Never Give Up", earned: |d, _| pregnant_after_misses(d, 5) },
    Achievement { id: "cant_stop", title: "Can't Stop", earned: |d, _| most_in_a_wait(d) >= 5 },
    Achievement { id: "whos_your_daddy", title: "Who's Your Daddy?", earned: |d, _| d.pregnancies.iter().any(|p| p.partner.status == PartnerStatus::Random) },
    Achievement { id: "century_club", title: "Century Club", earned: |d, _| d.fun_log.len() >= 100 },
    Achievement { id: "four_leaf_clover", title: "Four Leaf Clover", earned: |d, now| known_babies(d, now, 4) },
    Achievement { id: "one_of_each", title: "One Of Each", earned: one_of_each },
    Achievement { id: "baby_boom", title: "Baby Boom", earned: |d, _| babies_born(d) >= 10 },
    Achievement { id: "little_athlete", title: "Little Athlete", earned: |d, _| d.pregnancies.iter().map(|p| p.kicks.len()).sum::<usize>() >= 500 },
    Achievement { id: "globetrotter", title: "Globetrotter", earned: |d, _| worlds(d).len() >= 10 },
    Achievement { id: "favorite_spot", title: "Favorite Spot", earned: |d, _| worlds(d).values().any(|&n| n >= 10) },
    Achievement { id: "taking_a_break", title: "Taking A Break", earned: |d, _| d.pregnancies.iter().any(|p| p.born && p.timeline.paused_total_ms > 0) },
];

/// Is this achievement earned with the data as it is now?
pub fn earned(d: &AppData, id: &str, now_ms: i64) -> bool {
    ALL.iter().find(|a| a.id == id).is_some_and(|a| (a.earned)(d, now_ms))
}

pub fn title(id: &str) -> Option<&'static str> {
    ALL.iter().find(|a| a.id == id).map(|a| a.title)
}

/// Unlocks anything newly earned and returns it (in list order).
pub fn check(d: &mut AppData, now_ms: i64) -> Vec<&'static str> {
    let new: Vec<&'static str> = ALL.iter().filter(|a| !d.achievements.contains_key(a.id) && (a.earned)(d, now_ms)).map(|a| a.id).collect();
    for id in &new {
        d.achievements.insert(id.to_string(), now_ms);
        // The debug tools count as if it all really happened, so it's only a test.
        if d.settings.debug {
            d.test_achievements.push(id.to_string());
        }
    }
    new
}

/// Debug: takes back everything unlocked while debug mode was on. Your test
/// data stays, so what it still qualifies for can unlock again.
pub fn clear_test(d: &mut AppData) {
    for id in std::mem::take(&mut d.test_achievements) {
        d.achievements.remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FunEvent, Pregnancy};
    use crate::timeline::{DurationMode, Timeline};

    fn preg(start: i64, babies: u32, born_at: Option<i64>) -> Pregnancy {
        Pregnancy {
            timeline: Timeline::new(start, DurationMode::Custom, 1.0, Some(DAY_MS)),
            baby_count: babies,
            born: born_at.is_some(),
            ended_ms: born_at,
            ..Default::default()
        }
    }

    #[test]
    fn wardrobe_counts_only_pregnancy_system_avatars() {
        let mut d = AppData::new();
        for i in 0..6 {
            // Only every other one has the system.
            let params = if i % 2 == 0 { vec!["Pregnancy/Stage".to_string()] } else { vec!["Other/Thing".to_string()] };
            d.avatars.insert(format!("a{i}"), crate::model::AvatarProfile { params, ..Default::default() });
        }
        assert!(!check(&mut d, 0).contains(&"wardrobe"), "only 3 have it");
        for i in 6..8 {
            d.avatars.insert(format!("a{i}"), crate::model::AvatarProfile { params: vec!["Pregnancy/Stage".into()], ..Default::default() });
        }
        assert!(check(&mut d, 1).contains(&"wardrobe"));
    }

    #[test]
    fn unlock_once_and_remember_when() {
        let mut d = AppData::new();
        assert!(check(&mut d, 0).is_empty());
        d.fun_log.push(FunEvent { time_ms: 5, partner_name: Some("Rex".into()), ..Default::default() });
        assert_eq!(check(&mut d, 10), vec!["first_woohoo"]);
        assert_eq!(d.achievements.get("first_woohoo"), Some(&10));
        assert!(check(&mut d, 20).is_empty(), "only once");
    }

    #[test]
    fn babies_and_births() {
        let mut d = AppData::new();
        // Twins born after a one-day pregnancy.
        d.pregnancies.push(preg(0, 2, Some(DAY_MS)));
        let got = check(&mut d, DAY_MS);
        for id in ["little_bean", "double_trouble", "welcome_home", "speedrun"] {
            assert!(got.contains(&id), "{id}");
        }
        assert!(!got.contains(&"triple_treat") && !got.contains(&"full_house") && !got.contains(&"long_haul"));
    }

    #[test]
    fn late_and_linked() {
        let mut d = AppData::new();
        d.pregnancies.push(preg(0, 1, None));
        assert!(!check(&mut d, DAY_MS / 2).contains(&"fashionably_late"));
        assert!(check(&mut d, DAY_MS * 2).contains(&"fashionably_late"));
        for id in ["a", "b", "c"] {
            crate::actions::touch_avatar(&mut d, id, 0);
        }
        crate::actions::create_group(&mut d, "Trio", &["a".into(), "b".into(), "c".into()]).unwrap();
        assert!(check(&mut d, 0).contains(&"shape_shifter"));
    }

    fn fun(time_ms: i64, took: Option<bool>, wait_ms: Option<i64>) -> FunEvent {
        FunEvent { time_ms, took, wait_ms, ..Default::default() }
    }

    #[test]
    fn trying_for_a_baby() {
        // The first result took.
        let mut d = AppData::new();
        d.fun_log = vec![fun(1, None, None), fun(2, Some(true), Some(2))];
        assert!(earned(&d, "first_try", 0), "older WooHoos have no result, so they don't count");
        // The first didn't.
        d.fun_log = vec![fun(1, Some(false), Some(1)), fun(2, Some(true), Some(1))];
        assert!(!earned(&d, "first_try", 0));

        // Five waits that didn't take, then one that did. Misses in the same wait are one.
        let misses = |n: i64| (0..n).map(|i| fun(i * 10, Some(false), Some(i * 10)));
        d.fun_log = misses(4).chain([fun(39, Some(false), Some(30)), fun(100, Some(true), Some(100))]).collect();
        assert!(!earned(&d, "never_give_up", 0), "four waits");
        d.fun_log = misses(5).chain([fun(100, Some(true), Some(100))]).collect();
        assert!(earned(&d, "never_give_up", 0));
        // Five misses after the pregnancy don't count.
        d.fun_log = std::iter::once(fun(-1, Some(true), Some(-1))).chain(misses(5)).collect();
        assert!(!earned(&d, "never_give_up", 0));

        // Five in one wait: finished, or still running.
        d.fun_log = (0..5).map(|i| fun(i, Some(false), Some(0))).collect();
        assert!(earned(&d, "cant_stop", 0));
        d.fun_log.clear();
        d.waiting.push(crate::model::PendingConception { fun_times: vec![1, 2, 3, 4], ..Default::default() });
        assert!(!earned(&d, "cant_stop", 0));
        d.waiting[0].fun_times.push(5);
        assert!(earned(&d, "cant_stop", 0));
    }

    #[test]
    fn counts_and_worlds() {
        let mut d = AppData::new();
        // 90 WooHoos: 9 in each of 10 worlds.
        d.fun_log = (0..90).map(|i| FunEvent { time_ms: i, world_name: Some(format!("World {}", i % 10)), ..Default::default() }).collect();
        assert!(earned(&d, "globetrotter", 0));
        assert!(!earned(&d, "favorite_spot", 0));
        d.fun_log.push(FunEvent { time_ms: 100, world_name: Some("World 1".into()), ..Default::default() });
        assert!(earned(&d, "favorite_spot", 0));
        assert!(!earned(&d, "century_club", 0));
        d.fun_log.extend(vec![FunEvent::default(); 9]);
        assert!(earned(&d, "century_club", 0));
        // No world name doesn't count as a world.
        d.fun_log = vec![FunEvent::default(); 20];
        assert!(!earned(&d, "favorite_spot", 0) && !earned(&d, "globetrotter", 0));
    }

    #[test]
    fn babies_kicks_and_breaks() {
        let mut d = AppData::new();
        let mut p = preg(0, 2, None);
        p.genders = vec!["girl".into(), "boy".into()];
        d.pregnancies.push(p);
        assert!(!earned(&d, "one_of_each", 0), "not known yet");
        assert!(earned(&d, "one_of_each", DAY_MS * 3 / 4), "past week 20");
        d.pregnancies[0].genders = vec!["girl".into(), "girl".into()];
        assert!(!earned(&d, "one_of_each", DAY_MS));

        d.pregnancies = vec![preg(0, 4, Some(DAY_MS)), preg(0, 4, Some(DAY_MS))];
        assert!(earned(&d, "four_leaf_clover", 0));
        assert!(!earned(&d, "baby_boom", 0), "8 born");
        d.pregnancies.push(preg(0, 2, Some(DAY_MS)));
        assert!(earned(&d, "baby_boom", 0));

        assert!(!earned(&d, "taking_a_break", 0));
        d.pregnancies[0].timeline.paused_total_ms = 1;
        assert!(earned(&d, "taking_a_break", 0));
        d.pregnancies[0].born = false;
        d.pregnancies[1].timeline.paused_total_ms = 0;
        assert!(!earned(&d, "taking_a_break", 0), "paused, but no birth yet");

        // Kicks add up across pregnancies.
        d.pregnancies[0].kicks = vec![Default::default(); 300];
        d.pregnancies[1].kicks = vec![Default::default(); 199];
        assert!(!earned(&d, "little_athlete", 0));
        d.pregnancies[1].kicks.push(Default::default());
        assert!(earned(&d, "little_athlete", 0));

        assert!(!earned(&d, "whos_your_daddy", 0));
        d.pregnancies[0].partner.status = PartnerStatus::Random;
        assert!(earned(&d, "whos_your_daddy", 0));
    }

    #[test]
    fn unlocked_in_debug_mode_is_only_a_test() {
        let mut d = AppData::new();
        d.fun_log.push(FunEvent::default());
        assert_eq!(check(&mut d, 1), ["first_woohoo"]);
        assert!(d.test_achievements.is_empty(), "the real thing");

        d.settings.debug = true;
        d.pregnancies.push(preg(0, 1, None));
        assert_eq!(check(&mut d, 2), ["little_bean"]);
        assert_eq!(d.test_achievements, ["little_bean"]);

        // Turning debug mode off doesn't make it real.
        d.settings.debug = false;
        assert!(check(&mut d, 3).is_empty());
        assert_eq!(d.test_achievements, ["little_bean"]);

        // Clearing leaves the real ones; what the data still earns comes back for real.
        clear_test(&mut d);
        assert!(d.test_achievements.is_empty() && d.achievements.contains_key("first_woohoo") && !d.achievements.contains_key("little_bean"));
        assert_eq!(check(&mut d, 4), ["little_bean"]);
        assert!(d.test_achievements.is_empty());
    }

    #[test]
    fn every_id_is_unique() {
        let ids: BTreeSet<_> = ALL.iter().map(|a| a.id).collect();
        assert_eq!(ids.len(), ALL.len());
    }
}
