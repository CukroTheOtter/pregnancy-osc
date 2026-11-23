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

/// A baby's personality, one per pregnancy: how busy it is and how hard it kicks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KickMood {
    Sleepy,
    Calm,
    Wiggly,
    Bouncy,
}

impl KickMood {
    /// How many times as often as the "How often" setting it kicks.
    pub fn rate(self) -> f64 {
        match self {
            KickMood::Sleepy => 0.5,
            KickMood::Calm => 0.75,
            KickMood::Wiggly => 1.3,
            KickMood::Bouncy => 2.0,
        }
    }

    /// Bends the strength roll: above 1 favours little kicks, below 1 big ones.
    pub fn strength_bend(self) -> f64 {
        match self {
            KickMood::Sleepy => 1.6,
            KickMood::Calm => 1.2,
            KickMood::Wiggly => 0.9,
            KickMood::Bouncy => 0.6,
        }
    }
}

/// FNV-1a: a stable number from text (same as `hashText` in src/lib/kickMood.ts).
pub fn hash(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// The personality a pregnancy gets when none was picked: from its ID, so it
/// never changes (20% sleepy, 30% calm, 30% wiggly, 20% bouncy).
pub fn mood_for(id: &str) -> KickMood {
    match hash(id) % 100 {
        0..20 => KickMood::Sleepy,
        20..50 => KickMood::Calm,
        50..80 => KickMood::Wiggly,
        _ => KickMood::Bouncy,
    }
}

/// How long a busy or quiet spell lasts.
pub const SPELL_MS: i64 = 40 * 60_000;

/// Busy and quiet spells, so no hour is quite like the last: a quarter of
/// them half as busy, a quarter nearly twice as busy.
pub fn spell(id: &str, now_ms: i64) -> f64 {
    match hash(&format!("{id}/{}", now_ms.div_euclid(SPELL_MS))) % 4 {
        0 => 0.5,
        3 => 1.8,
        _ => 1.0,
    }
}

/// After a WooHoo while pregnant the baby is awake and busy for this long.
pub const WAKE_MS: i64 = 15 * 60_000;

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
    fn moods_are_stable_and_all_happen() {
        assert_eq!(mood_for("preg-abc"), mood_for("preg-abc"));
        let mut seen: Vec<KickMood> = (0..200).map(|i| mood_for(&format!("preg-{i}"))).collect();
        seen.sort_by_key(|m| *m as u8);
        seen.dedup();
        assert_eq!(seen.len(), 4);
        assert!(KickMood::Bouncy.rate() > KickMood::Sleepy.rate());
    }

    #[test]
    fn same_moods_as_the_ui() {
        // The same IDs and answers as src/lib/kickMood.test.ts.
        assert_eq!(hash("preg-abc"), 6_813_124_331_427_210_414);
        let got: Vec<KickMood> = ["preg-abc", "preg-2", "preg-3", "preg-4"].iter().map(|id| mood_for(id)).collect();
        assert_eq!(got, vec![KickMood::Sleepy, KickMood::Wiggly, KickMood::Bouncy, KickMood::Calm]);
    }

    #[test]
    fn spells_vary_but_hold_for_a_while() {
        assert_eq!(spell("p", 0), spell("p", SPELL_MS - 1));
        let all: Vec<f64> = (0..50).map(|i| spell("p", i * SPELL_MS)).collect();
        assert!(all.contains(&0.5) && all.contains(&1.0) && all.contains(&1.8));
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
