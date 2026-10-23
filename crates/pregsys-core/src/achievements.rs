//! Achievements: little milestones. Each unlocks once and is kept with the
//! time it happened (`AppData::achievements`). Emoji and hints for the UI live
//! in `src/lib/achievements.ts`; keep the ids and titles in step.

use crate::model::AppData;
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

fn birth_lengths(d: &AppData) -> impl Iterator<Item = i64> + '_ {
    d.pregnancies.iter().filter(|p| p.born).filter_map(|p| p.ended_ms.map(|e| e - p.timeline.start_ms))
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
];

pub fn title(id: &str) -> Option<&'static str> {
    ALL.iter().find(|a| a.id == id).map(|a| a.title)
}

/// Unlocks anything newly earned and returns it (in list order).
pub fn check(d: &mut AppData, now_ms: i64) -> Vec<&'static str> {
    let new: Vec<&'static str> = ALL.iter().filter(|a| !d.achievements.contains_key(a.id) && (a.earned)(d, now_ms)).map(|a| a.id).collect();
    for id in &new {
        d.achievements.insert(id.to_string(), now_ms);
    }
    new
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

    #[test]
    fn every_id_is_unique() {
        let ids: BTreeSet<_> = ALL.iter().map(|a| a.id).collect();
        assert_eq!(ids.len(), ALL.len());
    }
}
