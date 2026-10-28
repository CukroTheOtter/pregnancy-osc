//! Labor and birth: when labor starts, its phases, contractions, and whether
//! the partner is here. Pure logic: the engine runs [`Watch::tick`] every
//! second and the driver sends the phase (`Pregnancy/Labor`).
//!
//! Labor only moves while you're in-game on an avatar that can show it
//! (it has the labor parameter). On any other avatar it waits, or pauses
//! partway, and "Welcome baby" skips straight to the birth.

use crate::actions;
use crate::model::{AppData, PartnerStatus, Pregnancy};
use crate::timeline::TERM_WEEKS;
use serde::{Deserialize, Serialize};

/// Earliest labor can start on its own (term).
pub const EARLY_WEEK: f64 = 37.0;
/// Latest labor starts on its own.
pub const LATE_WEEK: f64 = 42.0;
/// The avatar's Start button starts labor (held 2 s) from this week.
pub const START_BUTTON_WEEK: f64 = EARLY_WEEK;
/// How often the water breaks before labor gets going.
const WATER_FIRST_CHANCE: f64 = 0.15;
/// Once everything is ready, labor starts after a random wait in this range,
/// so it never starts while the avatar is still loading.
const START_DELAY_MS: (i64, i64) = (60_000, 5 * 60_000);
/// The partner still counts as here this long after leaving (world hops, crashes).
pub const PARTNER_GAP_MS: i64 = 2 * 60_000;
/// One contraction (FetalMovement holds the contraction code this long).
pub const CONTRACTION_MS: i64 = 30_000;
/// With "resting holds off labor": once due, the chance per second that
/// labor starts while you're up (about 4 minutes on average), and while
/// walking around (about 2).
const UP_START_PER_SEC: f64 = 1.0 / 240.0;
const WALKING_START_PER_SEC: f64 = 1.0 / 120.0;
/// A tick gap longer than this (sleep, a stalled engine) doesn't count as labor time.
const MAX_STEP_MS: i64 = 5_000;

/// When labor starts on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum LaborStart {
    /// A surprise day from week 37 to 42 (earlier for twins and more).
    #[default]
    Random,
    /// On the due date.
    DueDate,
    /// Never on its own: you start it.
    Manual,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LaborSettings {
    pub start: LaborStart,
    /// Start labor by this week if it hasn't started yet (None = off).
    pub induce_week: Option<u32>,
    /// Real minutes from the first contraction to the birth.
    pub minutes: u32,
    /// Labor doesn't start until the partner is in the instance.
    pub waits_for_partner: bool,
    /// A WooHoo from week 37 can start labor (see [`woohoo_chance`]).
    pub woohoo_starts: bool,
    /// Early labor shows nothing on the avatar (Labor stays 0, kicks carry
    /// on, no contractions): the first sign is the water breaking.
    pub quiet_early: bool,
    /// Labor only starts, and the water only breaks, while you're up
    /// (standing, not seated). Resting holds it off; Induce at still forces it.
    pub resting_holds: bool,
}

impl Default for LaborSettings {
    fn default() -> Self {
        Self { start: LaborStart::Random, induce_week: None, minutes: 20, waits_for_partner: false, woohoo_starts: false, quiet_early: false, resting_holds: false }
    }
}

/// Labor phase, as sent to the avatar's `Pregnancy/Labor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    /// Contractions getting closer.
    #[default]
    Early,
    /// The water broke: contractions every minute.
    Active,
    /// Almost there.
    Pushing,
}

impl Phase {
    pub fn value(self) -> i32 {
        match self {
            Phase::Early => 1,
            Phase::Active => 2,
            Phase::Pushing => 3,
        }
    }
}

/// `Pregnancy/Labor` while the belly goes down after the birth.
pub const BORN_VALUE: i32 = 4;

/// Labor in progress (kept on the pregnancy after the birth, for achievements).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Labor {
    pub phase: Phase,
    pub started_ms: i64,
    /// Labor time so far (only counts while it runs).
    pub elapsed_ms: i64,
    /// The water broke before labor got going.
    pub water_first: bool,
    pub water_broke_ms: Option<i64>,
    /// Waiting: not in-game, the pregnancy is paused, or this avatar can't show labor.
    pub paused: bool,
    /// The partner was here for the birth.
    pub partner_at_birth: bool,
    /// When the last tick counted (not saved: after a restart counting starts fresh).
    #[serde(skip)]
    last_ms: Option<i64>,
}

