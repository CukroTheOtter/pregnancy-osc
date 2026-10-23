//! Pregnancy timeline math.
//!
//! A pregnancy is stored as a start time plus a duration, never as a current
//! size. Progress is always `(now - start - paused) / duration`, so it is
//! correct whenever the app opens, even after weeks away.

use serde::{Deserialize, Serialize};

pub const DAY_MS: i64 = 86_400_000;
/// A full human term, used as the base for every timeline.
pub const TERM_DAYS: f64 = 280.0;
pub const TERM_WEEKS: f64 = 40.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DurationMode {
    /// About 9 months of real time.
    Realtime,
    /// Faster than real time by `speed` (7 = one real day is one pregnancy week).
    Accelerated,
    /// A total real duration chosen by the user.
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timeline {
    pub start_ms: i64,
    pub duration_ms: i64,
    pub mode: DurationMode,
    /// Speed multiplier for accelerated mode (1 = real time).
    pub speed: f64,
    #[serde(default)]
    pub paused_at_ms: Option<i64>,
    #[serde(default)]
    pub paused_total_ms: i64,
}

/// Real duration of a full term for a mode.
pub fn duration_for(mode: DurationMode, speed: f64, custom_ms: Option<i64>) -> i64 {
    let term = TERM_DAYS * DAY_MS as f64;
    match mode {
        DurationMode::Realtime => term as i64,
        DurationMode::Accelerated => (term / speed.max(0.001)) as i64,
        DurationMode::Custom => custom_ms.unwrap_or(term as i64).max(60_000),
    }
}

impl Timeline {
    pub fn new(start_ms: i64, mode: DurationMode, speed: f64, custom_ms: Option<i64>) -> Self {
        let speed = if mode == DurationMode::Realtime { 1.0 } else { speed };
        Self {
            start_ms,
            duration_ms: duration_for(mode, speed, custom_ms),
            mode,
            speed,
            paused_at_ms: None,
            paused_total_ms: 0,
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused_at_ms.is_some()
    }

    /// Real time counted toward the pregnancy.
    pub fn elapsed_ms(&self, now_ms: i64) -> i64 {
        let end = self.paused_at_ms.unwrap_or(now_ms);
        (end - self.start_ms - self.paused_total_ms).max(0)
    }

    /// 0 at conception, 1 at the due date, above 1 when overdue.
    pub fn progress(&self, now_ms: i64) -> f64 {
        if self.duration_ms <= 0 {
            return 1.0;
        }
        self.elapsed_ms(now_ms) as f64 / self.duration_ms as f64
    }

    /// Pregnancy week (0 to 40, more when overdue).
    pub fn week(&self, now_ms: i64) -> f64 {
        self.progress(now_ms) * TERM_WEEKS
    }

    /// When the due date falls, given pauses so far (a running pause pushes it back).
    pub fn due_ms(&self, now_ms: i64) -> i64 {
        let running_pause = self.paused_at_ms.map(|p| now_ms - p).unwrap_or(0);
        self.start_ms + self.paused_total_ms + running_pause + self.duration_ms
    }

    /// Real time at which a given progress is reached (ignores future pauses).
    pub fn time_at_progress(&self, now_ms: i64, p: f64) -> i64 {
        let running_pause = self.paused_at_ms.map(|x| now_ms - x).unwrap_or(0);
        self.start_ms + self.paused_total_ms + running_pause + (p * self.duration_ms as f64) as i64
    }

    pub fn pause(&mut self, now_ms: i64) {
        if self.paused_at_ms.is_none() {
            self.paused_at_ms = Some(now_ms);
        }
    }

    pub fn resume(&mut self, now_ms: i64) {
        if let Some(p) = self.paused_at_ms.take() {
            self.paused_total_ms += (now_ms - p).max(0);
        }
    }

    /// Changes the speed without making the belly jump: progress stays the
    /// same, only the time left changes.
    pub fn retime(&mut self, now_ms: i64, mode: DurationMode, speed: f64, custom_ms: Option<i64>) {
        let p = self.progress(now_ms);
        let speed = if mode == DurationMode::Realtime { 1.0 } else { speed };
        self.mode = mode;
        self.speed = speed;
        self.duration_ms = duration_for(mode, speed, custom_ms);
        let end = self.paused_at_ms.unwrap_or(now_ms);
        self.start_ms = end - self.paused_total_ms - (p * self.duration_ms as f64) as i64;
    }

    /// Jumps to a given progress (used by "set week" and conception back-dating).
    pub fn set_progress(&mut self, now_ms: i64, p: f64) {
        let end = self.paused_at_ms.unwrap_or(now_ms);
        self.start_ms = end - self.paused_total_ms - (p.max(0.0) * self.duration_ms as f64) as i64;
    }
}

/// Trimester for a progress value (1, 2 or 3).
pub fn trimester(p: f64) -> u8 {
    let w = p * TERM_WEEKS;
    if w < 13.0 {
        1
    } else if w < 27.0 {
        2
    } else {
        3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_700_000_000_000;

    #[test]
    fn realtime_progress() {
        let t = Timeline::new(T0, DurationMode::Realtime, 99.0, None);
        assert_eq!(t.speed, 1.0);
        assert_eq!(t.progress(T0), 0.0);
        assert!((t.progress(T0 + 140 * DAY_MS) - 0.5).abs() < 1e-9);
        assert!((t.week(T0 + 140 * DAY_MS) - 20.0).abs() < 1e-9);
        assert_eq!(t.due_ms(T0), T0 + 280 * DAY_MS);
    }

    #[test]
    fn accelerated_one_day_is_one_week() {
        let t = Timeline::new(T0, DurationMode::Accelerated, 7.0, None);
        assert!((t.week(T0 + DAY_MS) - 1.0).abs() < 1e-9);
        assert!((t.progress(T0 + 40 * DAY_MS) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn progress_survives_weeks_away() {
        let t = Timeline::new(T0, DurationMode::Accelerated, 7.0, None);
        // App closed for 50 days: overdue, not stuck.
        assert!(t.progress(T0 + 50 * DAY_MS) > 1.2);
    }

    #[test]
    fn custom_duration() {
        let t = Timeline::new(T0, DurationMode::Custom, 1.0, Some(2 * 3_600_000));
        assert!((t.progress(T0 + 3_600_000) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn pause_and_resume() {
        let mut t = Timeline::new(T0, DurationMode::Realtime, 1.0, None);
        t.pause(T0 + 10 * DAY_MS);
        let p = t.progress(T0 + 10 * DAY_MS);
        assert_eq!(t.progress(T0 + 30 * DAY_MS), p, "frozen while paused");
        assert_eq!(t.due_ms(T0 + 30 * DAY_MS), T0 + 300 * DAY_MS);
        t.resume(T0 + 30 * DAY_MS);
        assert_eq!(t.progress(T0 + 30 * DAY_MS), p);
        assert!((t.progress(T0 + 40 * DAY_MS) - 20.0 / 280.0).abs() < 1e-9);
        assert_eq!(t.due_ms(T0 + 40 * DAY_MS), T0 + 300 * DAY_MS);
    }

    #[test]
    fn double_pause_is_harmless() {
        let mut t = Timeline::new(T0, DurationMode::Realtime, 1.0, None);
        t.pause(T0 + DAY_MS);
        t.pause(T0 + 5 * DAY_MS);
        t.resume(T0 + 6 * DAY_MS);
        t.resume(T0 + 7 * DAY_MS);
        assert_eq!(t.paused_total_ms, 5 * DAY_MS);
    }

    #[test]
    fn backdated_conception() {
        let t = Timeline::new(T0 - 70 * DAY_MS, DurationMode::Realtime, 1.0, None);
        assert!((t.week(T0) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn retime_keeps_progress() {
        let mut t = Timeline::new(T0, DurationMode::Realtime, 1.0, None);
        let now = T0 + 70 * DAY_MS;
        let before = t.progress(now);
        t.retime(now, DurationMode::Accelerated, 7.0, None);
        assert!((t.progress(now) - before).abs() < 1e-6);
        // Now one day is one week.
        assert!((t.week(now + DAY_MS) - t.week(now) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn retime_while_paused() {
        let mut t = Timeline::new(T0, DurationMode::Realtime, 1.0, None);
        t.pause(T0 + 28 * DAY_MS);
        let before = t.progress(T0 + 50 * DAY_MS);
        t.retime(T0 + 50 * DAY_MS, DurationMode::Accelerated, 7.0, None);
        assert!((t.progress(T0 + 60 * DAY_MS) - before).abs() < 1e-6);
    }

    #[test]
    fn set_progress() {
        let mut t = Timeline::new(T0, DurationMode::Realtime, 1.0, None);
        t.set_progress(T0, 0.25);
        assert!((t.week(T0) - 10.0).abs() < 1e-6);
    }

    #[test]
    fn trimesters() {
        assert_eq!(trimester(0.1), 1);
        assert_eq!(trimester(0.5), 2);
        assert_eq!(trimester(0.9), 3);
    }
}
