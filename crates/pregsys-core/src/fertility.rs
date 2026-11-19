//! Fertility cycle and grace period. Mirrored in `src/lib/fertility.ts`.
//!
//! Everything scales with the pregnancy length the user picked: a cycle is
//! 1/10 of it (28 days vs 280, like real life) and the grace period before
//! you find out is 1/20 (two pregnancy weeks), unless a fixed wait is picked.

use crate::model::{AppData, BabyOdds, Litter, LitterSpread, Multiples, Settings};
use crate::timeline::duration_for;

pub const CYCLE_DAYS: f64 = 28.0;

/// Chance of pregnancy per cycle day, relative to the peak (1.0).
/// Day 1 starts the cycle; the fertile window is days 9 to 16, peaking on 13.
const CURVE: [f64; 28] = [
    0.03, 0.03, 0.03, 0.03, 0.03, 0.03, 0.03, // 1-7
    0.08, 0.15, 0.35, 0.6, 0.85, 1.0, 0.9, // 8-14
    0.5, 0.2, 0.05, 0.03, 0.03, 0.03, 0.03, // 15-21
    0.03, 0.03, 0.03, 0.03, 0.03, 0.03, 0.03, // 22-28
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    Low,
    Medium,
    High,
    Peak,
}

/// Full pregnancy length for new pregnancies (what the user picked last).
pub fn term_ms(s: &Settings) -> i64 {
    duration_for(s.default_mode, s.default_speed, s.default_custom_ms)
}

pub fn cycle_ms(s: &Settings) -> i64 {
    (term_ms(s) / 10).max(60_000)
}

/// Real time from a WooHoo until you find out. A fixed wait is capped at 1/4
/// of the pregnancy: it counts from the WooHoo, so a long wait on a short
/// pregnancy would reveal it nearly over.
pub fn grace_ms(s: &Settings) -> i64 {
    let term = term_ms(s);
    let wait = match s.grace_minutes {
        Some(m) => (m as i64 * 60_000).min(term / 4),
        None => term / 20,
    };
    wait.max(60_000)
}

/// Cycle day as a fraction, from 1.0 up to (not including) 29.0.
pub fn cycle_day(now_ms: i64, anchor_ms: i64, cycle_ms: i64) -> f64 {
    let into = (now_ms - anchor_ms).rem_euclid(cycle_ms.max(1));
    1.0 + into as f64 / cycle_ms as f64 * CYCLE_DAYS
}

/// Chance of pregnancy (0 to 1) on a cycle day, with `peak` at the most fertile day.
pub fn chance(day: f64, peak: f64) -> f64 {
    let i = (day.floor() as usize).clamp(1, 28) - 1;
    (CURVE[i] * peak).clamp(0.0, 1.0)
}

pub fn level(chance: f64, peak: f64) -> Level {
    let r = if peak > 0.0 { chance / peak } else { 0.0 };
    if r >= 0.8 {
        Level::Peak
    } else if r >= 0.45 {
        Level::High
    } else if r >= 0.12 {
        Level::Medium
    } else {
        Level::Low
    }
}

/// Where a new cycle starts when none is set yet: a few days before the
/// fertile window, so the first days aren't all "low".
pub fn first_anchor(now_ms: i64, cycle_ms: i64) -> i64 {
    now_ms - (cycle_ms as f64 * 8.0 / CYCLE_DAYS) as i64
}

/// Pregnancy week when the number of babies can be seen.
pub const BABY_REVEAL_WEEK: f64 = 12.0;

/// Pregnancy week when the babies' genders can be seen (anatomy scan).
pub const GENDER_REVEAL_WEEK: f64 = 20.0;

/// "girl" or "boy" from a random number in [0, 1).
pub fn roll_gender(roll: f64) -> &'static str {
    if roll < 0.5 { "girl" } else { "boy" }
}