/// How labor time splits: early, active, pushing.
const SPLIT: [f64; 3] = [0.5, 0.3, 0.2];

fn total_ms(minutes: u32) -> i64 {
    minutes.max(1) as i64 * 60_000
}

/// Labor time when each phase ends.
fn ends_ms(minutes: u32) -> [i64; 3] {
    let t = total_ms(minutes) as f64;
    [(t * SPLIT[0]) as i64, (t * (SPLIT[0] + SPLIT[1])) as i64, t as i64]
}

fn phase_at(elapsed_ms: i64, minutes: u32) -> Phase {
    let e = ends_ms(minutes);
    if elapsed_ms < e[0] {
        Phase::Early
    } else if elapsed_ms < e[1] {
        Phase::Active
    } else {
        Phase::Pushing
    }
}

impl Labor {
    pub fn new(water_first: bool, minutes: u32, now_ms: i64) -> Self {
        // The water breaking first skips the early part.
        let elapsed_ms = if water_first { ends_ms(minutes)[0] } else { 0 };
        Self {
            phase: phase_at(elapsed_ms, minutes),
            started_ms: now_ms,
            elapsed_ms,
            water_first,
            water_broke_ms: water_first.then_some(now_ms),
            ..Default::default()
        }
    }

    /// Time until the next contraction starts: every 5 minutes, closing in to
    /// 2 early on, then every minute, a bit faster while pushing. `boost`
    /// (partner here) makes them come a little sooner.
    pub fn contraction_gap_ms(&self, minutes: u32, boost: bool) -> i64 {
        let e = ends_ms(minutes);
        let gap = match self.phase {
            Phase::Early => {
                let f = (self.elapsed_ms as f64 / e[0].max(1) as f64).clamp(0.0, 1.0);
                (5.0 - 3.0 * f) * 60_000.0
            }
            Phase::Active => 60_000.0,
            Phase::Pushing => 45_000.0,
        };
        let gap = if boost { gap * 0.8 } else { gap };
        (gap as i64).max(CONTRACTION_MS + 2_000)
    }
}

/// A 0..1 number from a pregnancy id: the same every run.
fn seeded(id: &str, salt: &str) -> f64 {
    // FNV-1a.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in id.bytes().chain(salt.bytes()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// The surprise labor week for a pregnancy: 37 to 42, most likely about 40,
/// earlier for twins and more. The same every run.
pub fn labor_week(p: &Pregnancy) -> f64 {
    // Triangular, peaking at 40.
    let (a, b, c) = (EARLY_WEEK, LATE_WEEK, 40.0);
    let u = seeded(&p.id, "labor");
    let k = (c - a) / (b - a);
    let w = if u < k { a + ((b - a) * (c - a) * u).sqrt() } else { b - ((b - a) * (b - c) * (1.0 - u)).sqrt() };
    let earlier = match p.baby_count {
        0 | 1 => 0.0,
        2 => 1.0,
        3 => 2.5,
        _ => 4.0,
    };
    w - earlier
}

/// Does the water break first in this pregnancy? (The same every run.)
pub fn water_first(p: &Pregnancy) -> bool {
    seeded(&p.id, "water") < WATER_FIRST_CHANCE
}

/// Labor is due by the settings (ignores whether you're in-game).
pub fn due(p: &Pregnancy, s: &LaborSettings, now_ms: i64) -> bool {
    let week = p.timeline.progress(now_ms) * TERM_WEEKS;
    let by_start = match s.start {
        LaborStart::Random => week >= labor_week(p),
        LaborStart::DueDate => week >= TERM_WEEKS,
        LaborStart::Manual => false,
    };
    by_start || induced(p, s, now_ms)
}

/// Past the Induce at week: labor starts even while resting.
fn induced(p: &Pregnancy, s: &LaborSettings, now_ms: i64) -> bool {
    s.induce_week.is_some_and(|w| p.timeline.week(now_ms) >= w as f64)
}

/// The avatar shows this labor phase (`Pregnancy/Labor`). Quiet early labor shows nothing.
pub fn shown(l: &Labor, s: &LaborSettings) -> bool {
    !(s.quiet_early && l.phase == Phase::Early)
}

/// Is the pregnancy's partner in the instance? `None` = can't tell (no
/// partner set, or the log isn't read).
pub fn partner_here(data: &AppData, p: &Pregnancy, now_ms: i64) -> Option<bool> {
    let partner = &p.partner;
    if partner.status == PartnerStatus::Unknown || (partner.name.is_none() && partner.user_id.is_none()) || !data.settings.read_logs {
        return None;
    }
    let same = |name: &str, id: Option<&str>| match (partner.user_id.as_deref(), id) {
        (Some(a), Some(b)) => a == b,
        _ => partner.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(name)),
    };
    let i = &data.instance;
    // You're your own partner: always here.
    if i.local.as_ref().is_some_and(|l| same(&l.name, l.user_id.as_deref())) {
        return Some(true);
    }
    if i.others().iter().any(|o| same(&o.name, o.user_id.as_deref())) {
        return Some(true);
    }
    // Left a moment ago (world hop, crash): still counts.
    let recent = i.history.iter().any(|h| same(&h.name, h.user_id.as_deref()) && now_ms - h.last_seen_ms < PARTNER_GAP_MS);
    Some(recent)
}

