//! VRChat log line parser.
//!
//! Read-only and local: logs are never uploaded. Tolerates format drift by
//! matching on short markers instead of exact layouts, and returns `None`
//! for anything it doesn't recognise. It never panics on odd input.

use chrono::{Local, NaiveDateTime, TimeZone};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LogEvent {
    /// The local user (`User Authenticated: Name (usr_...)`).
    LocalUser { name: String, user_id: Option<String> },
    /// `Joining wrld_...:instance~...`
    JoiningWorld { world_id: String, instance: String },
    /// `Entering Room: World Name`
    EnteringRoom { world_name: String },
    /// The local user left the instance.
    LeftRoom,
    PlayerJoined { name: String, user_id: Option<String> },
    PlayerLeft { name: String, user_id: Option<String> },
    /// `Switching Name to avatar Avatar Name` (avatar NAME only, no ID).
    AvatarSwitch { player: String, avatar_name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    /// Local log time as Unix ms, if the line had a readable timestamp.
    pub time_ms: Option<i64>,
    pub event: LogEvent,
}

/// `2024.01.15 20:31:45` at the start of a line, read as local time.
fn parse_time(line: &str) -> Option<i64> {
    let ts = line.get(0..19)?;
    let naive = NaiveDateTime::parse_from_str(ts, "%Y.%m.%d %H:%M:%S").ok()?;
    Local.from_local_datetime(&naive).earliest().map(|d| d.timestamp_millis())
}

/// Splits `Name (usr_xxx)` into the name and the user ID.
fn split_user(s: &str) -> (String, Option<String>) {
    let s = s.trim();
    if let Some(open) = s.rfind(" (usr_") {
        if s.ends_with(')') {
            let id = &s[open + 2..s.len() - 1];
            return (s[..open].trim().to_string(), Some(id.to_string()));
        }
    }
    (s.to_string(), None)
}

fn after<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    line.find(marker).map(|i| &line[i + marker.len()..])
}

pub fn parse_line(raw: &str) -> Option<LogLine> {
    let line = raw.trim_end_matches(['\r', '\n']);
    if line.len() < 10 {
        return None;
    }
    let event = parse_event(line)?;
    Some(LogLine { time_ms: parse_time(line), event })
}

fn parse_event(line: &str) -> Option<LogEvent> {
    // Most lines we care about are tagged [Behaviour]; skip the rest cheaply.
    if !line.contains("[Behaviour]") && !line.contains("User Authenticated") {
        return None;
    }
    if let Some(rest) = after(line, "OnPlayerJoined ") {
        let (name, user_id) = split_user(rest);
        return (!name.is_empty()).then_some(LogEvent::PlayerJoined { name, user_id });
    }
    if let Some(rest) = after(line, "OnPlayerLeft ") {
        let (name, user_id) = split_user(rest);
        return (!name.is_empty()).then_some(LogEvent::PlayerLeft { name, user_id });
    }
    if line.contains("OnLeftRoom") {
        return Some(LogEvent::LeftRoom);
    }
    if let Some(rest) = after(line, "Joining wrld_") {
        let full = format!("wrld_{}", rest.trim());
        let (world, instance) = full.split_once(':').unwrap_or((&full, ""));
        return Some(LogEvent::JoiningWorld { world_id: world.to_string(), instance: instance.to_string() });
    }
    if let Some(rest) = after(line, "Entering Room: ") {
        return Some(LogEvent::EnteringRoom { world_name: rest.trim().to_string() });
    }
    if let Some(rest) = after(line, "User Authenticated: ") {
        let (name, user_id) = split_user(rest);
        return Some(LogEvent::LocalUser { name, user_id });
    }
    if let Some(rest) = after(line, "Switching ") {
        let (player, avatar) = rest.split_once(" to avatar ")?;
        return Some(LogEvent::AvatarSwitch { player: player.trim().to_string(), avatar_name: avatar.trim().to_string() });
    }
    None
}

