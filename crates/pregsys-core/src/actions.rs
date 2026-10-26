//! Data changes that both the engine and the UI use.

use crate::fertility;
use crate::instance::{PartnerGuess, Person};
use crate::model::*;
use crate::timeline::{DurationMode, Timeline};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NewPregnancy {
    pub name: Option<String>,
    /// Back-dated conception; None = now.
    pub start_ms: Option<i64>,
    pub mode: Option<DurationMode>,
    pub speed: Option<f64>,
    pub custom_ms: Option<i64>,
    pub baby_count: Option<u32>,
    pub kind: Option<BodyKind>,
    pub mother_species: Option<String>,
    pub source: PregnancySource,
    /// Avatar to attach to (defaults to the current avatar).
    pub avatar_id: Option<String>,
    /// Make it the one shown on the avatar.
    pub make_current: Option<bool>,
    /// Partner decided earlier (e.g. at the moment of a delayed conception).
    pub partner: Option<Partner>,
}

/// Looks up a remembered species answer for a person.
pub fn remembered_species(data: &AppData, user_id: Option<&str>, avatar_name: Option<&str>) -> Option<String> {
    user_id
        .and_then(|u| data.species_answers.get(u))
        .or_else(|| avatar_name.and_then(|a| data.species_answers.get(&a.trim().to_lowercase())))
        .or_else(|| avatar_name.and_then(|a| data.community_species.get(&a.trim().to_lowercase())))
        .cloned()
}

fn partner_from(data: &AppData, person: &Person, status: PartnerStatus, candidates: Vec<Person>) -> Partner {
    let species = remembered_species(data, person.user_id.as_deref(), person.avatar_name.as_deref());
    Partner {
        status,
        name: Some(person.name.clone()),
        user_id: person.user_id.clone(),
        avatar_name: person.avatar_name.clone(),
        species_source: if species.is_some() { SpeciesSource::Confirmed } else { SpeciesSource::Guess },
        species,
        candidates,
    }
}

/// Gives every baby a gender (keeps any already rolled).
pub fn ensure_genders(p: &mut Pregnancy) {
    let n = p.baby_count.max(1) as usize;
    p.genders.truncate(n);
    while p.genders.len() < n {
        p.genders.push(fertility::roll_gender(fertility::roll()).to_string());
    }
}

/// Rolls genders for any pregnancy that doesn't have them yet (older saves).
pub fn ensure_all_genders(data: &mut AppData) {
    for p in data.pregnancies.iter_mut().filter(|p| p.is_active()) {
        ensure_genders(p);
    }
}

/// The partner guess from who is in the instance right now.
pub fn guess_partner_now(data: &AppData, now_ms: i64) -> Partner {
    let settle = data.settings.settle_minutes as i64 * 60_000;
    let candidates = data.instance.candidates(now_ms);
    match data.instance.guess_partner(now_ms, settle) {
        PartnerGuess::Guessed { person } => partner_from(data, &person, PartnerStatus::Guessed, candidates),
        PartnerGuess::Unknown => Partner { candidates, ..Default::default() },
    }
}

/// Starts the fertility cycle if it hasn't started yet.
pub fn ensure_cycle(data: &mut AppData, now_ms: i64) {
    if data.settings.cycle_anchor_ms.is_none() {
        let c = fertility::cycle_ms(&data.settings);
        data.settings.cycle_anchor_ms = Some(fertility::first_anchor(now_ms, c));
    }
}

/// What happened after a WooHoo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunOutcome {
    /// Fertility off: the pregnancy started right away.
    Started,
    /// Grace period: outcome hidden until later.
    Pending,
    /// Already pregnant: nothing to do.
    Ignored,
}

/// Someone from `candidates` at random. `roll` is a random number in [0, 1).
fn pick(candidates: &[Person], roll: f64) -> Option<&Person> {
    let i = (roll * candidates.len() as f64) as usize;
    candidates.get(i.min(candidates.len().saturating_sub(1)))
}

/// The partner for a WooHoo: the guess, or someone here picked at random
/// once you've WooHoo'd in this instance as many times as there are other
/// people (2 or more) in it.
fn fun_partner(data: &AppData, now_ms: i64, roll: f64) -> Partner {
    let partner = guess_partner_now(data, now_ms);
    let here = data.instance.others().len();
    let woohoos = 1 + data.instance.joined_ms.map_or(0, |j| data.fun_log.iter().filter(|f| f.time_ms >= j).count());
    if partner.name.is_some() || here < 2 || woohoos < here {
        return partner;
    }
    // The candidate list starts with the people still here.
    match pick(&partner.candidates[..here.min(partner.candidates.len())], roll).cloned() {
        Some(person) => partner_from(data, &person, PartnerStatus::Random, partner.candidates),
        None => partner,
    }
}