/// Chance a WooHoo starts labor: 25% at week 37, rising to 75% from week 40.
pub fn woohoo_chance(week: f64) -> f64 {
    if week < EARLY_WEEK {
        return 0.0;
    }
    (0.25 + (week - EARLY_WEEK) / 3.0 * 0.5).min(0.75)
}

/// A WooHoo while pregnant: starts labor if that's on, it's week 37 or
/// later, the partner is here (if labor waits for them), and `roll`
/// (random in [0, 1)) comes up. The caller checks the avatar can show
/// labor. Returns whether the water broke first.
pub fn woohoo(data: &mut AppData, preg_id: &str, now_ms: i64, roll: f64) -> Option<bool> {
    if !data.settings.labor.woohoo_starts {
        return None;
    }
    let p = data.pregnancy(preg_id).filter(|p| p.is_active() && p.labor.is_none() && !p.timeline.is_paused())?;
    if data.settings.labor.waits_for_partner && partner_here(data, p, now_ms) == Some(false) {
        return None;
    }
    if roll >= woohoo_chance(p.timeline.week(now_ms)) {
        return None;
    }
    start(data, preg_id, now_ms).then(|| data.pregnancy(preg_id).and_then(|p| p.labor.as_ref()).is_some_and(|l| l.water_first))
}

/// Starts labor now (the app's button or the avatar's Start button).
/// False when there's nothing to start.
pub fn start(data: &mut AppData, preg_id: &str, now_ms: i64) -> bool {
    let minutes = data.settings.labor.minutes;
    let Some(p) = data.pregnancy_mut(preg_id).filter(|p| p.is_active() && p.labor.is_none()) else { return false };
    p.labor = Some(Labor::new(water_first(p), minutes, now_ms));
    true
}

/// Debug: moves labor along. The next tick plays what that passes (the
/// water breaking, pushing, the birth), like it happened on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Skip {
    /// One more minute of labor.
    Minute,
    /// To the start of the next phase (or the birth from pushing).
    NextPhase,
    /// Straight to the birth.
    Birth,
}

/// Debug: moves labor on a pregnancy along. False if it isn't in labor.
pub fn skip(data: &mut AppData, preg_id: &str, how: Skip) -> bool {
    let minutes = data.settings.labor.minutes;
    let Some(l) = data.pregnancy_mut(preg_id).filter(|p| p.is_active()).and_then(|p| p.labor.as_mut()) else { return false };
    let ends = ends_ms(minutes);
    l.elapsed_ms = match how {
        Skip::Minute => l.elapsed_ms + 60_000,
        Skip::NextPhase => ends.into_iter().find(|&e| e > l.elapsed_ms).unwrap_or(ends[2]),
        Skip::Birth => ends[2],
    }
    .min(ends[2]);
    true
}

/// What happened this tick.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Started { water_first: bool },
    WaterBroke,
    Pushing,
    /// The pregnancy ended with a birth.
    Born { pregnancy_id: String },
}

