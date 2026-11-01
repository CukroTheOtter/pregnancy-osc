//! Safety: what kind of instance you're in (from the log's "Joining" line),
//! and where the belly is hidden and labor waits. Never touches the saved
//! Sync values: leave a public instance and your own Hide Belly is back.

use serde::{Deserialize, Serialize};

/// VRChat's instance types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstanceKind {
    Public,
    FriendsPlus,
    Friends,
    InvitePlus,
    Invite,
    GroupPublic,
    GroupPlus,
    Group,
}

impl InstanceKind {
    pub const ALL: [InstanceKind; 8] = [
        InstanceKind::Public,
        InstanceKind::GroupPublic,
        InstanceKind::FriendsPlus,
        InstanceKind::GroupPlus,
        InstanceKind::Friends,
        InstanceKind::Group,
        InstanceKind::InvitePlus,
        InstanceKind::Invite,
    ];

    /// The kind from the part after `wrld_…:` (`12345~private(usr_x)~region(eu)`).
    /// None when there's no instance (not in a world).
    pub fn of(instance: &str) -> Option<Self> {
        if instance.trim().is_empty() {
            return None;
        }
        let has = |tag: &str| instance.contains(&format!("~{tag}"));
        Some(if has("private(") {
            if has("canRequestInvite") { Self::InvitePlus } else { Self::Invite }
        } else if has("friends(") {
            Self::Friends
        } else if has("hidden(") {
            Self::FriendsPlus
        } else if has("group(") {
            if has("groupAccessType(public)") {
                Self::GroupPublic
            } else if has("groupAccessType(plus)") {
                Self::GroupPlus
            } else {
                Self::Group
            }
        } else {
            Self::Public
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SafetySettings {
    /// The belly is hidden in these instances (your Hide Belly stays as it is).
    pub hide_belly_in: Vec<InstanceKind>,
    /// Labor doesn't start, and labor under way waits, in these instances.
    pub labor_waits_in: Vec<InstanceKind>,
}

impl Default for SafetySettings {
    fn default() -> Self {
        let public = vec![InstanceKind::Public, InstanceKind::GroupPublic];
        Self { hide_belly_in: public.clone(), labor_waits_in: public }
    }
}

impl SafetySettings {
    /// The belly is hidden in this instance.
    pub fn hides(&self, instance: Option<&str>) -> bool {
        instance.and_then(InstanceKind::of).is_some_and(|k| self.hide_belly_in.contains(&k))
    }

    /// Labor waits in this instance.
    pub fn holds_labor(&self, instance: Option<&str>) -> bool {
        instance.and_then(InstanceKind::of).is_some_and(|k| self.labor_waits_in.contains(&k))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use InstanceKind::*;

    #[test]
    fn instance_kinds_from_the_log() {
        assert_eq!(InstanceKind::of("12345"), Some(Public));
        assert_eq!(InstanceKind::of("12345~region(eu)"), Some(Public));
        assert_eq!(InstanceKind::of("1~hidden(usr_x)~region(us)"), Some(FriendsPlus));
        assert_eq!(InstanceKind::of("1~friends(usr_x)~region(us)"), Some(Friends));
        assert_eq!(InstanceKind::of("1~private(usr_x)~canRequestInvite~region(eu)"), Some(InvitePlus));
        assert_eq!(InstanceKind::of("12345~private(usr_x)~region(eu)"), Some(Invite));
        assert_eq!(InstanceKind::of("1~group(grp_x)~groupAccessType(public)~region(us)"), Some(GroupPublic));
        assert_eq!(InstanceKind::of("1~group(grp_x)~groupAccessType(plus)"), Some(GroupPlus));
        assert_eq!(InstanceKind::of("1~group(grp_x)~groupAccessType(members)"), Some(Group));
        assert_eq!(InstanceKind::of(""), None);
    }

    #[test]
    fn public_by_default() {
        let s = SafetySettings::default();
        assert!(s.hides(Some("1~region(eu)")) && s.holds_labor(Some("1~region(eu)")));
        assert!(!s.hides(Some("1~private(usr_x)")) && !s.holds_labor(Some("1~friends(usr_x)")));
        assert!(!s.hides(None), "not in a world (or no log): nothing changes");
        let off = SafetySettings { hide_belly_in: vec![], labor_waits_in: vec![Friends] };
        assert!(!off.hides(Some("1")) && off.holds_labor(Some("1~friends(usr_x)")));
    }
}