/// In-game fun was detected. `roll` is a random number in [0, 1).
pub fn record_fun(data: &mut AppData, now_ms: i64, roll: f64) -> FunOutcome {
    if data.current_pregnancy(data.current_avatar_id.as_deref()).is_some() {
        return FunOutcome::Ignored;
    }
    let partner = fun_partner(data, now_ms, fertility::roll());
    data.fun_log.push(FunEvent {
        time_ms: now_ms,
        partner_name: partner.name.clone(),
        partner_user_id: partner.user_id.clone(),
        avatar_id: data.current_avatar_id.clone(),
        world_name: data.instance.world_name.clone(),
    });
    if !data.settings.fertility {
        start_pregnancy(data, NewPregnancy { source: PregnancySource::Conception, partner: Some(partner), ..Default::default() }, now_ms);
        return FunOutcome::Started;
    }
    ensure_cycle(data, now_ms);
    let s = &data.settings;
    let day = fertility::cycle_day(now_ms, s.cycle_anchor_ms.unwrap_or(now_ms), fertility::cycle_ms(s));
    let chance = fertility::chance(day, s.fertility_peak);
    let positive = roll < chance;
    let babies = fertility::roll_babies(s.multiples, fertility::roll());
    let owner = data.owner(data.current_avatar_id.as_deref());
    let global = data.settings.sync_scope == SyncScope::Global;
    let reveal_ms = now_ms + fertility::grace_ms(s);
    let world_name = data.instance.world_name.clone();
    match data.waiting.iter_mut().find(|w| global || w.avatar_id == owner) {
        // Another moment during the grace period: a new chance if the first didn't take.
        Some(p) if !p.positive && positive => {
            p.positive = true;
            p.time_ms = now_ms;
            p.chance = chance;
            p.cycle_day = day;
            // Its partner list and world are from this instance, not the first one.
            p.partner = partner;
            p.baby_count = babies;
            p.world_name = world_name;
        }
        Some(_) => {}
        None => data.waiting.push(PendingConception {
            time_ms: now_ms,
            reveal_ms,
            chance,
            cycle_day: day,
            positive,
            baby_count: babies,
            partner,
            avatar_id: owner,
            world_name,
        }),
    }
    FunOutcome::Pending
}

/// Ends the grace period if it's time. Returns the outcome when it does.
pub fn resolve_due(data: &mut AppData, now_ms: i64) -> Option<bool> {
    let i = data.waiting.iter().position(|w| w.reveal_ms <= now_ms)?;
    let p = data.waiting.remove(i);
    if p.positive {
        // Counts from the moment it happened, so you're already a bit along.
        start_pregnancy(
            data,
            NewPregnancy {
                start_ms: Some(p.time_ms),
                source: PregnancySource::Conception,
                avatar_id: p.avatar_id.clone(),
                partner: Some(p.partner.clone()),
                baby_count: Some(p.baby_count.max(1)),
                ..Default::default()
            },
            now_ms,
        );
        if let Some(world) = p.world_name {
            if let Some(last) = data.pregnancies.last_mut() {
                last.world_name = Some(world);
            }
        }
    }
    data.last_reveal = Some(RevealResult { time_ms: now_ms, event_ms: p.time_ms, positive: p.positive, seen: false });
    Some(p.positive)
}

/// Pregnancies that just reached their due date (each only once, kept in
/// its milestones): their baby counts, for the notification.
pub fn take_due(data: &mut AppData, now_ms: i64) -> Vec<u32> {
    let mut out = Vec::new();
    for p in data.pregnancies.iter_mut() {
        if p.ended_ms.is_some() || p.timeline.is_paused() || p.timeline.progress(now_ms) < 1.0 || p.milestones.iter().any(|m| m == "due") {
            continue;
        }
        p.milestones.push("due".into());
        out.push(p.baby_count);
    }
    out
}

pub fn start_pregnancy(data: &mut AppData, np: NewPregnancy, now_ms: i64) -> String {
    let s = &data.settings;
    let avatar_id = np.avatar_id.clone().or_else(|| data.current_avatar_id.clone());
    let mode = np.mode.unwrap_or(s.default_mode);
    let speed = np.speed.unwrap_or(s.default_speed);
    let partner = np.partner.clone().unwrap_or_else(|| guess_partner_now(data, now_ms));

    let profile = avatar_id.as_deref().and_then(|a| data.avatars.get(a));
    let kind = np.kind.or(profile.and_then(|p| p.kind)).unwrap_or_default();
    let mother_species = np.mother_species.clone().unwrap_or_else(|| data.user_species(avatar_id.as_deref()));
    let id = new_id("preg");
    let n = data.pregnancies.len() + 1;
    let mut p = Pregnancy {
        id: id.clone(),
        name: np.name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| format!("Little Bean #{n}")),
        created_ms: now_ms,
        timeline: Timeline::new(np.start_ms.unwrap_or(now_ms), mode, speed, np.custom_ms.or(data.settings.default_custom_ms)),
        baby_count: np.baby_count.unwrap_or_else(|| fertility::roll_babies(data.settings.multiples, fertility::roll())),
        kind,
        mother_species,
        partner,
        source: np.source,
        conceived_ms: (np.source == PregnancySource::Conception).then(|| np.start_ms.unwrap_or(now_ms)),
        avatar_id: avatar_id.clone(),
        world_name: data.instance.world_name.clone(),
        ..Default::default()
    };
    p.sync_babies();
    ensure_genders(&mut p);
    let title = match np.source {
        PregnancySource::Conception => "Conception detected in-game",
        PregnancySource::Manual => "Pregnancy started",
    };
    data.journal.push(JournalEntry {
        id: new_id("j"),
        time_ms: now_ms,
        pregnancy_id: Some(id.clone()),
        kind: "event".into(),
        title: title.into(),
        text: match &p.partner.name {
            Some(n) => format!("{} (partner guessed: {n})", p.name),
            None => p.name.clone(),
        },
        image: None,
    });
    data.pregnancies.push(p);
    if np.make_current.unwrap_or(true) {
        data.set_current(avatar_id.as_deref(), Some(id.clone()));
    }
    id
}