/// How many babies, from a random number in [0, 1).
pub fn roll_babies(m: Multiples, roll: f64) -> u32 {
    let (quads, triplets, twins) = match m {
        Multiples::Realistic => (0.0, 0.001, 0.031),
        Multiples::Playful => (0.002, 0.022, 0.122),
        Multiples::Lots => (0.02, 0.10, 0.35),
    };
    if roll < quads {
        4
    } else if roll < triplets {
        3
    } else if roll < twins {
        2
    } else {
        1
    }
}

/// The most babies a pregnancy can have.
pub const MAX_BABIES: u32 = 8;

/// The chance of each litter size, 1 to `MAX_BABIES`: a bell around the usual
/// size (same numbers as `litterOdds` in src/lib/fertility.ts).
pub fn litter_odds(l: Litter) -> Vec<f64> {
    let usual = l.usual.clamp(1, MAX_BABIES) as f64;
    let sd = match l.spread {
        LitterSpread::Little => 0.5,
        LitterSpread::Some => 0.9,
        LitterSpread::Lots => 1.5,
    };
    let w: Vec<f64> = (1..=MAX_BABIES).map(|k| (-((k as f64 - usual).powi(2)) / (2.0 * sd * sd)).exp()).collect();
    let total: f64 = w.iter().sum();
    w.into_iter().map(|x| x / total).collect()
}

/// A litter's size, from a random number in [0, 1).
pub fn roll_litter(l: Litter, roll: f64) -> u32 {
    let mut acc = 0.0;
    for (i, p) in litter_odds(l).into_iter().enumerate() {
        acc += p;
        if roll < acc {
            return i as u32 + 1;
        }
    }
    MAX_BABIES
}

/// How many babies, and the usual litter size when it's a litter.
pub fn roll_count(odds: BabyOdds, roll: f64) -> (u32, Option<u32>) {
    match odds {
        BabyOdds::Preset(m) => (roll_babies(m, roll), None),
        BabyOdds::Litter(l) => (roll_litter(l, roll), Some(l.usual.clamp(1, MAX_BABIES))),
    }
}

/// 0 low, 1 medium, 2 high, 3 peak; 0 when pregnant, waiting, or fertility is off.
pub fn level_now(data: &AppData, avatar_id: Option<&str>, now_ms: i64) -> u8 {
    let s = &data.settings;
    if !s.fertility || data.current_pregnancy(avatar_id).is_some() || data.pending_for(avatar_id).is_some() {
        return 0;
    }
    let Some(anchor) = s.cycle_anchor_ms else { return 0 };
    let c = chance(cycle_day(now_ms, anchor, cycle_ms(s)), s.fertility_peak);
    match level(c, s.fertility_peak) {
        Level::Low => 0,
        Level::Medium => 1,
        Level::High => 2,
        Level::Peak => 3,
    }
}

