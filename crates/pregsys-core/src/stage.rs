//! Turns pregnancy progress into the value written to the avatar's belly
//! parameter (`stage` in the mapping).

/// Below this the avatar treats the belly as "not pregnant".
pub const MIN_PREGNANT_STAGE: f32 = 0.0002;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StageInput {
    /// `None` = not pregnant.
    pub progress: Option<f64>,
    /// Per-avatar belly size multiplier (1 = normal).
    pub belly_scale: f64,
}

impl Default for StageInput {
    fn default() -> Self {
        Self { progress: None, belly_scale: 1.0 }
    }
}

pub fn stage_value(i: StageInput) -> f32 {
    let Some(p) = i.progress else { return 0.0 };
    let v = p.clamp(0.0, 1.0) * i.belly_scale.clamp(0.0, 2.0);
    (v.min(1.0) as f32).max(MIN_PREGNANT_STAGE)
}

/// What progress does an avatar's current belly value mean?
pub fn progress_from_stage(stage: f32) -> f64 {
    stage.clamp(0.0, 1.0) as f64
}

/// Input for the avatar's Extra size (`bellyExtra`): the SDK's Bigger Belly
/// adds Belly_Bigger on top of the normal belly, scaled by Stage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtraInput {
    pub progress: f64,
    pub babies: u32,
    /// A litter: its usual size (that size and smaller get no extra).
    pub litter: Option<u32>,
    /// Twins and more make the belly bigger.
    pub multiples: bool,
    /// The belly keeps growing past the due date.
    pub overdue: bool,
    /// How far past due (fraction of the term) it takes to reach full extra size.
    pub max_overdue_fraction: f64,
}

/// Extra size from 0 to 1: a bit for each extra baby, then growing to 1 when overdue.
/// A litter's usual size is a normal belly; bigger litters reach 0.8 at the most babies.
pub fn extra_value(i: ExtraInput) -> f32 {
    let max = crate::fertility::MAX_BABIES;
    let base = match (i.multiples, i.litter) {
        (false, _) => 0.0,
        (true, Some(usual)) if usual >= max || i.babies <= usual => 0.0,
        (true, Some(usual)) => 0.8 * (i.babies.min(max) - usual) as f64 / (max - usual) as f64,
        (true, None) => match i.babies {
            0 | 1 => 0.0,
            2 => 0.35,
            3 => 0.6,
            _ => 0.8,
        },
    };
    let over = if i.overdue && i.progress > 1.0 { ((i.progress - 1.0) / i.max_overdue_fraction.max(1e-6)).min(1.0) } else { 0.0 };
    (base + (1.0 - base) * over) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(p: f64) -> StageInput {
        StageInput { progress: Some(p), ..Default::default() }
    }

    #[test]
    fn not_pregnant_is_zero() {
        assert_eq!(stage_value(StageInput::default()), 0.0);
    }

    #[test]
    fn normal_range() {
        assert_eq!(stage_value(input(0.0)), MIN_PREGNANT_STAGE);
        assert!((stage_value(input(0.5)) - 0.5).abs() < 1e-6);
        assert_eq!(stage_value(input(1.7)), 1.0);
    }

    #[test]
    fn extra_size() {
        let mk = |progress, babies| ExtraInput { progress, babies, litter: None, multiples: true, overdue: true, max_overdue_fraction: 0.05 };
        assert_eq!(extra_value(mk(0.5, 1)), 0.0);
        assert!((extra_value(mk(0.5, 2)) - 0.35).abs() < 1e-6);
        assert!((extra_value(mk(0.5, 5)) - 0.8).abs() < 1e-6);
        // Overdue: halfway to full extra size, then capped at 1.
        assert!((extra_value(mk(1.025, 1)) - 0.5).abs() < 1e-6);
        assert!((extra_value(mk(1.025, 2)) - 0.675).abs() < 1e-6);
        assert_eq!(extra_value(mk(2.0, 3)), 1.0);
        // Both off.
        let off = ExtraInput { multiples: false, overdue: false, ..mk(2.0, 3) };
        assert_eq!(extra_value(off), 0.0);
    }

    #[test]
    fn a_usual_litter_is_a_normal_belly() {
        let mk = |babies| ExtraInput { progress: 0.5, babies, litter: Some(4), multiples: true, overdue: true, max_overdue_fraction: 0.05 };
        assert_eq!(extra_value(mk(3)), 0.0, "smaller than usual");
        assert_eq!(extra_value(mk(4)), 0.0, "the usual size");
        assert!((extra_value(mk(6)) - 0.4).abs() < 1e-6);
        assert!((extra_value(mk(8)) - 0.8).abs() < 1e-6, "the most babies");
        // Overdue still grows to full.
        assert_eq!(extra_value(ExtraInput { progress: 2.0, ..mk(4) }), 1.0);
        // Usual 8: never bigger than normal for its size.
        assert_eq!(extra_value(ExtraInput { litter: Some(8), ..mk(8) }), 0.0);
    }

    #[test]
    fn belly_scale() {
        let i = StageInput { progress: Some(0.5), belly_scale: 0.5, ..Default::default() };
        assert!((stage_value(i) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn inverse() {
        assert!((progress_from_stage(0.25) - 0.25).abs() < 1e-6);
    }
}