/// The partner chosen from the candidate list or typed in by name.
fn chosen(data: &AppData, person: Option<Person>, manual_name: Option<String>, candidates: Vec<Person>) -> Partner {
    match (person, manual_name) {
        (Some(person), _) => partner_from(data, &person, PartnerStatus::Confirmed, candidates),
        (None, Some(name)) if !name.trim().is_empty() => Partner {
            status: PartnerStatus::Manual,
            name: Some(name.trim().to_string()),
            candidates,
            ..Default::default()
        },
        _ => Partner { candidates, ..Default::default() },
    }
}

/// Credits the WooHoo at `time_ms` to `partner` (for the Partners list).
fn credit_fun(data: &mut AppData, time_ms: i64, partner: &Partner) {
    if let Some(f) = data.fun_log.iter_mut().find(|f| f.time_ms == time_ms) {
        f.partner_name = partner.name.clone();
        f.partner_user_id = partner.user_id.clone();
    }
}

/// Picks the other parent from the candidate list or by name.
pub fn set_partner(data: &mut AppData, preg_id: &str, person: Option<Person>, manual_name: Option<String>) {
    let Some(p) = data.pregnancy_mut(preg_id) else { return };
    let candidates = std::mem::take(&mut p.partner.candidates);
    let partner = chosen(data, person, manual_name, candidates);
    put_partner(data, preg_id, partner);
}

/// Picks the other parent at random from the candidate list.
pub fn random_partner(data: &mut AppData, preg_id: &str, roll: f64) {
    let Some(p) = data.pregnancy(preg_id) else { return };
    let Some(person) = pick(&p.partner.candidates, roll).cloned() else { return };
    let candidates = p.partner.candidates.clone();
    let partner = partner_from(data, &person, PartnerStatus::Random, candidates);
    put_partner(data, preg_id, partner);
}

fn put_partner(data: &mut AppData, preg_id: &str, partner: Partner) {
    // A pregnancy from in-game fun: credit that moment to whoever is chosen.
    if let Some(start) = data.pregnancy(preg_id).and_then(|p| p.conceived_ms) {
        credit_fun(data, start, &partner);
    }
    if let Some(p) = data.pregnancy_mut(preg_id) {
        p.partner = partner;
    }
}

/// Picks who it was while waiting to find out (by the WooHoo's time). It
/// counts for them even if nothing comes of it.
pub fn set_waiting_partner(data: &mut AppData, time_ms: i64, person: Option<Person>, manual_name: Option<String>) {
    let Some(w) = data.waiting.iter_mut().find(|w| w.time_ms == time_ms) else { return };
    let candidates = std::mem::take(&mut w.partner.candidates);
    let partner = chosen(data, person, manual_name, candidates);
    put_waiting_partner(data, time_ms, partner);
}

/// Picks who it was at random while waiting to find out.
pub fn random_waiting_partner(data: &mut AppData, time_ms: i64, roll: f64) {
    let Some(w) = data.waiting.iter().find(|w| w.time_ms == time_ms) else { return };
    let Some(person) = pick(&w.partner.candidates, roll).cloned() else { return };
    let candidates = w.partner.candidates.clone();
    let partner = partner_from(data, &person, PartnerStatus::Random, candidates);
    put_waiting_partner(data, time_ms, partner);
}

fn put_waiting_partner(data: &mut AppData, time_ms: i64, partner: Partner) {
    credit_fun(data, time_ms, &partner);
    if let Some(w) = data.waiting.iter_mut().find(|w| w.time_ms == time_ms) {
        w.partner = partner;
    }
}

pub fn end_pregnancy(data: &mut AppData, preg_id: &str, now_ms: i64) {
    if let Some(p) = data.pregnancy_mut(preg_id) {
        p.ended_ms = Some(now_ms);
        let name = p.name.clone();
        data.journal.push(JournalEntry {
            id: new_id("j"),
            time_ms: now_ms,
            pregnancy_id: Some(preg_id.to_string()),
            kind: "event".into(),
            title: "Pregnancy ended".into(),
            text: name,
            image: None,
        });
    }
    if data.active_pregnancy_id.as_deref() == Some(preg_id) {
        data.active_pregnancy_id = None;
    }
    for a in data.avatars.values_mut() {
        if a.pregnancy_id.as_deref() == Some(preg_id) {
            a.pregnancy_id = None;
        }
    }
    for g in data.groups.values_mut() {
        if g.pregnancy_id.as_deref() == Some(preg_id) {
            g.pregnancy_id = None;
        }
    }
}

/// The happy ending: ends the pregnancy as a birth.
pub fn give_birth(data: &mut AppData, preg_id: &str, now_ms: i64) {
    end_pregnancy(data, preg_id, now_ms);
    if let Some(p) = data.pregnancy_mut(preg_id) {
        p.born = true;
    }
}

pub fn delete_pregnancy(data: &mut AppData, preg_id: &str) {
    end_pregnancy(data, preg_id, 0);
    data.pregnancies.retain(|p| p.id != preg_id);
    data.journal.retain(|j| j.pregnancy_id.as_deref() != Some(preg_id));
    data.keepsakes.retain(|k| k.pregnancy_id != preg_id);
}

/// Forgets a person everywhere ("forget this partner"): history, the WooHoo
/// log (which the Partners list is built from), and pregnancies, which keep
/// going with an unknown partner.
pub fn forget_person(data: &mut AppData, name: &str, user_id: Option<&str>) {
    // Same person: by user ID when both have one, else by name.
    let same = |n: Option<&str>, u: Option<&str>| match (u, user_id) {
        (Some(a), Some(b)) => a == b,
        _ => n == Some(name),
    };
    data.instance.forget(name, user_id);
    if let Some(u) = user_id {
        data.species_answers.remove(u);
    }
    data.fun_log.retain(|f| !same(f.partner_name.as_deref(), f.partner_user_id.as_deref()));
    for p in &mut data.pregnancies {
        p.partner.candidates.retain(|c| !same(Some(&c.name), c.user_id.as_deref()));
        if same(p.partner.name.as_deref(), p.partner.user_id.as_deref()) {
            let candidates = std::mem::take(&mut p.partner.candidates);
            p.partner = Partner { candidates, ..Default::default() };
        }
    }
}

