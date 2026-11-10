//! Settings > Troubleshooting: problems in the save (broken links between
//! WooHoos, waits and pregnancies, from old versions or a crash) and in the
//! settings, each with ways to fix it. `problems` lists them; `fix` finds one
//! again by its id and applies the fix picked.

use crate::model::*;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// Things only the app knows (not in the save).
#[derive(Debug, Clone, Copy, Default)]
pub struct Env {
    /// The log folder chosen by hand isn't there.
    pub log_dir_missing: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    /// Stable while the problem lasts (for `fix`).
    pub id: String,
    pub text: String,
    /// When it happened, if it's about one moment.
    pub at_ms: Option<i64>,
    /// The fixes, in order: their index is the fix id.
    pub fixes: Vec<String>,
    /// A tab that helps (e.g. "partners"), shown as a button.
    pub go: Option<&'static str>,
}

type Action = Box<dyn Fn(&mut AppData)>;

struct Found {
    problem: Problem,
    actions: Vec<Action>,
}

fn found(id: String, text: String, at_ms: Option<i64>, fixes: Vec<(&str, Action)>) -> Found {
    let (labels, actions) = fixes.into_iter().map(|(l, a)| (l.to_string(), a)).unzip();
    Found { problem: Problem { id, text, at_ms, fixes: labels, go: None }, actions }
}

pub fn problems(data: &AppData, env: Env) -> Vec<Problem> {
    all(data, env).into_iter().map(|f| f.problem).collect()
}

/// Applies fix `fix` of problem `id`. False if it's gone (already fixed).
pub fn fix(data: &mut AppData, env: Env, id: &str, fix: usize) -> bool {
    let Some(f) = all(data, env).into_iter().find(|f| f.problem.id == id) else { return false };
    let Some(action) = f.actions.get(fix) else { return false };
    action(data);
    true
}

// ---- How the save links things (actions.rs record_fun / resolve_due) ----
// A try is one wait: its WooHoos share wait_ms (the first one's time). While it
// runs it's an entry in `waiting`. When it ends, each WooHoo gets `took` (true
// on the one that took) and a pregnancy starts with conceived_ms = that time.

fn wait_times(w: &PendingConception) -> BTreeSet<i64> {
    let mut t: BTreeSet<i64> = w.fun_times.iter().copied().collect();
    t.insert(w.time_ms);
    if let Some(l) = &w.later {
        t.insert(l.time_ms);
    }
    t
}

fn in_a_wait(d: &AppData, time_ms: i64) -> bool {
    d.waiting.iter().any(|w| wait_times(w).contains(&time_ms))
}

fn not_a_try(f: &FunEvent) -> bool {
    f.already_pregnant || f.not_partner || f.protected
}

/// Finished tries: their WooHoos' times (sorted) by their key.
fn done_tries(d: &AppData) -> Vec<Vec<i64>> {
    let mut groups: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
    for f in &d.fun_log {
        if not_a_try(f) || in_a_wait(d, f.time_ms) {
            continue;
        }
        let key = f.wait_ms.or(f.took.map(|_| f.time_ms));
        if let Some(k) = key {
            groups.entry(k).or_default().push(f.time_ms);
        }
    }
    groups.into_values().map(|mut v| { v.sort(); v }).collect()
}

fn fun_at(d: &AppData, t: i64) -> Option<&FunEvent> {
    d.fun_log.iter().find(|f| f.time_ms == t)
}

fn each_fun(d: &mut AppData, times: &[i64], mut f: impl FnMut(&mut FunEvent)) {
    for e in d.fun_log.iter_mut().filter(|e| times.contains(&e.time_ms)) {
        f(e);
    }
}

fn preg_from(d: &AppData, t: i64) -> Option<&Pregnancy> {
    d.pregnancies.iter().find(|p| p.conceived_ms == Some(t))
}

fn group_of<'a>(d: &'a AppData, a: &'a str) -> &'a str {
    d.avatars.get(a).and_then(|p| p.group_id.as_deref()).unwrap_or(a)
}

/// The same avatar (or group), or either not known.
fn same_avatar(d: &AppData, a: Option<&str>, b: Option<&str>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a == b || group_of(d, a) == group_of(d, b) || group_of(d, a) == b || a == group_of(d, b),
        _ => true,
    }
}

