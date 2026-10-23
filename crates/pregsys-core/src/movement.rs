//! Belly movement (kicks): when they happen and how strong. Pure logic;
//! the driver sends them to the avatar's FetalMovement parameter.

use serde::{Deserialize, Serialize};

/// Kicks start around here (quickening).
pub const FIRST_KICK_WEEK: f64 = 16.0;
pub const STRONG_FROM_WEEK: f64 = 24.0;
pub const EXTREME_FROM_WEEK: f64 = 32.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum KickStrength {
    #[default]
    Weak,
    Strong,
    Extreme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum KickFrequency {
    Calm,
    #[default]
    Normal,
    Active,
}

/// Which strengths the user allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AllowedKicks {
    pub weak: bool,
    pub strong: bool,
    pub extreme: bool,
}

impl Default for AllowedKicks {
    fn default() -> Self {
        Self { weak: true, strong: true, extreme: true }
    }
}

/// Real time until the next kick: a few minutes, whatever the pregnancy
/// length. More babies kick more often. `roll` is random in [0, 1).
pub fn next_kick_delay_ms(freq: KickFrequency, babies: u32, roll: f64) -> i64 {
    let (lo, hi) = match freq {
        KickFrequency::Calm => (8.0, 20.0),
        KickFrequency::Normal => (3.0, 10.0),
        KickFrequency::Active => (1.0, 4.0),
    };
    let minutes = (lo + (hi - lo) * roll) / (babies.max(1) as f64).sqrt();
    (minutes * 60_000.0) as i64
}

/// How strong a kick is at this week, or `None` if no kick is possible
/// (too early, or nothing allowed). Later weeks favour stronger kicks.
pub fn pick_strength(week: f64, allowed: AllowedKicks, roll: f64) -> Option<KickStrength> {
    if week < FIRST_KICK_WEEK {
        return None;
    }
    let mut options: Vec<(KickStrength, f64)> = Vec::new();
    if allowed.weak {
        options.push((KickStrength::Weak, if week < STRONG_FROM_WEEK { 1.0 } else { 0.5 }));
    }
    if allowed.strong && week >= STRONG_FROM_WEEK {
        options.push((KickStrength::Strong, if week < EXTREME_FROM_WEEK { 0.5 } else { 0.4 }));
    }
    if allowed.extreme && week >= EXTREME_FROM_WEEK {
        options.push((KickStrength::Extreme, 0.15));
    }
    // e.g. only "strong" allowed at week 20: no kicks until strong ones are possible.
    if options.is_empty() {
        return None;
    }
    let total: f64 = options.iter().map(|(_, w)| w).sum();
    let mut r = roll * total;
    for (s, w) in &options {
        r -= w;
        if r <= 0.0 {
            return Some(*s);
        }
    }
    options.last().map(|(s, _)| *s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_kicks_early() {
        assert_eq!(pick_strength(10.0, AllowedKicks::default(), 0.5), None);
    }

    #[test]
    fn strength_grows_with_the_weeks() {
        let all = AllowedKicks::default();
        let seen = |week: f64| -> Vec<KickStrength> {
            let mut v: Vec<_> = (0..100).filter_map(|i| pick_strength(week, all, i as f64 / 100.0)).collect();
            v.sort_by_key(|s| *s as u8);
            v.dedup();
            v
        };
        assert_eq!(seen(18.0), vec![KickStrength::Weak]);
        assert_eq!(seen(26.0), vec![KickStrength::Weak, KickStrength::Strong]);
        assert_eq!(seen(34.0), vec![KickStrength::Weak, KickStrength::Strong, KickStrength::Extreme]);
    }

    #[test]
    fn respects_allowed() {
        let only_weak = AllowedKicks { weak: true, strong: false, extreme: false };
        assert!((0..100).all(|i| pick_strength(36.0, only_weak, i as f64 / 100.0) == Some(KickStrength::Weak)));
        let none = AllowedKicks { weak: false, strong: false, extreme: false };
        assert_eq!(pick_strength(36.0, none, 0.5), None);
    }

    #[test]
    fn delays_scale_with_frequency_and_babies() {
        let calm = next_kick_delay_ms(KickFrequency::Calm, 1, 0.5);
        let active = next_kick_delay_ms(KickFrequency::Active, 1, 0.5);
        assert!(calm > active);
        assert!(next_kick_delay_ms(KickFrequency::Normal, 4, 0.5) < next_kick_delay_ms(KickFrequency::Normal, 1, 0.5));
        assert_eq!(next_kick_delay_ms(KickFrequency::Active, 1, 0.0), 60_000);
    }
}
