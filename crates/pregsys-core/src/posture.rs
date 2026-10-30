//! Standing or resting, from SteamVR's own tracking (for "Resting holds off
//! labor"). Positions are in SteamVR's raw space (the room as set up, before
//! any playspace mover), so floating your playspace doesn't fool it.
//!
//! - Headset height: how far below your normal height it is. Sitting down
//!   (from standing) or lying down puts it well below.
//! - Headset tilt: tipped far over (on your back or side) means lying down.
//! - With a waist (hip) tracker: the line from hip to headset. Mostly
//!   sideways means lying down, however high you are.
//!
//! Your normal height, guarded against tracking glitches: the highest height
//! the headset has spent [`PROVEN_MS`] in total at this session (standing,
//! or sitting up in a chair or wheelchair: no walking needed). A glitch never
//! stays put that long, and heights over [`MAX_STANDING_M`] above the floor
//! never count. A height saved from earlier sessions is the starting point,
//! so starting seated still counts as resting.
//!
//! Without SteamVR the app goes by VRChat's Upright / Seated / InStation.

use std::collections::BTreeMap;

/// Headset this far below your normal height (metres) counts as resting.
pub const DROP_RESTING_M: f32 = 0.45;
/// Hip-to-head line less upright than this (its vertical share) counts as lying down.
pub const LYING_UPRIGHT: f32 = 0.55;
/// Headset's up direction less upright than this (about 70° over) counts as lying down.
pub const HEAD_TILT_UPRIGHT: f32 = 0.35;
/// A change has to hold this long before it counts (a quick crouch isn't resting).
pub const HOLD_MS: i64 = 3_000;
/// Normal heights (headset above the floor) outside this aren't believed.
pub const MIN_STANDING_M: f32 = 0.9;
pub const MAX_STANDING_M: f32 = 2.3;
/// Time spent at a height (in total, give or take [`NEAR_BUCKETS`]) that proves it.
pub const PROVEN_MS: i64 = 20_000;
/// Heights are tallied in steps this big...
const BUCKET_M: f32 = 0.02;
/// ...and a height counts the time spent this many steps either side of it.
const NEAR_BUCKETS: i32 = 2;
/// One reading counts for at most this long (gaps don't count as time spent).
const MAX_STEP_MS: i64 = 1_000;

/// What the tracking says right now.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Reading {
    pub resting: bool,
    /// Headset below your normal height, in metres.
    pub drop_m: f32,
    /// Lying down by the hip tracker (None: no hip tracker).
    pub lying: Option<bool>,
    /// Lying down by the headset's tilt.
    pub tilted: bool,
    /// Your normal height has been proven this session (see [`PROVEN_MS`]).
    pub proven: bool,
}

#[derive(Debug, Default)]
pub struct VrPosture {
    /// The floor's height in raw space, when known.
    floor: Option<f32>,
    /// Your normal height saved from earlier sessions (raw space).
    seeded: Option<f32>,
    /// Time spent at each height this session (bucket -> ms), while worn and level.
    held: BTreeMap<i32, i64>,
    /// The last reading's time, while worn and level.
    last_at: Option<i64>,
    /// Only the height you set counts (nothing is learned).
    manual: bool,
    resting: bool,
    /// A change waiting to hold for [`HOLD_MS`]: (resting, since).
    pending: Option<(bool, i64)>,
}

impl VrPosture {
    /// Where the floor is in raw space (SteamVR's room setup).
    pub fn set_floor(&mut self, floor: f32) {
        self.floor = Some(floor);
    }

    /// Starts from a normal height (above the floor) saved earlier. Needs the
    /// floor; ignored if it isn't believable.
    pub fn seed(&mut self, standing_m: f32) {
        if let Some(f) = self.floor {
            self.seeded = (MIN_STANDING_M..=MAX_STANDING_M).contains(&standing_m).then_some(f + standing_m);
        }
    }

    /// Learn your height as you play (true), or only use the one you set.
    pub fn set_auto(&mut self, auto: bool) {
        self.manual = !auto;
    }

    /// You set your height just now: it replaces whatever was learned this session.
    pub fn set_height(&mut self, standing_m: f32) {
        self.held.clear();
        self.seed(standing_m);
    }

    /// The highest believable height the headset has spent [`PROVEN_MS`] at (raw space).
    fn proven(&self) -> Option<f32> {
        if self.manual {
            return None;
        }
        let near = |b: i32| self.held.range(b - NEAR_BUCKETS..=b + NEAR_BUCKETS).map(|(_, ms)| ms).sum::<i64>();
        self.held
            .keys()
            .rev()
            .map(|&b| (b, b as f32 * BUCKET_M))
            .find(|&(b, y)| self.floor.is_none_or(|f| (MIN_STANDING_M..=MAX_STANDING_M).contains(&(y - f))) && near(b) >= PROVEN_MS)
            .map(|(_, y)| y)
    }

