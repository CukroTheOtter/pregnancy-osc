//! Standing or resting, from SteamVR's own tracking (for "Resting holds off
//! labor"). Positions are in SteamVR's raw space (the room as set up, before
//! any playspace mover), so floating your playspace doesn't fool it.
//!
//! - Headset height: how far below the highest it has been this session.
//!   Sitting or lying down puts it well below standing height.
//! - With a waist (hip) tracker: the line from hip to headset. Mostly
//!   sideways means lying down, however high you are.
//!
//! Without SteamVR the app goes by VRChat's Upright / Seated / InStation.

/// Headset this far below its highest (metres) counts as resting.
pub const DROP_RESTING_M: f32 = 0.45;
/// Hip-to-head line less upright than this (its vertical share) counts as lying down.
pub const LYING_UPRIGHT: f32 = 0.55;
/// A change has to hold this long before it counts (a quick crouch isn't resting).
pub const HOLD_MS: i64 = 3_000;
/// The highest headset height grows at most this much per reading (4 a second).
const MAX_RISE_M: f32 = 0.15;

/// What the tracking says right now.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Reading {
    pub resting: bool,
    /// Headset below its highest, in metres.
    pub drop_m: f32,
    /// Lying down by the hip tracker (None: no hip tracker).
    pub lying: Option<bool>,
}

#[derive(Debug, Default)]
pub struct VrPosture {
    /// Highest headset height seen (standing).
    top: Option<f32>,
    resting: bool,
    /// A change waiting to hold for [`HOLD_MS`]: (resting, since).
    pending: Option<(bool, i64)>,
}

impl VrPosture {
    /// Takes the headset and (if known) waist tracker positions, x/y/z with y up.
    pub fn update(&mut self, head: [f32; 3], hip: Option<[f32; 3]>, now_ms: i64) -> Reading {
        // Grows a little at a time, so a tracking glitch that flings the
        // headset up doesn't make you look like you're resting afterwards.
        let top = self.top.map_or(head[1], |t| t.max(head[1].min(t + MAX_RISE_M)));
        self.top = Some(top);
        let drop_m = top - head[1];
        let lying = hip.map(|h| {
            let v = [head[0] - h[0], head[1] - h[1], head[2] - h[2]];
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            len > 0.2 && v[1] / len < LYING_UPRIGHT
        });
        let now = drop_m > DROP_RESTING_M || lying == Some(true);
        if now == self.resting {
            self.pending = None;
        } else {
            match self.pending {
                Some((r, since)) if r == now => {
                    if now_ms - since >= HOLD_MS {
                        self.resting = now;
                        self.pending = None;
                    }
                }
                _ => self.pending = Some((now, now_ms)),
            }
        }
        Reading { resting: self.resting, drop_m, lying }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feeds the same pose once a second for `secs` seconds.
    fn hold(p: &mut VrPosture, head: [f32; 3], hip: Option<[f32; 3]>, from: i64, secs: i64) -> Reading {
        (0..secs).map(|i| p.update(head, hip, from + i * 1000)).last().unwrap()
    }

    #[test]
    fn headset_height_sitting_and_standing() {
        let mut p = VrPosture::default();
        assert!(!hold(&mut p, [0.0, 1.6, 0.0], None, 0, 5).resting, "standing");
        // A quick crouch doesn't count...
        p.update([0.0, 0.9, 0.0], None, 6_000);
        assert!(!p.update([0.0, 1.6, 0.0], None, 7_000).resting);
        // ...sitting down for a while does.
        let r = hold(&mut p, [0.0, 1.0, 0.0], None, 10_000, 5);
        assert!(r.resting && (r.drop_m - 0.6).abs() < 1e-4);
        assert!(!hold(&mut p, [0.0, 1.55, 0.0], None, 20_000, 5).resting, "back up");
    }

    #[test]
    fn a_tracking_glitch_doesnt_make_you_resting() {
        let mut p = VrPosture::default();
        hold(&mut p, [0.0, 1.6, 0.0], None, 0, 5);
        p.update([0.0, 9.0, 0.0], None, 6_000); // flung up for one reading
        assert!(!hold(&mut p, [0.0, 1.6, 0.0], None, 7_000, 10).resting);
    }

    #[test]
    fn the_hip_tracker_spots_lying_down_wherever_the_head_is() {
        let mut p = VrPosture::default();
        hold(&mut p, [0.0, 1.6, 0.0], Some([0.0, 1.0, 0.0]), 0, 5);
        // Lying down with the head still high (on a high bed or floated up): head beside the hip.
        let r = hold(&mut p, [0.6, 1.5, 0.0], Some([0.0, 1.45, 0.0]), 10_000, 5);
        assert_eq!(r.lying, Some(true));
        assert!(r.resting);
        // Standing again.
        let r = hold(&mut p, [0.0, 1.6, 0.0], Some([0.0, 1.0, 0.0]), 20_000, 5);
        assert!(!r.resting && r.lying == Some(false));
    }
}