/// Parses a whole chunk of text. Handles CRLF (Windows) and LF (Proton).
pub fn parse_text(text: &str) -> Vec<LogLine> {
    text.lines().filter_map(parse_line).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(s: &str) -> Option<LogEvent> {
        parse_line(s).map(|l| l.event)
    }

    #[test]
    fn player_join_with_id() {
        assert_eq!(
            ev("2025.03.01 21:00:05 Debug      -  [Behaviour] OnPlayerJoined Foxy Paws (usr_1a2b3c4d-0000-1111-2222-333344445555)"),
            Some(LogEvent::PlayerJoined { name: "Foxy Paws".into(), user_id: Some("usr_1a2b3c4d-0000-1111-2222-333344445555".into()) })
        );
    }

    #[test]
    fn player_join_old_format_without_id() {
        assert_eq!(
            ev("2023.05.01 21:00:05 Log        -  [Behaviour] OnPlayerJoined Someone"),
            Some(LogEvent::PlayerJoined { name: "Someone".into(), user_id: None })
        );
    }

    #[test]
    fn names_with_parentheses() {
        let e = ev("2025.03.01 21:00:05 Debug      -  [Behaviour] OnPlayerLeft Bun (AFK) (usr_abc)");
        assert_eq!(e, Some(LogEvent::PlayerLeft { name: "Bun (AFK)".into(), user_id: Some("usr_abc".into()) }));
    }

    #[test]
    fn left_room_is_not_player_left() {
        assert_eq!(ev("2025.03.01 21:00:05 Debug      -  [Behaviour] OnLeftRoom"), Some(LogEvent::LeftRoom));
        assert_eq!(ev("2025.03.01 21:00:05 Debug      -  [Behaviour] OnPlayerLeftRoom Foo"), None);
    }

    #[test]
    fn world_join() {
        assert_eq!(
            ev("2025.03.01 21:00:00 Debug      -  [Behaviour] Joining wrld_4cf554b4-430c-4f8f-b53e-1f294eed230b:12345~private(usr_x)~region(eu)"),
            Some(LogEvent::JoiningWorld {
                world_id: "wrld_4cf554b4-430c-4f8f-b53e-1f294eed230b".into(),
                instance: "12345~private(usr_x)~region(eu)".into()
            })
        );
        assert_eq!(
            ev("2025.03.01 21:00:01 Debug      -  [Behaviour] Entering Room: The Black Cat"),
            Some(LogEvent::EnteringRoom { world_name: "The Black Cat".into() })
        );
    }

    #[test]
    fn avatar_switch() {
        assert_eq!(
            ev("2025.03.01 21:01:00 Debug      -  [Behaviour] Switching Foxy Paws to avatar Kitsune v3.2"),
            Some(LogEvent::AvatarSwitch { player: "Foxy Paws".into(), avatar_name: "Kitsune v3.2".into() })
        );
    }

    #[test]
    fn local_user() {
        assert_eq!(
            ev("2025.03.01 20:59:00 Debug      -  User Authenticated: Me Myself (usr_me)"),
            Some(LogEvent::LocalUser { name: "Me Myself".into(), user_id: Some("usr_me".into()) })
        );
    }

    #[test]
    fn timestamp_is_read() {
        let l = parse_line("2025.03.01 21:00:05 Debug      -  [Behaviour] OnPlayerJoined A (usr_a)").unwrap();
        assert!(l.time_ms.is_some());
        let l2 = parse_line("garbled 21:00:05 [Behaviour] OnPlayerJoined A (usr_a)").unwrap();
        assert!(l2.time_ms.is_none(), "event still parsed without a timestamp");
    }

    #[test]
    fn junk_never_panics() {
        for s in ["", "\r\n", "[Behaviour]", "[Behaviour] Switching", "[Behaviour] OnPlayerJoined ", "ééé [Behaviour] Joining wrld_", "\u{0}\u{1}\u{2}"] {
            let _ = parse_line(s);
        }
        assert_eq!(ev("2025.03.01 21:00:05 Debug      -  [Behaviour] OnPlayerJoined "), None);
    }
}