    /// Your normal height above the floor, proven this session (the one worth saving).
    pub fn proven_m(&self) -> Option<f32> {
        Some(self.proven()? - self.floor?)
    }

    /// Takes the headset position (x/y/z, y up), how upright the headset is
    /// (its up direction's vertical share, 1 = level), if known the waist
    /// tracker position, and whether the headset is on your head (only then
    /// does it learn your height: not from a headset on a shelf).
    pub fn update(&mut self, head: [f32; 3], head_up: f32, hip: Option<[f32; 3]>, worn: bool, now_ms: i64) -> Reading {
        let tilted = head_up < HEAD_TILT_UPRIGHT;
        if worn && !tilted {
            if let Some(last) = self.last_at {
                let bucket = (head[1] / BUCKET_M).round() as i32;
                *self.held.entry(bucket).or_default() += (now_ms - last).clamp(0, MAX_STEP_MS);
            }
            self.last_at = Some(now_ms);
        } else {
            self.last_at = None;
        }
        let proven = self.proven();
        let top = match (proven, self.seeded) {
            (Some(p), Some(s)) => Some(p.max(s)),
            (p, s) => p.or(s),
        };
        let drop_m = top.map_or(0.0, |t| (t - head[1]).max(0.0));
        let lying = hip.map(|h| {
            let v = [head[0] - h[0], head[1] - h[1], head[2] - h[2]];
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            len > 0.2 && v[1] / len < LYING_UPRIGHT
        });
        let now = drop_m > DROP_RESTING_M || lying == Some(true) || tilted;
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
        Reading { resting: self.resting, drop_m, lying, tilted, proven: proven.is_some() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feeds the same pose once a second for `secs` seconds (headset level).
    fn hold(p: &mut VrPosture, head: [f32; 3], hip: Option<[f32; 3]>, from: i64, secs: i64) -> Reading {
        tilt(p, head, 1.0, hip, from, secs)
    }

    fn tilt(p: &mut VrPosture, head: [f32; 3], up: f32, hip: Option<[f32; 3]>, from: i64, secs: i64) -> Reading {
        (0..secs).map(|i| p.update(head, up, hip, true, from + i * 1000)).last().unwrap()
    }

    /// A posture that has learned you stand at 1.6 m (floor at 0).
    fn standing() -> VrPosture {
        let mut p = VrPosture::default();
        p.set_floor(0.0);
        hold(&mut p, [0.0, 1.6, 0.0], None, 0, 30);
        p
    }

    #[test]
    fn headset_height_sitting_and_standing() {
        let mut p = standing();
        // A quick crouch doesn't count...
        p.update([0.0, 0.9, 0.0], 1.0, None, true, 31_000);
        assert!(!p.update([0.0, 1.6, 0.0], 1.0, None, true, 32_000).resting);
        // ...sitting down for a while does.
        let r = hold(&mut p, [0.0, 1.0, 0.0], None, 40_000, 5);
        assert!(r.resting && (r.drop_m - 0.6).abs() < 1e-4);
        assert!(!hold(&mut p, [0.0, 1.55, 0.0], None, 50_000, 5).resting, "back up");
    }

    #[test]
    fn your_height_needs_time_there_but_no_walking() {
        let mut p = VrPosture::default();
        p.set_floor(0.0);
        assert!(!hold(&mut p, [0.0, 1.25, 0.0], None, 0, 15).proven, "not long enough yet");
        // Sitting up in a wheelchair the whole time is a normal height too.
        assert!(hold(&mut p, [0.0, 1.25, 0.0], None, 15_000, 10).proven);
        assert!((p.proven_m().unwrap() - 1.25).abs() <= 0.011);
        // Lying back from there counts as resting.
        assert!(tilt(&mut p, [0.0, 0.7, 0.0], 0.1, None, 30_000, 5).resting);
    }

    #[test]
    fn a_tracking_glitch_doesnt_make_you_resting() {
        let mut p = standing();
        p.update([0.0, 9.0, 0.0], 1.0, None, true, 31_000); // flung up for one reading
        assert!(!hold(&mut p, [0.0, 1.6, 0.0], None, 32_000, 10).resting);
    }

    #[test]
    fn a_long_glitch_flying_about_doesnt_either() {
        let mut p = standing();
        // Ten seconds of the headset sailing off, 4 readings a second.
        for i in 0..40 {
            p.update([i as f32, 1.7 + i as f32 * 0.05, 0.0], 1.0, None, true, 31_000 + i * 250);
        }
        assert!(!hold(&mut p, [0.0, 1.6, 0.0], None, 42_000, 10).resting);
    }

    #[test]
    fn a_glitch_stuck_in_place_for_a_while_doesnt_either() {
        let mut p = standing();
        hold(&mut p, [0.0, 2.2, 0.0], None, 31_000, 15); // frozen above you for 15 s
        assert!(!hold(&mut p, [0.0, 1.6, 0.0], None, 50_000, 10).resting);
        hold(&mut p, [0.0, 6.0, 0.0], None, 60_000, 60); // frozen in the sky: never believable
        assert!(!hold(&mut p, [0.0, 1.6, 0.0], None, 130_000, 10).resting);
    }

    #[test]
    fn a_tipped_over_headset_is_lying_down() {
        let mut p = VrPosture::default();
        hold(&mut p, [0.0, 1.2, 0.0], None, 0, 30); // seated all along: no height drop to go by
        // Looking up or nodding off a bit isn't lying down...
        assert!(!tilt(&mut p, [0.0, 1.2, 0.0], 0.6, None, 40_000, 5).resting);
        // ...lying back on the couch is.
        let r = tilt(&mut p, [0.0, 1.0, 0.0], 0.1, None, 50_000, 5);
        assert!(r.tilted && r.resting && r.drop_m < DROP_RESTING_M);
        assert!(!hold(&mut p, [0.0, 1.2, 0.0], None, 60_000, 5).resting, "sat back up");
    }

    #[test]
    fn the_hip_tracker_spots_lying_down_wherever_the_head_is() {
        let mut p = VrPosture::default();
        hold(&mut p, [0.0, 1.6, 0.0], Some([0.0, 1.0, 0.0]), 0, 30);
        // Lying down with the head still high (on a high bed or floated up): head beside the hip.
        let r = hold(&mut p, [0.6, 1.5, 0.0], Some([0.0, 1.45, 0.0]), 40_000, 5);
        assert_eq!(r.lying, Some(true));
        assert!(r.resting);
        // Standing again.
        let r = hold(&mut p, [0.0, 1.6, 0.0], Some([0.0, 1.0, 0.0]), 50_000, 5);
        assert!(!r.resting && r.lying == Some(false));
    }

    #[test]
    fn a_saved_height_catches_starting_seated() {
        let mut p = VrPosture::default();
        p.set_floor(0.1);
        p.seed(1.5);
        assert!(hold(&mut p, [0.0, 1.05, 0.0], None, 0, 5).resting, "seated from the start");
        assert!(!hold(&mut p, [0.0, 1.6, 0.0], None, 10_000, 5).resting);
    }

    #[test]
    fn an_unbelievable_saved_height_is_ignored() {
        let mut p = VrPosture::default();
        p.set_floor(0.0);
        p.seed(5.0);
        assert!(!hold(&mut p, [0.0, 1.6, 0.0], None, 0, 5).resting);
    }

    #[test]
    fn a_height_you_set_wins_and_manual_mode_learns_nothing() {
        let mut p = standing();
        // Set lower than what was learned (say you're a wheelchair user and that's your normal).
        p.set_height(1.2);
        assert!(!hold(&mut p, [0.0, 1.2, 0.0], None, 40_000, 5).resting);
        assert!(hold(&mut p, [0.0, 0.7, 0.0], None, 50_000, 5).resting, "slumped way down");
        // Manual: standing tall for ages doesn't change it.
        let mut p = VrPosture::default();
        p.set_floor(0.0);
        p.set_auto(false);
        p.set_height(1.6);
        hold(&mut p, [0.0, 2.1, 0.0], None, 0, 60);
        assert_eq!(p.proven_m(), None);
        assert!(!hold(&mut p, [0.0, 1.6, 0.0], None, 70_000, 5).resting);
        // Manual with no height set: only tilt and the hip tracker count.
        let mut p = VrPosture::default();
        p.set_auto(false);
        assert!(!hold(&mut p, [0.0, 0.8, 0.0], None, 0, 30).resting);
    }

    #[test]
    fn a_headset_on_a_shelf_isnt_your_height() {
        let mut p = VrPosture::default();
        p.set_floor(0.0);
        for i in 0..60 {
            p.update([0.0, 2.0, 0.0], 1.0, None, false, i * 1000);
        }
        assert_eq!(p.proven_m(), None);
        assert!(!hold(&mut p, [0.0, 1.6, 0.0], None, 70_000, 5).resting, "put on: standing is standing");
    }
}