/// The pregnancy a WooHoo happened during (for "Already pregnant" ones).
fn preg_during<'a>(d: &'a AppData, f: &FunEvent) -> Option<&'a Pregnancy> {
    d.pregnancies.iter().find(|p| f.time_ms >= p.timeline.start_ms && f.time_ms <= p.ended_ms.unwrap_or(i64::MAX) && same_avatar(d, p.avatar_id.as_deref(), f.avatar_id.as_deref()))
}

/// A try's result: Some(took) = that one took, None = none did. Like resolve_due.
fn set_outcome(d: &mut AppData, times: &[i64], took: Option<i64>) {
    let first = times.iter().copied().min().unwrap_or(0);
    each_fun(d, times, |f| {
        f.already_pregnant = false;
        f.not_partner = false;
        f.protected = false;
        f.wait_ms = Some(first);
        f.took = Some(Some(f.time_ms) == took);
    });
}

/// Removes WooHoos, and their times from any wait (an empty wait goes).
fn remove_woohoos(d: &mut AppData, times: &[i64]) {
    d.fun_log.retain(|f| !times.contains(&f.time_ms));
    drop_wait_times(d, times);
}

fn drop_wait_times(d: &mut AppData, times: &[i64]) {
    for w in &mut d.waiting {
        w.fun_times.retain(|t| !times.contains(t));
        if w.later.as_ref().is_some_and(|l| times.contains(&l.time_ms)) {
            w.later = None;
        }
        if times.contains(&w.time_ms) {
            if let Some(&m) = w.fun_times.iter().min() {
                w.time_ms = m;
            }
        }
    }
    d.waiting.retain(|w| !times.contains(&w.time_ms));
}

fn quoted(p: &Pregnancy) -> String {
    format!("\u{201c}{}\u{201d}", if p.name.is_empty() { &p.id } else { &p.name })
}

fn partner_named(name: Option<String>, user_id: Option<String>, candidates: Vec<crate::instance::Person>) -> Partner {
    match name {
        Some(n) => Partner { status: PartnerStatus::Guessed, name: Some(n), user_id, candidates, ..Default::default() },
        None => Partner { candidates, ..Default::default() },
    }
}

fn all(d: &AppData, env: Env) -> Vec<Found> {
    let mut out = settings_problems(d, env);
    tries_problems(d, &mut out);
    fun_problems(d, &mut out);
    pregnancy_problems(d, &mut out);
    leftovers(d, &mut out);
    out
}

fn settings_problems(d: &AppData, env: Env) -> Vec<Found> {
    let mut out = Vec::new();
    let s = &d.settings;
    if s.partners_only && d.my_partners.is_empty() {
        let mut f = found(
            "partners-none".into(),
            "Only my partners is on, but you have no partners, so no WooHoo can start a pregnancy.".into(),
            None,
            vec![("Turn off Only my partners", Box::new(|d: &mut AppData| d.settings.partners_only = false))],
        );
        f.problem.go = Some("partners");
        out.push(f);
    }
    if s.partners_only && !s.read_logs {
        out.push(found(
            "partners-logs".into(),
            "Only my partners needs log reading to know who it was. With it off, no WooHoo counts.".into(),
            None,
            vec![
                ("Turn on log reading", Box::new(|d: &mut AppData| d.settings.read_logs = true)),
                ("Turn off Only my partners", Box::new(|d: &mut AppData| d.settings.partners_only = false)),
            ],
        ));
    }
    if env.log_dir_missing && s.read_logs {
        out.push(found(
            "log-dir".into(),
            "The VRChat log folder you chose isn't there any more.".into(),
            None,
            vec![("Find it automatically", Box::new(|d: &mut AppData| d.settings.log_dir = None))],
        ));
    }
    out
}