/// A random number in [0, 1) without extra crates (time-seeded xorshift).
pub fn roll() -> f64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9e37_79b9);
    let mut x = nanos ^ 0x2545_f491_4f6c_dd1d;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    (x >> 11) as f64 / (1u64 << 53) as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::{DurationMode, DAY_MS};

    fn month() -> Settings {
        Settings { default_mode: DurationMode::Custom, default_custom_ms: Some(30 * DAY_MS), ..Default::default() }
    }

    #[test]
    fn scales_with_pregnancy_length() {
        let s = month();
        assert_eq!(cycle_ms(&s), 3 * DAY_MS);
        assert_eq!(grace_ms(&s), DAY_MS * 3 / 2);
        let real = Settings { default_mode: DurationMode::Realtime, ..Default::default() };
        assert_eq!(cycle_ms(&real), 28 * DAY_MS);
        assert_eq!(grace_ms(&real), 14 * DAY_MS);
    }

    #[test]
    fn a_fixed_wait_ignores_the_length_but_is_capped() {
        let s = Settings { grace_minutes: Some(60), ..month() };
        assert_eq!(grace_ms(&s), 60 * 60_000);
        assert_eq!(cycle_ms(&s), 3 * DAY_MS, "the cycle doesn't change");
        let day = Settings { grace_minutes: Some(24 * 60), default_custom_ms: Some(DAY_MS), ..month() };
        assert_eq!(grace_ms(&day), DAY_MS / 4);
    }

    #[test]
    fn cycle_days_wrap() {
        let c = 28 * DAY_MS;
        assert_eq!(cycle_day(0, 0, c), 1.0);
        assert!((cycle_day(13 * DAY_MS, 0, c) - 14.0).abs() < 1e-9);
        assert_eq!(cycle_day(28 * DAY_MS, 0, c), 1.0, "new cycle");
        assert!((cycle_day(-DAY_MS, 0, c) - 28.0).abs() < 1e-9, "the day before day 1 is day 28");
    }

    #[test]
    fn chances_peak_mid_cycle() {
        assert!((chance(13.5, 0.6) - 0.6).abs() < 1e-9);
        assert!(chance(2.0, 0.6) < 0.05);
        assert!(chance(25.0, 0.6) < 0.05);
        assert_eq!(level(chance(13.0, 0.6), 0.6), Level::Peak);
        assert_eq!(level(chance(11.0, 0.6), 0.6), Level::High);
        assert_eq!(level(chance(9.0, 0.6), 0.6), Level::Medium);
        assert_eq!(level(chance(3.0, 0.6), 0.6), Level::Low);
    }

    #[test]
    fn first_anchor_lands_before_the_window() {
        let c = 28 * DAY_MS;
        let d = cycle_day(1000, first_anchor(1000, c), c);
        assert!((8.0..10.0).contains(&d));
    }

    #[test]
    fn multiples_odds() {
        let count = |m, n| (0..10_000).filter(|i| roll_babies(m, *i as f64 / 10_000.0) == n).count();
        assert_eq!(count(Multiples::Playful, 2), 1000);
        assert_eq!(count(Multiples::Playful, 3), 200);
        assert_eq!(count(Multiples::Realistic, 4), 0);
        assert_eq!(count(Multiples::Lots, 4), 200);
        assert_eq!(roll_babies(Multiples::Lots, 0.99), 1);
    }

    #[test]
    fn roll_is_in_range() {
        for _ in 0..100 {
            let r = roll();
            assert!((0.0..1.0).contains(&r));
        }
    }

    #[test]
    fn litters_bunch_around_the_usual_size() {
        let some = Litter { usual: 4, spread: LitterSpread::Some };
        let odds = litter_odds(some);
        assert_eq!(odds.len(), MAX_BABIES as usize);
        assert!((odds.iter().sum::<f64>() - 1.0).abs() < 1e-9);
        assert!((odds[3] - 0.44).abs() < 0.02, "4 is the most likely: {}", odds[3]);
        assert!((odds[2] - odds[4]).abs() < 1e-9, "3 and 5 as likely");
        let tight = litter_odds(Litter { usual: 4, spread: LitterSpread::Little });
        let wide = litter_odds(Litter { usual: 4, spread: LitterSpread::Lots });
        assert!(tight[3] > odds[3] && odds[3] > wide[3]);
        // Every roll lands in range, and the ends are reachable.
        let count = |l, n| (0..10_000).filter(|i| roll_litter(l, *i as f64 / 10_000.0) == n).count();
        assert!(count(some, 4) > count(some, 3));
        assert_eq!(roll_litter(Litter { usual: 8, spread: LitterSpread::Little }, 0.999), 8);
        assert_eq!(roll_litter(Litter { usual: 1, spread: LitterSpread::Little }, 0.0), 1);
        assert_eq!(roll_litter(Litter { usual: 99, spread: LitterSpread::Some }, 0.999), 8, "capped");
        assert_eq!(roll_count(BabyOdds::Litter(some), 0.5), (4, Some(4)));
        assert_eq!(roll_count(BabyOdds::Preset(Multiples::Realistic), 0.5), (1, None));
    }
}
