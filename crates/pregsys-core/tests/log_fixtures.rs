//! Runs the sample logs (Windows CRLF and Proton LF) through the parser and
//! instance tracker end to end.

use pregsys_core::instance::{InstanceTracker, PartnerGuess, DEFAULT_SETTLE_MS};
use pregsys_core::logparse::{parse_text, LogEvent};

fn replay(text: &str) -> (InstanceTracker, i64) {
    let mut t = InstanceTracker::default();
    let mut last = 0;
    for l in parse_text(text) {
        let at = l.time_ms.expect("fixture lines have timestamps");
        t.apply(&l.event, at);
        last = at;
    }
    (t, last)
}

#[test]
fn windows_log_one_partner() {
    let text = include_str!("fixtures/windows_sample.txt");
    assert!(text.contains("\r\n"), "fixture should use CRLF");
    let events = parse_text(text);
    assert_eq!(events.iter().filter(|l| matches!(l.event, LogEvent::PlayerJoined { .. })).count(), 3);

    let (t, last) = replay(text);
    assert_eq!(t.local.as_ref().unwrap().name, "Moonbun");
    assert_eq!(t.world_name.as_deref(), Some("Cozy Cabin"));
    // Grey left at 21:05:20 (the last event). Right after, the room isn't settled.
    assert_eq!(t.guess_partner(last + 60_000, DEFAULT_SETTLE_MS), PartnerGuess::Unknown);
    // 25 minutes later it is.
    match t.guess_partner(last + 25 * 60_000, DEFAULT_SETTLE_MS) {
        PartnerGuess::Guessed { person } => {
            assert_eq!(person.name, "Ember Fox");
            assert_eq!(person.avatar_name.as_deref(), Some("Kitsune Deluxe (Quest)"));
        }
        g => panic!("expected a guess, got {g:?}"),
    }
    let names: Vec<_> = t.candidates(last).into_iter().map(|p| p.name).collect();
    assert_eq!(names, vec!["Ember Fox", "Grey"]);
}

#[test]
fn proton_log_two_others_then_alone() {
    let text = include_str!("fixtures/proton_sample.txt");
    assert!(!text.contains('\r'));
    let (t, last) = replay(text);
    // Now alone at Home: no partner and no one to pick (Rex and Scales were in the last instance).
    assert_eq!(t.world_name.as_deref(), Some("Home"));
    assert_eq!(t.guess_partner(last + DEFAULT_SETTLE_MS, DEFAULT_SETTLE_MS), PartnerGuess::Unknown);
    assert!(t.candidates(last).is_empty());
    assert_eq!(t.history.len(), 2);
}

#[test]
fn proton_log_mid_session_is_unknown() {
    let text = include_str!("fixtures/proton_sample.txt");
    // Cut the log while Rex and Scales were both present.
    let cut: String = text.lines().take(10).map(|l| format!("{l}\n")).collect();
    let (t, last) = replay(&cut);
    assert_eq!(t.others().len(), 2);
    let c = t.candidates(last);
    assert!(c.iter().any(|p| p.name == "Scales" && p.avatar_name.as_deref() == Some("Dragon Kin v4")));
    assert_eq!(t.guess_partner(last + 30 * 60_000, DEFAULT_SETTLE_MS), PartnerGuess::Unknown);
}