fn tries_problems(d: &AppData, out: &mut Vec<Found>) {
    for times in done_tries(d) {
        let first = times[0];
        let tooks: Vec<i64> = times.iter().copied().filter(|&t| fun_at(d, t).is_some_and(|f| f.took == Some(true))).collect();
        if tooks.len() > 1 {
            let keep = tooks.iter().copied().find(|&t| preg_from(d, t).is_some()).unwrap_or(tooks[0]);
            let others: Vec<i64> = tooks.iter().copied().filter(|&t| t != keep).collect();
            out.push(found(
                format!("tooks:{first}"),
                format!("A try has {} WooHoos marked Pregnant. Only one can be.", tooks.len()),
                Some(first),
                vec![("Keep one, the rest Not this time", Box::new(move |d: &mut AppData| each_fun(d, &others, |f| f.took = Some(false))))],
            ));
        }
        if let Some(&took) = tooks.first().filter(|_| tooks.len() == 1) {
            if preg_from(d, took).is_none() {
                let f = fun_at(d, took).unwrap();
                // A pregnancy started by hand soon after it (same avatar) is most likely the one.
                let near = d
                    .pregnancies
                    .iter()
                    .filter(|p| p.conceived_ms.is_none() && p.timeline.start_ms >= took - 3_600_000 && p.timeline.start_ms <= took + 7 * 86_400_000)
                    .filter(|p| same_avatar(d, p.avatar_id.as_deref(), f.avatar_id.as_deref()))
                    .min_by_key(|p| (p.timeline.start_ms - took).abs());
                let mut fixes: Vec<(String, Action)> = Vec::new();
                if let Some(p) = near {
                    let id = p.id.clone();
                    fixes.push((
                        format!("It started {}", quoted(p)),
                        Box::new(move |d: &mut AppData| {
                            if let Some(p) = d.pregnancy_mut(&id) {
                                p.conceived_ms = Some(took);
                                p.source = PregnancySource::Conception;
                            }
                        }),
                    ));
                }
                let all = times.clone();
                fixes.push(("Mark the try Not this time".into(), Box::new(move |d: &mut AppData| set_outcome(d, &all, None))));
                let (labels, actions) = fixes.into_iter().unzip();
                out.push(Found {
                    problem: Problem { id: format!("nopreg:{took}"), text: "A WooHoo marked Pregnant didn't start any pregnancy.".into(), at_ms: Some(took), fixes: labels, go: None },
                    actions,
                });
            }
        }
        // Every WooHoo in a try names its first one (wait_ms).
        if times.iter().any(|&t| fun_at(d, t).is_some_and(|f| f.wait_ms != Some(first))) {
            let none = times.iter().all(|&t| fun_at(d, t).is_some_and(|f| f.wait_ms.is_none()));
            let all = times.clone();
            out.push(found(
                format!("waitms:{first}"),
                if none { "A WooHoo has a result but no try.".into() } else { "A try's WooHoos point at a first WooHoo that's gone.".into() },
                Some(first),
                vec![(if times.len() > 1 { "Point them at the first one" } else { "Make it a try of its own" }, Box::new(move |d: &mut AppData| each_fun(d, &all, |f| f.wait_ms = Some(first))))],
            ));
        }
    }
}

fn fun_problems(d: &AppData, out: &mut Vec<Found>) {
    for f in &d.fun_log {
        let t = f.time_ms;
        if f.already_pregnant && preg_during(d, f).is_none() {
            out.push(found(
                format!("already:{t}"),
                "An \u{201c}Already pregnant\u{201d} WooHoo isn't during any pregnancy.".into(),
                Some(t),
                vec![
                    ("Remove it", Box::new(move |d: &mut AppData| remove_woohoos(d, &[t]))),
                    ("Make it No result", Box::new(move |d: &mut AppData| each_fun(d, &[t], |f| f.already_pregnant = false))),
                ],
            ));
        }
        if not_a_try(f) && (f.took.is_some() || f.wait_ms.is_some()) {
            let what = if f.already_pregnant {
                "An \u{201c}Already pregnant\u{201d}"
            } else if f.not_partner {
                "A \u{201c}Not a partner\u{201d}"
            } else {
                "A \u{201c}Protected\u{201d}"
            };
            out.push(found(format!("notry:{t}"), format!("{what} WooHoo also has a try result."), Some(t), vec![("Clear the result", clear(t))]));
        } else if in_a_wait(d, t) && (f.took.is_some() || f.wait_ms.is_some()) {
            out.push(found(format!("waiting:{t}"), "A WooHoo still waiting to find out already has a result.".into(), Some(t), vec![("Clear the result", clear(t))]));
        }
    }
    for w in &d.waiting {
        let missing: Vec<i64> = wait_times(w).into_iter().filter(|&t| fun_at(d, t).is_none()).collect();
        if missing.is_empty() {
            continue;
        }
        let wt = w.time_ms;
        let n = missing.len();
        out.push(found(
            format!("wait:{wt}"),
            format!("A wait to find out points at {n} WooHoo{} gone.", if n > 1 { "s that are" } else { " that's" }),
            Some(wt),
            vec![
                ("Drop the missing ones", Box::new(move |d: &mut AppData| drop_wait_times(d, &missing))),
                ("Drop the wait", Box::new(move |d: &mut AppData| d.waiting.retain(|w| w.time_ms != wt))),
            ],
        ));
    }
}