/// What the engine knows about VRChat right now.
#[derive(Debug, Clone, Default)]
pub struct Ctx {
    /// In-game with OSC on.
    pub linked: bool,
    pub avatar_id: Option<String>,
    /// The avatar has loaded.
    pub avatar_ready: bool,
    /// The avatar can show labor (its parameter list has been read and has the labor parameter).
    pub supports: bool,
    /// VRChat says you're AFK: labor doesn't start or move.
    pub afk: bool,
    /// Standing up: not seated, not lying down (see "resting holds off labor").
    pub up: bool,
    /// Walking around.
    pub moving: bool,
}

/// Starts labor when it's due and moves it along.
#[derive(Debug, Default)]
pub struct Watch {
    /// Everything is ready: labor starts then (pregnancy id, when).
    ready_at: Option<(String, i64)>,
}

impl Watch {
    /// Returns what happened; `roll` is random in [0, 1) for the start delay.
    pub fn tick(&mut self, data: &mut AppData, ctx: &Ctx, now_ms: i64, roll: f64) -> Vec<Event> {
        let mut out = Vec::new();
        let current = data.current_pregnancy_id(ctx.avatar_id.as_deref()).filter(|id| data.pregnancy(id).is_some_and(|p| p.is_active()));
        let minutes = data.settings.labor.minutes;
        let can_run = ctx.linked && ctx.avatar_ready && ctx.supports && !ctx.afk;

        // Labor on a pregnancy you're not wearing waits.
        for p in data.pregnancies.iter_mut().filter(|p| p.is_active() && Some(&p.id) != current.as_ref()) {
            if let Some(l) = p.labor.as_mut() {
                l.paused = true;
                l.last_ms = None;
            }
        }
        let Some(id) = current else {
            self.ready_at = None;
            return out;
        };

        let partner = {
            let p = data.pregnancy(&id).unwrap();
            partner_here(data, p, now_ms)
        };
        let s = data.settings.labor.clone();
        let p = data.pregnancy_mut(&id).unwrap();
        let running = can_run && !p.timeline.is_paused();

        let Some(l) = p.labor.as_mut() else {
            // Not in labor yet: is it time?
            let partner_ok = !s.waits_for_partner || partner != Some(false);
            if !(running && partner_ok && due(p, &s, now_ms)) {
                self.ready_at = None;
                return out;
            }
            // Resting holds it off; while you're up it can start any moment.
            if s.resting_holds && !induced(p, &s, now_ms) {
                self.ready_at = None;
                let chance = if ctx.moving { WALKING_START_PER_SEC } else { UP_START_PER_SEC };
                if ctx.up && roll < chance {
                    let wf = water_first(p);
                    p.labor = Some(Labor::new(wf, minutes, now_ms));
                    out.push(Event::Started { water_first: wf });
                }
                return out;
            }
            let at = match &self.ready_at {
                Some((pid, at)) if *pid == id => *at,
                _ => {
                    let (lo, hi) = START_DELAY_MS;
                    let at = now_ms + lo + ((hi - lo) as f64 * roll) as i64;
                    self.ready_at = Some((id.clone(), at));
                    at
                }
            };
            if now_ms >= at {
                self.ready_at = None;
                let wf = water_first(p);
                p.labor = Some(Labor::new(wf, minutes, now_ms));
                out.push(Event::Started { water_first: wf });
            }
            return out;
        };

        self.ready_at = None;
        l.paused = !running;
        if !running {
            l.last_ms = None;
            return out;
        }
        let step = l.last_ms.map_or(0, |t| (now_ms - t).clamp(0, MAX_STEP_MS));
        l.last_ms = Some(now_ms);
        l.elapsed_ms += step;
        // Resting: early labor goes on, but the water doesn't break until you're up.
        if s.resting_holds && !ctx.up && l.phase == Phase::Early {
            l.elapsed_ms = l.elapsed_ms.min(ends_ms(minutes)[0] - 1);
        }
        let phase = phase_at(l.elapsed_ms, minutes);
        if phase > l.phase {
            if l.phase == Phase::Early {
                l.water_broke_ms = Some(now_ms);
                out.push(Event::WaterBroke);
            }
            if phase == Phase::Pushing {
                out.push(Event::Pushing);
            }
            l.phase = phase;
        }
        if l.elapsed_ms >= total_ms(minutes) {
            l.partner_at_birth = partner == Some(true);
            actions::give_birth(data, &id, now_ms);
            out.push(Event::Born { pregnancy_id: id });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::Person;
    use crate::model::Partner;
    use crate::timeline::{DurationMode, Timeline, DAY_MS};

    const T0: i64 = 1_700_000_000_000;
    const TERM: i64 = 280 * DAY_MS;

    fn data() -> AppData {
        let mut d = AppData::new();
        d.pregnancies.push(Pregnancy { id: "p".into(), timeline: Timeline::new(T0, DurationMode::Realtime, 1.0, None), ..Default::default() });
        d.active_pregnancy_id = Some("p".into());
        d.settings.labor.start = LaborStart::DueDate;
        d
    }

    fn ready() -> Ctx {
        Ctx { linked: true, avatar_id: Some("avtr_a".into()), avatar_ready: true, supports: true, afk: false, up: true, moving: false }
    }

    /// Ticks once a second from `from` for `secs` seconds, collecting events.
    fn run(w: &mut Watch, d: &mut AppData, ctx: &Ctx, from: i64, secs: i64) -> Vec<Event> {
        (0..secs).flat_map(|i| w.tick(d, ctx, from + i * 1000, 0.0)).collect()
    }

    #[test]
    fn labor_week_is_term_and_stable() {
        for i in 0..200 {
            let p = Pregnancy { id: format!("preg_{i}"), ..Default::default() };
            let w = labor_week(&p);
            assert!((EARLY_WEEK..=LATE_WEEK).contains(&w), "{w}");
            assert_eq!(w, labor_week(&p));
            let twins = Pregnancy { baby_count: 2, ..p };
            assert!((labor_week(&twins) - (w - 1.0)).abs() < 1e-9);
        }
        let firsts = (0..1000).filter(|i| water_first(&Pregnancy { id: format!("preg_{i}"), ..Default::default() })).count();
        assert!((100..200).contains(&firsts), "about 15%: {firsts}");
    }

    #[test]
    fn starts_after_a_wait_then_runs_to_a_birth() {
        let mut d = data();
        let mut w = Watch::default();
        let ctx = ready();
        assert!(run(&mut w, &mut d, &ctx, T0 + TERM - DAY_MS, 600).is_empty(), "not due yet");
        // Due: starts a minute later (roll 0), never right away.
        assert!(w.tick(&mut d, &ctx, T0 + TERM, 0.0).is_empty());
        let ev = run(&mut w, &mut d, &ctx, T0 + TERM + 1000, 60);
        assert!(matches!(ev.as_slice(), [Event::Started { .. }]), "{ev:?}");
        let l = d.pregnancy("p").unwrap().labor.clone().unwrap();
        // 20 minutes of labor: water, pushing, birth.
        let ev = run(&mut w, &mut d, &ctx, T0 + TERM + 70_000, 21 * 60);
        let p = d.pregnancy("p").unwrap();
        if l.water_first {
            assert!(matches!(ev.as_slice(), [Event::Pushing, Event::Born { .. }]), "{ev:?}");
        } else {
            assert!(matches!(ev.as_slice(), [Event::WaterBroke, Event::Pushing, Event::Born { .. }]), "{ev:?}");
        }
        assert!(p.born && p.ended_ms.is_some());
    }

    #[test]
    fn waits_for_vrchat_and_an_avatar_that_can_show_it() {
        let mut d = data();
        let mut w = Watch::default();
        let late = T0 + TERM + 30 * DAY_MS;
        for ctx in [Ctx { linked: false, ..ready() }, Ctx { supports: false, ..ready() }, Ctx { avatar_ready: false, ..ready() }, Ctx { afk: true, ..ready() }] {
            assert!(run(&mut w, &mut d, &ctx, late, 600).is_empty());
            assert!(d.pregnancy("p").unwrap().labor.is_none());
        }
        // Manual: never on its own, even long overdue.
        d.settings.labor.start = LaborStart::Manual;
        assert!(run(&mut w, &mut d, &ready(), late, 600).is_empty());
        // ...unless induced.
        d.settings.labor.induce_week = Some(42);
        assert!(!run(&mut w, &mut d, &ready(), late, 600).is_empty());
    }

    #[test]
    fn switching_to_an_avatar_without_labor_pauses_it() {
        let mut d = data();
        let mut w = Watch::default();
        assert!(start(&mut d, "p", T0));
        assert!(!start(&mut d, "p", T0), "only once");
        run(&mut w, &mut d, &ready(), T0, 60);
        let at = d.pregnancy("p").unwrap().labor.as_ref().unwrap().elapsed_ms;
        let other = Ctx { supports: false, ..ready() };
        run(&mut w, &mut d, &other, T0 + 60_000, 3600);
        let l = d.pregnancy("p").unwrap().labor.clone().unwrap();
        assert!(l.paused && l.elapsed_ms == at, "frozen");
        run(&mut w, &mut d, &ready(), T0 + 4_000_000, 5);
        let l = d.pregnancy("p").unwrap().labor.clone().unwrap();
        assert!(!l.paused && l.elapsed_ms > at, "picks up again");
    }

    fn with_partner(d: &mut AppData) {
        d.pregnancies[0].partner = Partner { status: PartnerStatus::Confirmed, name: Some("Fox".into()), user_id: Some("usr_fox".into()), ..Default::default() };
        d.instance.local = Some(Person { name: "Me".into(), user_id: Some("usr_me".into()), avatar_name: None, last_seen_ms: 0, world_name: None });
    }

    fn fox(at: i64) -> Person {
        Person { name: "Fox".into(), user_id: Some("usr_fox".into()), avatar_name: None, last_seen_ms: at, world_name: None }
    }

    #[test]
    fn partner_here_counts_short_gaps() {
        let mut d = data();
        assert_eq!(partner_here(&d, &d.pregnancies[0], T0), None, "no partner set");
        with_partner(&mut d);
        assert_eq!(partner_here(&d, &d.pregnancies[0], T0), Some(false));
        d.instance.present.push(fox(T0));
        assert_eq!(partner_here(&d, &d.pregnancies[0], T0), Some(true));
        d.instance.present.clear();
        d.instance.history.push(fox(T0));
        assert_eq!(partner_here(&d, &d.pregnancies[0], T0 + 60_000), Some(true), "just left");
        assert_eq!(partner_here(&d, &d.pregnancies[0], T0 + PARTNER_GAP_MS), Some(false));
        d.settings.read_logs = false;
        assert_eq!(partner_here(&d, &d.pregnancies[0], T0), None);
    }

    #[test]
    fn waits_for_the_partner_then_keeps_going_without_them() {
        let mut d = data();
        with_partner(&mut d);
        d.settings.labor.waits_for_partner = true;
        let mut w = Watch::default();
        let due = T0 + TERM;
        assert!(run(&mut w, &mut d, &ready(), due, 600).is_empty(), "waiting for Fox");
        d.instance.present.push(fox(due));
        let ev = run(&mut w, &mut d, &ready(), due + 600_000, 120);
        assert!(matches!(ev.first(), Some(Event::Started { .. })));
        // Fox leaves: labor carries on to the birth.
        d.instance.present.clear();
        let ev = run(&mut w, &mut d, &ready(), due + 720_000, 21 * 60);
        assert!(matches!(ev.last(), Some(Event::Born { .. })));
        assert!(!d.pregnancy("p").unwrap().labor.as_ref().unwrap().partner_at_birth);
    }

    #[test]
    fn a_woohoo_can_start_labor_late_on() {
        assert_eq!(woohoo_chance(36.9), 0.0);
        assert!((woohoo_chance(37.0) - 0.25).abs() < 1e-9);
        assert!((woohoo_chance(38.5) - 0.5).abs() < 1e-9);
        assert_eq!(woohoo_chance(41.0), 0.75);
        let mut d = data();
        let w38 = T0 + 38 * 7 * DAY_MS;
        assert_eq!(woohoo(&mut d, "p", w38, 0.0), None, "off by default");
        d.settings.labor.woohoo_starts = true;
        assert_eq!(woohoo(&mut d, "p", T0 + 30 * 7 * DAY_MS, 0.0), None, "too early");
        assert_eq!(woohoo(&mut d, "p", w38, 0.9), None, "unlucky roll");
        assert!(woohoo(&mut d, "p", w38, 0.1).is_some());
        assert!(d.pregnancies[0].labor.is_some());
        assert_eq!(woohoo(&mut d, "p", w38, 0.1), None, "already in labor");
    }

    #[test]
    fn a_woohoo_waits_for_the_partner_too() {
        let mut d = data();
        with_partner(&mut d);
        d.settings.labor.woohoo_starts = true;
        d.settings.labor.waits_for_partner = true;
        let w38 = T0 + 38 * 7 * DAY_MS;
        assert_eq!(woohoo(&mut d, "p", w38, 0.0), None, "Fox isn't here");
        d.instance.present.push(fox(w38));
        assert!(woohoo(&mut d, "p", w38, 0.0).is_some());
    }

    #[test]
    fn resting_holds_off_labor_and_the_water() {
        let mut d = data();
        d.settings.labor.resting_holds = true;
        let mut w = Watch::default();
        let due = T0 + TERM;
        let resting = Ctx { up: false, ..ready() };
        // Resting: never starts, even with every roll coming up.
        assert!((0..3600).all(|i| w.tick(&mut d, &resting, due + i * 1000, 0.0).is_empty()));
        // Up: a lucky roll starts it, an unlucky one doesn't.
        assert!(w.tick(&mut d, &ready(), due + 4_000_000, 0.5).is_empty());
        assert!(!w.tick(&mut d, &ready(), due + 4_001_000, 0.0).is_empty());
        let l = d.pregnancies[0].labor.clone().unwrap();
        if l.water_first {
            return; // nothing left to hold back
        }
        // Early labor while resting: stuck before the water breaks.
        let ev = run(&mut w, &mut d, &resting, due + 4_002_000, 3600);
        assert!(ev.is_empty(), "{ev:?}");
        assert_eq!(d.pregnancies[0].labor.as_ref().unwrap().phase, Phase::Early);
        // Up again: the water breaks.
        let ev = run(&mut w, &mut d, &ready(), due + 8_000_000, 5);
        assert_eq!(ev, vec![Event::WaterBroke]);
    }

    #[test]
    fn induce_beats_resting() {
        let mut d = data();
        d.settings.labor.resting_holds = true;
        d.settings.labor.start = LaborStart::Manual;
        d.settings.labor.induce_week = Some(42);
        let mut w = Watch::default();
        let resting = Ctx { up: false, ..ready() };
        let ev = run(&mut w, &mut d, &resting, T0 + 42 * 7 * DAY_MS, 400);
        assert!(matches!(ev.first(), Some(Event::Started { .. })));
    }

    #[test]
    fn quiet_early_labor_shows_nothing_until_the_water_breaks() {
        let s = LaborSettings { quiet_early: true, ..Default::default() };
        let early = Labor::new(false, 20, 0);
        assert!(!shown(&early, &s) && shown(&early, &LaborSettings::default()));
        assert!(shown(&Labor { phase: Phase::Active, ..early }, &s));
    }

    #[test]
    fn debug_skips_play_out_on_the_next_tick() {
        let mut d = data();
        let mut w = Watch::default();
        assert!(!skip(&mut d, "p", Skip::Minute), "not in labor");
        start(&mut d, "p", T0);
        d.pregnancies[0].labor.as_mut().unwrap().water_first = false;
        d.pregnancies[0].labor.as_mut().unwrap().elapsed_ms = 0;
        d.pregnancies[0].labor.as_mut().unwrap().phase = Phase::Early;
        w.tick(&mut d, &ready(), T0, 0.0);
        assert!(skip(&mut d, "p", Skip::Minute));
        assert_eq!(d.pregnancies[0].labor.as_ref().unwrap().elapsed_ms, 60_000);
        skip(&mut d, "p", Skip::NextPhase);
        assert_eq!(w.tick(&mut d, &ready(), T0 + 1000, 0.0), vec![Event::WaterBroke]);
        skip(&mut d, "p", Skip::NextPhase);
        assert_eq!(w.tick(&mut d, &ready(), T0 + 2000, 0.0), vec![Event::Pushing]);
        skip(&mut d, "p", Skip::Birth);
        assert!(matches!(w.tick(&mut d, &ready(), T0 + 3000, 0.0).as_slice(), [Event::Born { .. }]));
        assert!(d.pregnancies[0].born);
    }

    #[test]
    fn contractions_close_in() {
        let early = Labor::new(false, 20, 0);
        let later = Labor { elapsed_ms: 9 * 60_000, ..early.clone() };
        assert_eq!(early.contraction_gap_ms(20, false), 5 * 60_000);
        assert!(later.contraction_gap_ms(20, false) < 2 * 60_000 + 20_000);
        assert!(early.contraction_gap_ms(20, true) < early.contraction_gap_ms(20, false));
        let pushing = Labor { phase: Phase::Pushing, ..early };
        assert!(pushing.contraction_gap_ms(20, true) > CONTRACTION_MS);
    }
}