/// What a Data Management reset clears.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResetKind {
    /// The avatar list and per-avatar settings.
    Avatars,
    /// Pregnancies, grace periods, reveals, journal and keepsakes.
    Pregnancies,
    /// WooHoo history, instance history and remembered species.
    Partners,
    Achievements,
    Settings,
    /// Factory reset.
    Everything,
}

/// Clears one kind of data (Settings > Data Management).
pub fn reset_data(data: &mut AppData, kind: ResetKind) {
    match kind {
        ResetKind::Avatars => {
            data.avatars.clear();
            data.groups.clear();
            for p in &mut data.pregnancies {
                p.avatar_id = None;
            }
        }
        ResetKind::Pregnancies => {
            data.pregnancies.clear();
            data.active_pregnancy_id = None;
            data.waiting.clear();
            data.last_reveal = None;
            data.journal.clear();
            data.keepsakes.clear();
            for a in data.avatars.values_mut() {
                a.pregnancy_id = None;
            }
            for g in data.groups.values_mut() {
                g.pregnancy_id = None;
            }
        }
        ResetKind::Partners => {
            data.fun_log.clear();
            data.instance.history.clear();
            data.species_answers.clear();
        }
        ResetKind::Achievements => data.achievements.clear(),
        ResetKind::Settings => data.settings = Default::default(),
        ResetKind::Everything => *data = AppData::new(),
    }
}

/// Records an avatar switch from OSC.
pub fn touch_avatar(data: &mut AppData, avatar_id: &str, now_ms: i64) {
    data.current_avatar_id = Some(avatar_id.to_string());
    let p = data
        .avatars
        .entry(avatar_id.to_string())
        .or_insert_with(|| AvatarProfile { id: avatar_id.to_string(), ..Default::default() });
    p.last_seen_ms = now_ms;
}

/// Does this owner (avatar or group) have a pregnancy or a grace period going?
fn has_pregnancy(data: &AppData, owner: &str) -> bool {
    let preg = match data.groups.get(owner) {
        Some(g) => g.pregnancy_id.clone(),
        None => data.avatars.get(owner).and_then(|a| a.pregnancy_id.clone()),
    };
    preg.and_then(|id| data.pregnancy(&id).map(|p| p.is_active())).unwrap_or(false) || data.waiting.iter().any(|w| w.avatar_id.as_deref() == Some(owner))
}

/// Hands one owner's pregnancy and grace period to another.
fn move_pregnancy(data: &mut AppData, from: &str, to: &str) {
    let preg = match data.groups.get_mut(from) {
        Some(g) => g.pregnancy_id.take(),
        None => data.avatars.get_mut(from).and_then(|a| a.pregnancy_id.take()),
    };
    match data.groups.get_mut(to) {
        Some(g) => g.pregnancy_id = preg,
        None => {
            if let Some(a) = data.avatars.get_mut(to) {
                a.pregnancy_id = preg;
            }
        }
    }
    for w in &mut data.waiting {
        if w.avatar_id.as_deref() == Some(from) {
            w.avatar_id = Some(to.to_string());
        }
    }
}

/// Makes a new avatar group and puts these avatars in it.
pub fn create_group(data: &mut AppData, name: &str, avatar_ids: &[String]) -> Result<String, String> {
    let id = new_id("grp");
    let name = name.trim();
    let n = data.groups.len() + 1;
    data.groups.insert(id.clone(), AvatarGroup { id: id.clone(), name: if name.is_empty() { format!("Group {n}") } else { name.to_string() }, pregnancy_id: None });
    for a in avatar_ids {
        if let Err(e) = set_avatar_group(data, a, Some(&id)) {
            // Keep the avatars already moved; just stop at the clash.
            prune_groups(data);
            return Err(e);
        }
    }
    Ok(id)
}

/// Moves an avatar into a group, or out of its group with `None`.
///
/// An avatar with its own pregnancy brings it along if the group has none.
/// Leaving keeps the group's pregnancy with the group, unless this was the
/// last avatar in it, which takes the pregnancy with it.
pub fn set_avatar_group(data: &mut AppData, avatar_id: &str, group_id: Option<&str>) -> Result<(), String> {
    if let Some(g) = group_id {
        if !data.groups.contains_key(g) {
            return Err("That group doesn't exist anymore.".into());
        }
    }
    let Some(profile) = data.avatars.get(avatar_id) else { return Err("That avatar isn't in your list.".into()) };
    let old = profile.group_id.clone().filter(|g| data.groups.contains_key(g));
    if old.as_deref() == group_id {
        return Ok(());
    }
    let name = profile.name.clone().unwrap_or_else(|| "This avatar".into());
    // Where this avatar's pregnancy lives now: its group if it's the last one in it, else itself.
    let leaving_last = old.as_ref().is_some_and(|g| data.group_members(g).len() == 1);
    let carried = match &old {
        Some(g) if leaving_last => g.clone(),
        Some(_) => String::new(),
        None => avatar_id.to_string(),
    };
    let carries = !carried.is_empty() && has_pregnancy(data, &carried);
    if let Some(g) = group_id {
        if carries && has_pregnancy(data, g) {
            let gname = data.groups[g].name.clone();
            return Err(format!("{name} and {gname} both have a pregnancy. End or finish one first."));
        }
    }
    data.avatars.get_mut(avatar_id).unwrap().group_id = group_id.map(str::to_string);
    if carries {
        move_pregnancy(data, &carried, group_id.unwrap_or(avatar_id));
    }
    prune_groups(data);
    Ok(())
}