fn clear(t: i64) -> Action {
    Box::new(move |d: &mut AppData| {
        each_fun(d, &[t], |f| {
            f.took = None;
            f.wait_ms = None;
        })
    })
}

fn unlink(id: String) -> Action {
    Box::new(move |d: &mut AppData| {
        if let Some(p) = d.pregnancy_mut(&id) {
            p.source = PregnancySource::Manual;
            p.conceived_ms = None;
        }
    })
}

fn pregnancy_problems(d: &AppData, out: &mut Vec<Found>) {
    for p in &d.pregnancies {
        let Some(t) = p.conceived_ms else { continue };
        let id = p.id.clone();
        match fun_at(d, t) {
            None => {
                let (name, user_id) = (p.partner.name.clone(), p.partner.user_id.clone());
                let (avatar_id, world_name) = (p.avatar_id.clone(), p.world_name.clone());
                out.push(found(
                    format!("preg-gone:{id}"),
                    format!("{} came from a WooHoo that's gone.", quoted(p)),
                    Some(t),
                    vec![
                        (
                            "Add the WooHoo back",
                            Box::new(move |d: &mut AppData| {
                                d.fun_log.push(FunEvent { time_ms: t, partner_name: name.clone(), partner_user_id: user_id.clone(), avatar_id: avatar_id.clone(), world_name: world_name.clone(), took: Some(true), wait_ms: Some(t), ..Default::default() });
                                d.fun_log.sort_by_key(|f| f.time_ms);
                            }),
                        ),
                        ("Make it a pregnancy started by hand", unlink(id.clone())),
                    ],
                ));
            }
            Some(f) if f.took != Some(true) => {
                let try_times = done_tries(d).into_iter().find(|v| v.contains(&t));
                out.push(found(
                    format!("preg-unmarked:{id}"),
                    format!("{} came from a WooHoo that isn't marked Pregnant.", quoted(p)),
                    Some(t),
                    vec![
                        (
                            "Mark that WooHoo Pregnant",
                            Box::new(move |d: &mut AppData| match &try_times {
                                Some(v) => set_outcome(d, v, Some(t)),
                                None => set_outcome(d, &[t], Some(t)),
                            }),
                        ),
                        ("Make it a pregnancy started by hand", unlink(id.clone())),
                    ],
                ));
            }
            Some(f) if f.partner_name != p.partner.name => {
                let (pn, pu) = (p.partner.name.clone(), p.partner.user_id.clone());
                let (fun_name, fun_user) = (f.partner_name.clone(), f.partner_user_id.clone());
                let id2 = id.clone();
                out.push(found(
                    format!("preg-partner:{id}"),
                    format!("{} and the WooHoo it came from name different people.", quoted(p)),
                    Some(t),
                    vec![
                        (
                            "Use the pregnancy's",
                            Box::new(move |d: &mut AppData| {
                                each_fun(d, &[t], |f| {
                                    f.partner_name = pn.clone();
                                    f.partner_user_id = pu.clone();
                                })
                            }),
                        ),
                        (
                            "Use the WooHoo's",
                            Box::new(move |d: &mut AppData| {
                                if let Some(p) = d.pregnancy_mut(&id2) {
                                    let candidates = std::mem::take(&mut p.partner.candidates);
                                    p.partner = partner_named(fun_name.clone(), fun_user.clone(), candidates);
                                }
                            }),
                        ),
                    ],
                ));
            }
            Some(_) => {}
        }
    }
    // Stopped before stopping removed them (actions::end_pregnancy).
    let stopped: Vec<(String, i64)> = d.pregnancies.iter().filter(|p| p.stopped()).map(|p| (p.id.clone(), p.ended_ms.unwrap_or(0))).collect();
    if !stopped.is_empty() {
        let n = stopped.len();
        out.push(found(
            "stopped".into(),
            if n > 1 { format!("{n} stopped pregnancies are still saved.") } else { "A stopped pregnancy is still saved.".into() },
            None,
            vec![(
                if n > 1 { "Remove them, as if they never happened" } else { "Remove it, as if it never happened" },
                Box::new(move |d: &mut AppData| {
                    for (id, ended) in &stopped {
                        crate::actions::end_pregnancy(d, id, *ended);
                    }
                }),
            )],
        ));
    }
    for p in d.pregnancies.iter().filter(|p| p.born && p.ended_ms.is_none()) {
        let id = p.id.clone();
        out.push(found(
            format!("born:{id}"),
            format!("{} is marked born but never ended.", quoted(p)),
            Some(p.timeline.start_ms),
            vec![(
                "Not born yet",
                Box::new(move |d: &mut AppData| {
                    if let Some(p) = d.pregnancy_mut(&id) {
                        p.born = false;
                    }
                }),
            )],
        ));
    }
}

