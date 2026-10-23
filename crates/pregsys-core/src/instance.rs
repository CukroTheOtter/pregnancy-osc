//! Who is in the instance right now, and who the user has shared instances
//! with recently. Fed by log events; used for the partner guess.

use crate::logparse::LogEvent;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// How long after a join/leave the instance still counts as "changing".
pub const DEFAULT_SETTLE_MS: i64 = 10 * 60_000;
const HISTORY_CAP: usize = 200;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub name: String,
    pub user_id: Option<String>,
    /// Last avatar name seen in the log (fuzzy, name only).
    pub avatar_name: Option<String>,
    pub last_seen_ms: i64,
    #[serde(default)]
    pub world_name: Option<String>,
}

impl Person {
    fn same(&self, name: &str, user_id: Option<&str>) -> bool {
        match (self.user_id.as_deref(), user_id) {
            (Some(a), Some(b)) => a == b,
            _ => self.name == name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum PartnerGuess {
    /// Exactly one other person, and the instance was settled.
    Guessed { person: Person },
    Unknown,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InstanceTracker {
    pub local: Option<Person>,
    pub world_id: Option<String>,
    pub world_name: Option<String>,
    pub instance: Option<String>,
    pub joined_ms: Option<i64>,
    /// Everyone in the instance, including the local user.
    pub present: Vec<Person>,
    /// People shared an instance with, most recent first.
    pub history: Vec<Person>,
    /// Everyone (except the local user) seen in the current instance visit.
    pub session: Vec<Person>,
    /// Times other players joined or left (for the "settled" check).
    #[serde(skip)]
    changes: VecDeque<i64>,
    /// Waiting for the first OnPlayerJoined after entering (usually the local user).
    #[serde(skip)]
    expect_self: bool,
    /// "Entering Room" was just seen. VRChat logs it right before "Joining
    /// wrld_...", which then belongs to the same new instance.
    #[serde(skip)]
    entering: bool,
}

impl InstanceTracker {
    pub fn apply(&mut self, ev: &LogEvent, now_ms: i64) {
        match ev {
            LogEvent::LocalUser { name, user_id } => {
                self.local = Some(Person { name: name.clone(), user_id: user_id.clone(), avatar_name: None, last_seen_ms: now_ms, world_name: None });
            }
            LogEvent::JoiningWorld { world_id, instance } => {
                // Either order works: "Entering Room" then "Joining" (what VRChat
                // logs) is one new instance, so the room name it just set is kept.
                if !std::mem::take(&mut self.entering) {
                    self.leave(now_ms);
                    self.joined_ms = Some(now_ms);
                }
                self.world_id = Some(world_id.clone());
                self.instance = Some(instance.clone());
                self.expect_self = true;
            }
            LogEvent::EnteringRoom { world_name } => {
                if self.world_id.is_some() && self.world_name.is_some() {
                    // Still in the last instance: this is a new one.
                    self.leave(now_ms);
                }
                self.entering = self.world_id.is_none();
                self.world_name = Some(world_name.clone());
                if self.joined_ms.is_none() {
                    self.joined_ms = Some(now_ms);
                }
                self.expect_self = self.present.is_empty();
            }
            LogEvent::LeftRoom => self.leave(now_ms),
            LogEvent::PlayerJoined { name, user_id } => {
                let p = Person { name: name.clone(), user_id: user_id.clone(), avatar_name: None, last_seen_ms: now_ms, world_name: self.world_name.clone() };
                // The first join after entering is the local user when we didn't see "User Authenticated".
                if self.expect_self && self.local.is_none() {
                    self.local = Some(p.clone());
                }
                self.expect_self = false;
                if !self.present.iter().any(|x| x.same(name, user_id.as_deref())) {
                    self.present.push(p.clone());
                }
                if !self.is_local(name, user_id.as_deref()) {
                    self.changes.push_back(now_ms);
                    self.touch_session(p.clone());
                    self.touch_history(p);
                }
            }
            LogEvent::PlayerLeft { name, user_id } => {
                if let Some(i) = self.present.iter().position(|x| x.same(name, user_id.as_deref())) {
                    let mut p = self.present.remove(i);
                    p.last_seen_ms = now_ms;
                    if !self.is_local(name, user_id.as_deref()) {
                        self.changes.push_back(now_ms);
                        self.touch_session(p.clone());
                        self.touch_history(p);
                    }
                }
            }
            LogEvent::AvatarSwitch { player, avatar_name } => {
                for p in self.present.iter_mut().filter(|p| &p.name == player) {
                    p.avatar_name = Some(avatar_name.clone());
                }
                if let Some(l) = self.local.as_mut().filter(|l| &l.name == player) {
                    l.avatar_name = Some(avatar_name.clone());
                }
                for h in self.history.iter_mut().chain(self.session.iter_mut()).filter(|h| &h.name == player) {
                    h.avatar_name = Some(avatar_name.clone());
                }
            }
        }
        while self.changes.len() > 64 {
            self.changes.pop_front();
        }
    }

    fn leave(&mut self, now_ms: i64) {
        let others: Vec<Person> = self.others().into_iter().cloned().collect();
        for mut p in others {
            p.last_seen_ms = now_ms;
            self.touch_history(p);
        }
        self.present.clear();
        self.session.clear();
        self.changes.clear();
        self.world_id = None;
        self.world_name = None;
        self.instance = None;
        self.joined_ms = None;
    }

    /// VRChat closed. It doesn't always log leaving the room, so leave here.
    pub fn vrchat_closed(&mut self, now_ms: i64) {
        self.leave(now_ms);
    }

    fn touch_session(&mut self, p: Person) {
        let avatar = self.session.iter().find(|s| s.same(&p.name, p.user_id.as_deref())).and_then(|s| s.avatar_name.clone());
        self.session.retain(|s| !s.same(&p.name, p.user_id.as_deref()));
        self.session.push(Person { avatar_name: p.avatar_name.clone().or(avatar), ..p });
    }

    fn touch_history(&mut self, p: Person) {
        self.history.retain(|h| !h.same(&p.name, p.user_id.as_deref()));
        self.history.insert(0, p);
        self.history.truncate(HISTORY_CAP);
    }

    pub fn is_local(&self, name: &str, user_id: Option<&str>) -> bool {
        self.local.as_ref().is_some_and(|l| l.same(name, user_id))
    }

    /// Everyone present except the local user.
    pub fn others(&self) -> Vec<&Person> {
        self.present.iter().filter(|p| !self.is_local(&p.name, p.user_id.as_deref())).collect()
    }

    /// No other player joined or left within `settle_ms`.
    pub fn settled(&self, now_ms: i64, settle_ms: i64) -> bool {
        self.changes.back().is_none_or(|t| now_ms - t >= settle_ms)
            && self.joined_ms.is_some_and(|j| now_ms - j >= settle_ms.min(60_000))
    }

    /// Exactly one other person and nobody came or went recently = guessed.
    pub fn guess_partner(&self, now_ms: i64, settle_ms: i64) -> PartnerGuess {
        let others = self.others();
        if others.len() == 1 && self.settled(now_ms, settle_ms) {
            let mut person = others[0].clone();
            person.last_seen_ms = now_ms;
            PartnerGuess::Guessed { person }
        } else {
            PartnerGuess::Unknown
        }
    }

    /// "Who was the other parent?" list: only people from this instance
    /// visit. Those still here come first, then those who left, most recent first.
    pub fn candidates(&self, now_ms: i64) -> Vec<Person> {
        let mut out: Vec<Person> = self
            .others()
            .into_iter()
            .map(|p| Person { last_seen_ms: now_ms, ..p.clone() })
            .collect();
        let mut gone: Vec<Person> = self
            .session
            .iter()
            .filter(|s| !out.iter().any(|o| o.same(&s.name, s.user_id.as_deref())))
            .cloned()
            .collect();
        gone.sort_by(|a, b| b.last_seen_ms.cmp(&a.last_seen_ms));
        out.extend(gone);
        out
    }

    /// Removes someone from history ("forget this partner").
    pub fn forget(&mut self, name: &str, user_id: Option<&str>) {
        self.history.retain(|h| !h.same(name, user_id));
        self.session.retain(|h| !h.same(name, user_id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logparse::LogEvent::*;

    const M: i64 = 60_000;

    fn join(n: &str) -> LogEvent {
        PlayerJoined { name: n.into(), user_id: Some(format!("usr_{n}")) }
    }
    fn left(n: &str) -> LogEvent {
        PlayerLeft { name: n.into(), user_id: Some(format!("usr_{n}")) }
    }

    fn room(t: &mut InstanceTracker, at: i64) {
        t.apply(&LocalUser { name: "Me".into(), user_id: Some("usr_Me".into()) }, at);
        t.apply(&JoiningWorld { world_id: "wrld_1".into(), instance: "1~private".into() }, at);
        t.apply(&EnteringRoom { world_name: "Cozy Home".into() }, at);
        t.apply(&join("Me"), at);
    }

    #[test]
    fn vrchat_order_keeps_the_world() {
        // VRChat logs "Entering Room" before "Joining wrld_...".
        let mut t = InstanceTracker::default();
        room(&mut t, 0);
        t.apply(&join("Fox"), M);
        t.apply(&EnteringRoom { world_name: "Pacific Dreams".into() }, 10 * M);
        t.apply(&JoiningWorld { world_id: "wrld_2".into(), instance: "2~hidden".into() }, 10 * M);
        t.apply(&join("Me"), 10 * M);
        t.apply(&join("Wolf"), 11 * M);
        assert_eq!(t.world_name.as_deref(), Some("Pacific Dreams"));
        assert_eq!(t.world_id.as_deref(), Some("wrld_2"));
        let others: Vec<_> = t.others().iter().map(|p| p.name.clone()).collect();
        assert_eq!(others, vec!["Wolf".to_string()], "Fox stayed in the old instance");
        // And the very first room of a session.
        let mut t = InstanceTracker::default();
        t.apply(&LocalUser { name: "Me".into(), user_id: Some("usr_Me".into()) }, 0);
        t.apply(&EnteringRoom { world_name: "Home".into() }, 0);
        t.apply(&JoiningWorld { world_id: "wrld_1".into(), instance: "1".into() }, 0);
        assert_eq!(t.world_name.as_deref(), Some("Home"));
    }

    #[test]
    fn one_other_settled_is_guessed() {
        let mut t = InstanceTracker::default();
        room(&mut t, 0);
        t.apply(&join("Fox"), M);
        assert_eq!(t.guess_partner(5 * M, DEFAULT_SETTLE_MS), PartnerGuess::Unknown, "not settled yet");
        match t.guess_partner(20 * M, DEFAULT_SETTLE_MS) {
            PartnerGuess::Guessed { person } => assert_eq!(person.name, "Fox"),
            g => panic!("expected guess, got {g:?}"),
        }
    }

    #[test]
    fn two_others_is_unknown() {
        let mut t = InstanceTracker::default();
        room(&mut t, 0);
        t.apply(&join("Fox"), M);
        t.apply(&join("Wolf"), M);
        assert_eq!(t.guess_partner(30 * M, DEFAULT_SETTLE_MS), PartnerGuess::Unknown);
    }

    #[test]
    fn leave_mid_event_is_unknown() {
        let mut t = InstanceTracker::default();
        room(&mut t, 0);
        t.apply(&join("Fox"), M);
        t.apply(&join("Wolf"), M);
        t.apply(&left("Wolf"), 25 * M);
        assert_eq!(t.guess_partner(28 * M, DEFAULT_SETTLE_MS), PartnerGuess::Unknown);
        // Later it settles again with just Fox.
        assert!(matches!(t.guess_partner(40 * M, DEFAULT_SETTLE_MS), PartnerGuess::Guessed { .. }));
    }

    #[test]
    fn alone_is_unknown() {
        let mut t = InstanceTracker::default();
        room(&mut t, 0);
        assert_eq!(t.guess_partner(30 * M, DEFAULT_SETTLE_MS), PartnerGuess::Unknown);
    }

    #[test]
    fn local_detected_without_authenticated_line() {
        let mut t = InstanceTracker::default();
        t.apply(&JoiningWorld { world_id: "wrld_1".into(), instance: "1".into() }, 0);
        t.apply(&join("Me"), 0);
        t.apply(&join("Fox"), 0);
        assert_eq!(t.local.as_ref().unwrap().name, "Me");
        assert_eq!(t.others().len(), 1);
    }

    #[test]
    fn closing_vrchat_empties_the_instance() {
        let mut t = InstanceTracker::default();
        room(&mut t, 0);
        t.apply(&join("Fox"), M);
        t.vrchat_closed(2 * M);
        assert!(t.others().is_empty());
        assert!(t.world_name.is_none());
        assert!(t.candidates(3 * M).is_empty());
        assert_eq!(t.history[0].name, "Fox", "still remembered as seen lately");
    }

    #[test]
    fn candidates_are_only_this_instance() {
        let mut t = InstanceTracker::default();
        room(&mut t, 0);
        t.apply(&join("Old"), M);
        t.apply(&LeftRoom, 2 * M);
        room(&mut t, 3 * M);
        t.apply(&join("Fox"), 4 * M);
        t.apply(&join("Wolf"), 5 * M);
        t.apply(&left("Wolf"), 6 * M);
        let c: Vec<String> = t.candidates(10 * M).into_iter().map(|p| p.name).collect();
        // Only this instance: Old from the previous instance is not listed.
        assert_eq!(c, vec!["Fox", "Wolf"]);
    }

    #[test]
    fn avatar_names_are_tracked() {
        let mut t = InstanceTracker::default();
        room(&mut t, 0);
        t.apply(&join("Fox"), M);
        t.apply(&AvatarSwitch { player: "Fox".into(), avatar_name: "Kitsune Deluxe".into() }, M);
        assert_eq!(t.others()[0].avatar_name.as_deref(), Some("Kitsune Deluxe"));
    }

    #[test]
    fn forget_partner() {
        let mut t = InstanceTracker::default();
        room(&mut t, 0);
        t.apply(&join("Fox"), M);
        t.apply(&LeftRoom, 2 * M);
        t.forget("Fox", Some("usr_Fox"));
        assert!(t.history.is_empty());
    }
}