pub fn rename_group(data: &mut AppData, group_id: &str, name: &str) {
    if let Some(g) = data.groups.get_mut(group_id) {
        if !name.trim().is_empty() {
            g.name = name.trim().to_string();
        }
    }
}

/// Removes a group. Its avatars go back to their own pregnancies, and the
/// group's pregnancy goes to the avatar worn most recently.
pub fn delete_group(data: &mut AppData, group_id: &str) {
    let members = data.group_members(group_id);
    if let Some(first) = members.first() {
        move_pregnancy(data, group_id, first);
    }
    for m in &members {
        data.avatars.get_mut(m).unwrap().group_id = None;
    }
    data.groups.remove(group_id);
    data.waiting.retain(|w| w.avatar_id.as_deref() != Some(group_id));
}

/// Forgets an avatar (and any group it leaves empty).
pub fn delete_avatar(data: &mut AppData, avatar_id: &str) {
    data.avatars.remove(avatar_id);
    prune_groups(data);
}

/// Groups with no avatars left are removed.
fn prune_groups(data: &mut AppData) {
    let used: std::collections::BTreeSet<String> = data.avatars.values().filter_map(|a| a.group_id.clone()).collect();
    let gone: Vec<String> = data.groups.keys().filter(|g| !used.contains(*g)).cloned().collect();
    for g in gone {
        data.groups.remove(&g);
        data.waiting.retain(|w| w.avatar_id.as_deref() != Some(g.as_str()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logparse::LogEvent;

    fn with_instance(others: &[&str]) -> AppData {
        let mut d = AppData::new();
        d.instance.apply(&LogEvent::LocalUser { name: "Me".into(), user_id: Some("usr_me".into()) }, 0);
        d.instance.apply(&LogEvent::JoiningWorld { world_id: "wrld_1".into(), instance: "1".into() }, 0);
        d.instance.apply(&LogEvent::EnteringRoom { world_name: "Cozy".into() }, 0);
        d.instance.apply(&LogEvent::PlayerJoined { name: "Me".into(), user_id: Some("usr_me".into()) }, 0);
        for o in others {
            d.instance.apply(&LogEvent::PlayerJoined { name: o.to_string(), user_id: Some(format!("usr_{o}")) }, 0);
            d.instance.apply(&LogEvent::AvatarSwitch { player: o.to_string(), avatar_name: format!("{o} Avatar") }, 0);
        }
        d
    }

    const LATER: i64 = 60 * 60_000;

    #[test]
    fn due_is_announced_once() {
        let mut d = AppData::new();
        let id = start_pregnancy(&mut d, NewPregnancy { mode: Some(DurationMode::Custom), custom_ms: Some(LATER), ..Default::default() }, 0);
        assert!(take_due(&mut d, LATER / 2).is_empty());
        let n = d.pregnancy(&id).unwrap().baby_count;
        assert_eq!(take_due(&mut d, LATER), vec![n]);
        assert!(take_due(&mut d, LATER * 2).is_empty(), "only once");
    }

    #[test]
    fn start_with_one_other_guesses_partner() {
        let mut d = with_instance(&["Fox"]);
        let id = start_pregnancy(&mut d, NewPregnancy { source: PregnancySource::Conception, ..Default::default() }, LATER);
        let p = d.pregnancy(&id).unwrap();
        assert_eq!(p.partner.status, PartnerStatus::Guessed);
        assert_eq!(p.partner.name.as_deref(), Some("Fox"));
        assert_eq!(p.world_name.as_deref(), Some("Cozy"));
        assert_eq!(d.current_pregnancy(None).unwrap().id, id);
        assert_eq!(d.journal.len(), 1);
    }

    #[test]
    fn start_with_crowd_is_unknown_but_lists_candidates() {
        let mut d = with_instance(&["Fox", "Wolf"]);
        let id = start_pregnancy(&mut d, NewPregnancy::default(), LATER);
        let p = d.pregnancy(&id).unwrap();
        assert_eq!(p.partner.status, PartnerStatus::Unknown);
        assert_eq!(p.partner.candidates.len(), 2);
        assert_eq!(p.mother_species, "human", "starts from the user's species alone");
    }

    #[test]
    fn remembered_species_is_used() {
        let mut d = with_instance(&["Fox"]);
        d.species_answers.insert("usr_Fox".into(), "fox".into());
        let id = start_pregnancy(&mut d, NewPregnancy::default(), LATER);
        let p = d.pregnancy(&id).unwrap();
        assert_eq!(p.partner.species.as_deref(), Some("fox"));
        assert_eq!(p.partner.species_source, SpeciesSource::Confirmed);
    }

    #[test]
    fn choose_partner_keeps_candidates() {
        let mut d = with_instance(&["Fox", "Wolf"]);
        let id = start_pregnancy(&mut d, NewPregnancy::default(), LATER);
        let wolf = d.pregnancy(&id).unwrap().partner.candidates.iter().find(|c| c.name == "Wolf").cloned();
        set_partner(&mut d, &id, wolf, None);
        let p = d.pregnancy(&id).unwrap();
        assert_eq!(p.partner.status, PartnerStatus::Confirmed);
        assert_eq!(p.partner.name.as_deref(), Some("Wolf"));
        assert_eq!(p.partner.candidates.len(), 2);
        set_partner(&mut d, &id, None, Some("Someone else".into()));
        assert_eq!(d.pregnancy(&id).unwrap().partner.status, PartnerStatus::Manual);
    }

    #[test]
    fn forget_removes_everywhere() {
        let mut d = with_instance(&["Fox"]);
        let id = start_pregnancy(&mut d, NewPregnancy::default(), LATER);
        d.fun_log.push(FunEvent { time_ms: LATER, partner_name: Some("Fox".into()), partner_user_id: Some("usr_Fox".into()), ..Default::default() });
        d.fun_log.push(FunEvent { time_ms: LATER, partner_name: Some("Wolf".into()), ..Default::default() });
        forget_person(&mut d, "Fox", Some("usr_Fox"));
        let p = d.pregnancy(&id).unwrap();
        assert!(p.partner.name.is_none());
        assert!(p.partner.candidates.is_empty());
        assert!(d.instance.history.iter().all(|c| c.name != "Fox"));
        // Gone from the WooHoo log too (the Partners list), and only them.
        assert_eq!(d.fun_log.len(), 1);
        assert_eq!(d.fun_log[0].partner_name.as_deref(), Some("Wolf"));
    }

    #[test]
    fn resets_clear_only_their_kind() {
        let mut d = with_instance(&["Fox"]);
        start_pregnancy(&mut d, NewPregnancy::default(), LATER);
        touch_avatar(&mut d, "avtr_a", 0);
        d.fun_log.push(FunEvent::default());
        reset_data(&mut d, ResetKind::Partners);
        assert!(d.fun_log.is_empty() && d.instance.history.is_empty());
        assert_eq!(d.pregnancies.len(), 1, "pregnancies kept");
        reset_data(&mut d, ResetKind::Pregnancies);
        assert!(d.pregnancies.is_empty() && d.active_pregnancy_id.is_none());
        assert!(d.avatars.contains_key("avtr_a"), "avatars kept");
        reset_data(&mut d, ResetKind::Avatars);
        assert!(d.avatars.is_empty());
        d.settings.kicks = false;
        reset_data(&mut d, ResetKind::Everything);
        assert!(d.settings.kicks);
    }

    #[test]
    fn end_and_delete() {
        let mut d = AppData::new();
        let id = start_pregnancy(&mut d, NewPregnancy::default(), 0);
        end_pregnancy(&mut d, &id, 10);
        assert!(d.current_pregnancy(None).is_none());
        delete_pregnancy(&mut d, &id);
        assert!(d.pregnancies.is_empty());
        assert!(d.journal.is_empty());
    }

    #[test]
    fn fun_with_fertility_waits_for_the_reveal() {
        let mut d = with_instance(&["Fox"]);
        assert_eq!(record_fun(&mut d, LATER, 0.0), FunOutcome::Pending);
        assert!(d.pregnancies.is_empty(), "no pregnancy yet");
        let p = d.waiting[0].clone();
        assert!(p.positive);
        assert_eq!(p.partner.name.as_deref(), Some("Fox"));
        assert_eq!(resolve_due(&mut d, p.reveal_ms - 1), None, "not before the grace period ends");
        assert_eq!(resolve_due(&mut d, p.reveal_ms), Some(true));
        let preg = d.current_pregnancy(None).unwrap();
        assert_eq!(preg.timeline.start_ms, LATER, "counts from the moment it happened");
        assert_eq!(preg.partner.name.as_deref(), Some("Fox"));
        assert!(!d.last_reveal.as_ref().unwrap().seen);
    }

    #[test]
    fn fun_is_logged_and_birth_counts() {
        let mut d = with_instance(&["Fox"]);
        d.settings.fertility = false;
        record_fun(&mut d, LATER, 0.0);
        assert_eq!(d.fun_log.len(), 1);
        assert_eq!(d.fun_log[0].partner_name.as_deref(), Some("Fox"));
        let id = d.current_pregnancy(None).unwrap().id.clone();
        give_birth(&mut d, &id, LATER + 1);
        let p = d.pregnancy(&id).unwrap();
        assert!(p.born && p.ended_ms.is_some());
        assert!(d.current_pregnancy(None).is_none());
    }

    #[test]
    fn choosing_a_partner_credits_the_moment() {
        let mut d = with_instance(&["Fox", "Wolf"]);
        record_fun(&mut d, LATER, 0.0);
        assert!(d.fun_log[0].partner_name.is_none(), "two people: unknown at first");
        let reveal = d.waiting[0].reveal_ms;
        resolve_due(&mut d, reveal);
        let id = d.current_pregnancy(None).unwrap().id.clone();
        // Moving the progress must not break the link.
        d.pregnancy_mut(&id).unwrap().timeline.set_progress(LATER, 0.5);
        let wolf = d.pregnancy(&id).unwrap().partner.candidates.iter().find(|c| c.name == "Wolf").cloned();
        set_partner(&mut d, &id, wolf, None);
        assert_eq!(d.fun_log[0].partner_name.as_deref(), Some("Wolf"));
    }

    #[test]
    fn random_partner_picks_someone_there() {
        let mut d = with_instance(&["Fox", "Wolf"]);
        d.settings.fertility = false;
        record_fun(&mut d, LATER, 0.0);
        let id = d.current_pregnancy(None).unwrap().id.clone();
        random_partner(&mut d, &id, 0.99);
        let p = d.pregnancy(&id).unwrap();
        assert_eq!(p.partner.status, PartnerStatus::Random);
        assert_eq!(p.partner.name.as_deref(), Some("Wolf"));
        assert_eq!(p.partner.candidates.len(), 2, "list kept for another go");
        assert_eq!(d.fun_log[0].partner_name.as_deref(), Some("Wolf"));
    }

    #[test]
    fn picking_while_waiting_counts_even_if_not_pregnant() {
        let mut d = with_instance(&["Fox", "Wolf"]);
        record_fun(&mut d, LATER, 0.999);
        let fox = d.waiting[0].partner.candidates.iter().find(|c| c.name == "Fox").cloned();
        set_waiting_partner(&mut d, LATER, fox, None);
        assert_eq!(d.waiting[0].partner.status, PartnerStatus::Confirmed);
        let reveal = d.waiting[0].reveal_ms;
        assert_eq!(resolve_due(&mut d, reveal), Some(false));
        assert_eq!(d.fun_log[0].partner_name.as_deref(), Some("Fox"));
        // And at random.
        record_fun(&mut d, reveal + 1, 0.999);
        random_waiting_partner(&mut d, reveal + 1, 0.0);
        assert_eq!(d.waiting[0].partner.status, PartnerStatus::Random);
        assert_eq!(d.fun_log[1].partner_name.as_deref(), Some("Fox"));
    }

    #[test]
    fn as_many_woohoos_as_people_picks_at_random() {
        let mut d = with_instance(&["Fox", "Wolf", "Bun"]);
        d.settings.fertility = false;
        for i in 0..2 {
            record_fun(&mut d, LATER + i, 0.0);
            let id = d.current_pregnancy(None).unwrap().id.clone();
            end_pregnancy(&mut d, &id, LATER + i);
        }
        assert!(d.fun_log.iter().all(|f| f.partner_name.is_none()), "fewer WooHoos than people: unknown");
        record_fun(&mut d, LATER + 5, 0.0);
        let p = d.current_pregnancy(None).unwrap();
        assert_eq!(p.partner.status, PartnerStatus::Random);
        assert!(["Fox", "Wolf", "Bun"].contains(&p.partner.name.as_deref().unwrap()));
        assert!(d.fun_log[2].partner_name.is_some());
    }

    #[test]
    fn woohoos_from_another_instance_dont_count() {
        let mut d = with_instance(&["Fox", "Wolf"]);
        d.fun_log.push(FunEvent { time_ms: -1, ..Default::default() });
        record_fun(&mut d, LATER, 0.999);
        assert!(d.waiting[0].partner.name.is_none());
    }

    #[test]
    fn every_baby_gets_a_gender() {
        let mut d = AppData::new();
        let id = start_pregnancy(&mut d, NewPregnancy { baby_count: Some(3), ..Default::default() }, 0);
        let p = d.pregnancy_mut(&id).unwrap();
        assert_eq!(p.genders.len(), 3);
        assert!(p.genders.iter().all(|g| g == "girl" || g == "boy"));
        let first = p.genders.clone();
        p.baby_count = 2;
        ensure_genders(p);
        assert_eq!(p.genders, first[..2].to_vec(), "kept, not rerolled");
    }

    #[test]
    fn unlucky_roll_is_not_pregnant() {
        let mut d = with_instance(&["Fox"]);
        record_fun(&mut d, LATER, 0.999);
        let reveal = d.waiting[0].reveal_ms;
        assert_eq!(resolve_due(&mut d, reveal), Some(false));
        assert!(d.pregnancies.is_empty());
        assert!(d.waiting.is_empty());
    }

    #[test]
    fn second_chance_during_grace() {
        let mut d = with_instance(&["Fox"]);
        record_fun(&mut d, LATER, 0.999);
        let first_reveal = d.waiting[0].reveal_ms;
        record_fun(&mut d, LATER + 1000, 0.0);
        assert_eq!(d.waiting.len(), 1);
        let p = &d.waiting[0];
        assert!(p.positive);
        assert_eq!(p.time_ms, LATER + 1000);
        assert_eq!(p.reveal_ms, first_reveal, "the reveal date doesn't move");
    }

    #[test]
    fn second_chance_in_a_new_instance_lists_that_instance() {
        let mut d = with_instance(&["Fox"]);
        record_fun(&mut d, LATER, 0.999);
        d.instance.apply(&LogEvent::LeftRoom, LATER + 10);
        d.instance.apply(&LogEvent::EnteringRoom { world_name: "Neon".into() }, LATER + 20);
        d.instance.apply(&LogEvent::JoiningWorld { world_id: "wrld_2".into(), instance: "2".into() }, LATER + 20);
        for n in ["Me", "Wolf", "Rex"] {
            d.instance.apply(&LogEvent::PlayerJoined { name: n.into(), user_id: Some(format!("usr_{}", n.to_lowercase())) }, LATER + 30);
        }
        record_fun(&mut d, LATER + 1000, 0.0);
        let p = &d.waiting[0];
        let names: Vec<&str> = p.partner.candidates.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Wolf", "Rex"], "only people from the instance where it took");
        assert_eq!(p.world_name.as_deref(), Some("Neon"));
    }

    #[test]
    fn each_avatar_group_has_its_own_grace_period() {
        let mut d = with_instance(&[]);
        d.settings.sync_scope = SyncScope::PerAvatar;
        touch_avatar(&mut d, "avtr_a", 0);
        record_fun(&mut d, LATER, 0.0);
        touch_avatar(&mut d, "avtr_b", 0);
        assert!(d.pending_for(Some("avtr_b")).is_none());
        record_fun(&mut d, LATER, 0.999);
        assert_eq!(d.waiting.len(), 2);
        let reveal = d.waiting[0].reveal_ms;
        assert_eq!(resolve_due(&mut d, reveal), Some(true));
        assert_eq!(resolve_due(&mut d, reveal), Some(false));
        assert!(d.current_pregnancy(Some("avtr_a")).is_some());
        assert!(d.current_pregnancy(Some("avtr_b")).is_none());
    }

    #[test]
    fn fertility_off_starts_right_away() {
        let mut d = with_instance(&["Fox"]);
        d.settings.fertility = false;
        assert_eq!(record_fun(&mut d, LATER, 0.5), FunOutcome::Started);
        assert!(d.current_pregnancy(None).is_some());
        assert_eq!(record_fun(&mut d, LATER + 1, 0.0), FunOutcome::Ignored);
    }

    #[test]
    fn per_avatar_start_links_profile() {
        let mut d = AppData::new();
        d.settings.sync_scope = SyncScope::PerAvatar;
        touch_avatar(&mut d, "avtr_1", 0);
        let id = start_pregnancy(&mut d, NewPregnancy::default(), 0);
        assert_eq!(d.avatars["avtr_1"].pregnancy_id.as_deref(), Some(id.as_str()));
        assert!(d.active_pregnancy_id.is_none());
    }

    fn three_avatars() -> AppData {
        let mut d = AppData::new();
        d.settings.sync_scope = SyncScope::PerAvatar;
        for (i, id) in ["a", "b", "c"].iter().enumerate() {
            touch_avatar(&mut d, id, i as i64);
        }
        d
    }

    #[test]
    fn group_takes_the_first_pregnancy_along() {
        let mut d = three_avatars();
        let p = start_pregnancy(&mut d, NewPregnancy { avatar_id: Some("a".into()), ..Default::default() }, 0);
        let g = create_group(&mut d, "Fluffs", &["a".into(), "b".into()]).unwrap();
        assert_eq!(d.groups[&g].pregnancy_id.as_deref(), Some(p.as_str()));
        assert!(d.avatars["a"].pregnancy_id.is_none());
        assert_eq!(d.current_pregnancy(Some("b")).unwrap().id, p);
        assert!(d.current_pregnancy(Some("c")).is_none());
        let mut m = d.linked_group("a");
        m.sort();
        assert_eq!(m, vec!["a", "b"]);
    }

    #[test]
    fn two_pregnancies_cant_share_a_group() {
        let mut d = three_avatars();
        start_pregnancy(&mut d, NewPregnancy { avatar_id: Some("a".into()), ..Default::default() }, 0);
        start_pregnancy(&mut d, NewPregnancy { avatar_id: Some("c".into()), ..Default::default() }, 0);
        let g = create_group(&mut d, "", &["a".into()]).unwrap();
        assert_eq!(d.groups[&g].name, "Group 1");
        assert!(set_avatar_group(&mut d, "c", Some(&g)).is_err());
        assert!(d.avatars["c"].group_id.is_none(), "unchanged");
        // A grace period counts too.
        let b_wait = PendingConception { avatar_id: Some("b".into()), ..Default::default() };
        d.waiting.push(b_wait);
        assert!(set_avatar_group(&mut d, "b", Some(&g)).is_err());
    }

    #[test]
    fn leaving_and_deleting_groups() {
        let mut d = three_avatars();
        let g = create_group(&mut d, "Fluffs", &["a".into(), "b".into()]).unwrap();
        let p = start_pregnancy(&mut d, NewPregnancy { avatar_id: Some("a".into()), ..Default::default() }, 0);
        // Leaving keeps the pregnancy with the group.
        set_avatar_group(&mut d, "a", None).unwrap();
        assert!(d.current_pregnancy(Some("a")).is_none());
        assert_eq!(d.current_pregnancy(Some("b")).unwrap().id, p);
        // The last one out takes it along, and the empty group goes.
        set_avatar_group(&mut d, "b", None).unwrap();
        assert!(d.groups.is_empty());
        assert_eq!(d.current_pregnancy(Some("b")).unwrap().id, p);
        // Deleting gives it to the avatar worn last (c).
        let g2 = create_group(&mut d, "Again", &["b".into(), "c".into()]).unwrap();
        assert_ne!(g, g2);
        delete_group(&mut d, &g2);
        assert!(d.groups.is_empty() && d.avatars.values().all(|a| a.group_id.is_none()));
        assert_eq!(d.current_pregnancy(Some("c")).unwrap().id, p);
        assert!(d.current_pregnancy(Some("b")).is_none());
    }

    #[test]
    fn forgetting_the_last_avatar_removes_its_group() {
        let mut d = three_avatars();
        let g = create_group(&mut d, "Solo", &["a".into()]).unwrap();
        delete_avatar(&mut d, "a");
        assert!(!d.groups.contains_key(&g));
    }

    #[test]
    fn group_grace_period_follows_the_group() {
        let mut d = with_instance(&["Fox"]);
        d.settings.sync_scope = SyncScope::PerAvatar;
        touch_avatar(&mut d, "a", 0);
        touch_avatar(&mut d, "b", 1);
        create_group(&mut d, "Fluffs", &["a".into(), "b".into()]).unwrap();
        record_fun(&mut d, LATER, 0.0);
        assert!(d.pending_for(Some("a")).is_some(), "fun on b counts for a");
        let reveal = d.waiting[0].reveal_ms;
        assert_eq!(resolve_due(&mut d, reveal), Some(true));
        assert!(d.current_pregnancy(Some("a")).is_some());
    }
}