fn leftovers(d: &AppData, out: &mut Vec<Found>) {
    let ids: BTreeSet<&str> = d.pregnancies.iter().map(|p| p.id.as_str()).collect();
    let gone = |id: &Option<String>| id.as_deref().is_some_and(|i| !ids.contains(i));
    let j = d.journal.iter().filter(|j| gone(&j.pregnancy_id)).count();
    if j > 0 {
        out.push(found(
            "journal".into(),
            format!("{j} journal entr{} for a pregnancy that's gone.", if j > 1 { "ies are" } else { "y is" }),
            None,
            vec![(
                "Remove them",
                Box::new(|d: &mut AppData| {
                    let ids: BTreeSet<String> = d.pregnancies.iter().map(|p| p.id.clone()).collect();
                    d.journal.retain(|j| j.pregnancy_id.as_ref().is_none_or(|i| ids.contains(i)));
                }),
            )],
        ));
    }
    let k = d.keepsakes.iter().filter(|k| !ids.contains(k.pregnancy_id.as_str())).count();
    if k > 0 {
        out.push(found(
            "keepsakes".into(),
            format!("{k} keepsake{} for a pregnancy that's gone.", if k > 1 { "s are" } else { " is" }),
            None,
            vec![(
                "Remove them",
                Box::new(|d: &mut AppData| {
                    let ids: BTreeSet<String> = d.pregnancies.iter().map(|p| p.id.clone()).collect();
                    d.keepsakes.retain(|k| ids.contains(&k.pregnancy_id));
                }),
            )],
        ));
    }
    let n = gone(&d.active_pregnancy_id) as usize + d.avatars.values().filter(|a| gone(&a.pregnancy_id)).count() + d.groups.values().filter(|g| gone(&g.pregnancy_id)).count();
    if n > 0 {
        out.push(found(
            "pointers".into(),
            format!("{n} avatar{} still show{} a pregnancy that's gone.", if n > 1 { "s" } else { "" }, if n > 1 { "" } else { "s" }),
            None,
            vec![(
                "Clear them",
                Box::new(|d: &mut AppData| {
                    let ids: BTreeSet<String> = d.pregnancies.iter().map(|p| p.id.clone()).collect();
                    let keep = |id: &mut Option<String>| {
                        if id.as_ref().is_some_and(|i| !ids.contains(i)) {
                            *id = None;
                        }
                    };
                    keep(&mut d.active_pregnancy_id);
                    d.avatars.values_mut().for_each(|a| keep(&mut a.pregnancy_id));
                    d.groups.values_mut().for_each(|g| keep(&mut g.pregnancy_id));
                }),
            )],
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{record_fun, resolve_due, set_my_partner, start_pregnancy, NewPregnancy};

    fn ids(d: &AppData) -> Vec<String> {
        problems(d, Env::default()).into_iter().map(|p| p.id).collect()
    }

    fn fun(t: i64) -> FunEvent {
        FunEvent { time_ms: t, partner_name: Some("Fox".into()), ..Default::default() }
    }

    #[test]
    fn a_healthy_save_has_none() {
        let mut d = AppData::new();
        d.settings.fertility = true;
        record_fun(&mut d, 1000, 0.0);
        resolve_due(&mut d, i64::MAX / 2);
        record_fun(&mut d, i64::MAX / 2 + 1, 0.0);
        start_pregnancy(&mut d, NewPregnancy::default(), 5);
        assert!(ids(&d).is_empty(), "{:?}", problems(&d, Env::default()));
    }

    #[test]
    fn stopped_pregnancies_from_before_can_go() {
        let mut d = AppData::new();
        let id = start_pregnancy(&mut d, NewPregnancy::default(), 0);
        crate::achievements::check(&mut d, 5);
        d.pregnancy_mut(&id).unwrap().ended_ms = Some(10);
        assert_eq!(ids(&d), ["stopped"]);
        assert!(fix(&mut d, Env::default(), "stopped", 0));
        assert!(d.pregnancies.is_empty() && d.journal.is_empty());
        assert!(!d.achievements.contains_key("little_bean"), "what it earned is locked again");
        assert!(ids(&d).is_empty());
    }

    #[test]
    fn only_my_partners_with_nobody() {
        let mut d = AppData::new();
        d.settings.partners_only = true;
        let p = problems(&d, Env::default());
        assert_eq!(p[0].id, "partners-none");
        assert_eq!(p[0].go, Some("partners"));
        set_my_partner(&mut d, "Fox", None, true, 0);
        assert!(ids(&d).is_empty());
        d.settings.read_logs = false;
        assert!(fix(&mut d, Env::default(), "partners-logs", 0));
        assert!(d.settings.read_logs);
        d.settings.log_dir = Some("X:/gone".into());
        assert!(fix(&mut d, Env { log_dir_missing: true }, "log-dir", 0));
        assert!(d.settings.log_dir.is_none());
    }

    #[test]
    fn two_tooks_keep_the_one_with_the_pregnancy() {
        let mut d = AppData::new();
        d.fun_log = vec![FunEvent { took: Some(true), wait_ms: Some(10), ..fun(10) }, FunEvent { took: Some(true), wait_ms: Some(10), ..fun(20) }];
        d.pregnancies.push(Pregnancy { id: "p".into(), conceived_ms: Some(20), source: PregnancySource::Conception, partner: Partner { name: Some("Fox".into()), ..Default::default() }, ..Default::default() });
        assert_eq!(ids(&d), ["tooks:10"]);
        assert!(fix(&mut d, Env::default(), "tooks:10", 0));
        assert_eq!(d.fun_log.iter().map(|f| f.took).collect::<Vec<_>>(), [Some(false), Some(true)]);
        assert!(ids(&d).is_empty());
        assert!(!fix(&mut d, Env::default(), "tooks:10", 0), "already fixed");
    }

    #[test]
    fn a_took_with_no_pregnancy_finds_the_one_started_by_hand() {
        let mut d = AppData::new();
        d.fun_log = vec![FunEvent { took: Some(true), wait_ms: Some(10), ..fun(10) }];
        d.pregnancies.push(Pregnancy { id: "p".into(), name: "Bean".into(), timeline: crate::timeline::Timeline::new(60_000, crate::timeline::DurationMode::Realtime, 1.0, None), partner: Partner { name: Some("Fox".into()), ..Default::default() }, ..Default::default() });
        let p = problems(&d, Env::default());
        assert_eq!(p[0].fixes[0], "It started \u{201c}Bean\u{201d}");
        assert!(fix(&mut d, Env::default(), "nopreg:10", 0));
        assert_eq!(d.pregnancies[0].conceived_ms, Some(10));
        assert!(ids(&d).is_empty());
    }

    #[test]
    fn a_pregnancy_whose_woohoo_is_gone() {
        let mut d = AppData::new();
        d.pregnancies.push(Pregnancy { id: "p".into(), conceived_ms: Some(10), source: PregnancySource::Conception, ..Default::default() });
        assert_eq!(ids(&d), ["preg-gone:p"]);
        assert!(fix(&mut d, Env::default(), "preg-gone:p", 0));
        assert_eq!(d.fun_log[0].took, Some(true));
        assert!(ids(&d).is_empty());
    }

    #[test]
    fn leftovers_and_not_a_try_results() {
        let mut d = AppData::new();
        d.active_pregnancy_id = Some("gone".into());
        d.journal.push(JournalEntry { pregnancy_id: Some("gone".into()), ..Default::default() });
        d.fun_log.push(FunEvent { not_partner: true, took: Some(false), wait_ms: Some(5), ..fun(5) });
        d.waiting.push(PendingConception { time_ms: 7, ..Default::default() });
        let mut got = ids(&d);
        got.sort();
        assert_eq!(got, ["journal", "notry:5", "pointers", "wait:7"]);
        for id in got {
            assert!(fix(&mut d, Env::default(), &id, 0), "{id}");
        }
        assert!(ids(&d).is_empty());
        assert!(d.journal.is_empty() && d.active_pregnancy_id.is_none() && d.waiting.is_empty());
    }
}
