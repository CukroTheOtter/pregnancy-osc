//! Settings > Troubleshooting > Copy details: a short report to paste when
//! asking for help. Comparing VRChat's live list with its saved file and the
//! app's copy shows whose list is wrong. Nothing about other people on purpose:
//! no partners, no one in the instance, no worlds, no WooHoo history.

use crate::mapping::Mapping;
use crate::safety::InstanceKind;
use std::collections::BTreeMap;

pub struct Input<'a> {
    pub version: &'a str,
    pub vrchat_running: bool,
    pub linked: bool,
    pub oscquery: bool,
    pub read_logs: bool,
    pub last_heard_secs: Option<i64>,
    pub avatar_id: Option<&'a str>,
    pub avatar_name: Option<&'a str>,
    pub loading: bool,
    /// What VRChat reports over OSCQuery right now.
    pub live: Option<&'a [String]>,
    /// VRChat's saved OSC file for this avatar.
    pub file: Option<&'a [String]>,
    /// The app's saved copy.
    pub saved: Option<&'a [String]>,
    /// This avatar's own parameter names.
    pub own_names: &'a BTreeMap<String, String>,
    pub mapping: &'a Mapping,
    pub sps_sockets: usize,
    pub instance_kind: Option<InstanceKind>,
    pub hide_belly: bool,
    pub auto_hidden: bool,
    pub shown_here: bool,
    pub labor_held: bool,
}

fn yes(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

/// "142 parameters, belly found, 12 for the app".
fn list_line(list: Option<&[String]>, missing: &str, own: &BTreeMap<String, String>, mapping: &Mapping) -> String {
    let Some(list) = list.filter(|l| !l.is_empty()) else { return missing.to_string() };
    let is_app = |n: &String| mapping.key_for_name(n).is_some() || own.values().any(|o| o == n);
    let belly = list.iter().any(|n| mapping.key_for_name(n) == Some("stage") || own.get("stage") == Some(n));
    format!("{} parameters, belly {}, {} for the app", list.len(), if belly { "found" } else { "missing" }, list.iter().filter(|n| is_app(n)).count())
}

fn kind_name(k: InstanceKind) -> &'static str {
    match k {
        InstanceKind::Public => "Public",
        InstanceKind::GroupPublic => "Group Public",
        InstanceKind::FriendsPlus => "Friends+",
        InstanceKind::GroupPlus => "Group+",
        InstanceKind::Friends => "Friends",
        InstanceKind::Group => "Group",
        InstanceKind::InvitePlus => "Invite+",
        InstanceKind::Invite => "Invite",
    }
}

pub fn render(i: &Input) -> String {
    let lines = [
        format!("Pregnancy OSC {}", i.version),
        format!("VRChat running: {}", yes(i.vrchat_running)),
        format!("Linked: {} ({})", yes(i.linked), if i.oscquery { "OSCQuery" } else { "fixed ports" }),
        format!("Last heard from VRChat: {}", i.last_heard_secs.map_or("never".to_string(), |s| format!("{s} s ago"))),
        format!("Reading VRChat logs: {}", if i.read_logs { "on" } else { "off" }),
        match (i.avatar_name, i.avatar_id) {
            (Some(n), Some(id)) => format!("Avatar: {n} ({id})"),
            (None, Some(id)) => format!("Avatar: {id}"),
            _ => "Avatar: none yet".to_string(),
        },
        format!("Loading: {}", yes(i.loading)),
        format!("VRChat live list: {}", list_line(i.live, "not read yet", i.own_names, i.mapping)),
        format!("VRChat's saved file: {}", list_line(i.file, "not found", i.own_names, i.mapping)),
        format!("Saved in the app: {}", list_line(i.saved, "not read yet", i.own_names, i.mapping)),
        format!("SPS sockets: {}", i.sps_sockets),
        format!("Instance type: {}", i.instance_kind.map_or("unknown", kind_name)),
        format!("Hide Belly: {}", if i.hide_belly { "on" } else { "off" }),
        format!("Safety: belly hidden {}{}, labor waiting {}", yes(i.auto_hidden), if i.shown_here { " (shown here)" } else { "" }, yes(i.labor_held)),
    ];
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_whose_list_is_wrong() {
        let mapping = Mapping::default();
        let own = BTreeMap::new();
        let live = vec!["AFK".to_string(), "Upright".to_string()];
        let file = vec!["AFK".to_string(), "Pregnancy/Stage".to_string(), "Pregnancy/Labor".to_string()];
        let text = render(&Input {
            version: "1.2.3",
            vrchat_running: true,
            linked: true,
            oscquery: true,
            read_logs: true,
            last_heard_secs: Some(3),
            avatar_id: Some("avtr_1"),
            avatar_name: Some("Bunny"),
            loading: false,
            live: Some(&live),
            file: Some(&file),
            saved: None,
            own_names: &own,
            mapping: &mapping,
            sps_sockets: 2,
            instance_kind: Some(InstanceKind::FriendsPlus),
            hide_belly: false,
            auto_hidden: false,
            shown_here: false,
            labor_held: true,
        });
        assert_eq!(
            text,
            "Pregnancy OSC 1.2.3\nVRChat running: yes\nLinked: yes (OSCQuery)\nLast heard from VRChat: 3 s ago\nReading VRChat logs: on\nAvatar: Bunny (avtr_1)\nLoading: no\nVRChat live list: 2 parameters, belly missing, 0 for the app\nVRChat's saved file: 3 parameters, belly found, 2 for the app\nSaved in the app: not read yet\nSPS sockets: 2\nInstance type: Friends+\nHide Belly: off\nSafety: belly hidden no, labor waiting yes"
        );
    }

    #[test]
    fn an_avatars_own_names_count() {
        let mapping = Mapping::default();
        let own = BTreeMap::from([("stage".to_string(), "Custom/Belly".to_string())]);
        let list = vec!["Custom/Belly".to_string()];
        assert_eq!(list_line(Some(&list), "-", &own, &mapping), "1 parameters, belly found, 1 for the app");
        assert_eq!(list_line(Some(&[]), "not read yet", &own, &mapping), "not read yet");
    }
}
