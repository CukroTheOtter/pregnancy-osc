//! Detecting WooHoos from VRCFury SPS sockets.
//!
//! VRCFury's haptics (the ones OSC Goes Brrr reads) expose each socket as
//! `OGB/Orf/<socket>/<channel>` floats. "Others" channels mean another
//! player; "Self" channels are your own and never count.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PREFIX: &str = "OGB/Orf/";

/// VRCFury leaves the `OGB/` channels at 0 until an OSC app sets this true
/// (OSC Goes Brrr does). VRChat doesn't keep it, so it resets on avatar load.
pub const ENABLE_PARAM: &str = "OGB_ENABLED";

/// Channels that mean "another player is inside", best first.
const OTHERS_CHANNELS: [&str; 3] = ["PenOthersNewRoot", "PenOthers", "PenOthersNewTip"];

/// Splits `OGB/Orf/<socket>/<channel>` into (socket, channel).
pub fn split(name: &str) -> Option<(&str, &str)> {
    let rest = name.strip_prefix(PREFIX)?;
    let (socket, channel) = rest.split_once('/')?;
    (!socket.is_empty() && !channel.is_empty()).then_some((socket, channel))
}

/// True if this channel means another player (not you) is inside.
pub fn is_others_channel(channel: &str) -> bool {
    OTHERS_CHANNELS.contains(&channel)
}

/// Sockets found among an avatar's parameter names, each with the
/// channel the app will watch.
pub fn sockets(params: &[String]) -> BTreeMap<String, String> {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    for p in params {
        let Some((socket, channel)) = split(p) else { continue };
        let Some(rank) = OTHERS_CHANNELS.iter().position(|c| *c == channel) else { continue };
        let better = out.get(socket).is_none_or(|cur| OTHERS_CHANNELS.iter().position(|c| c == cur).unwrap_or(9) > rank);
        if better {
            out.insert(socket.to_string(), channel.to_string());
        }
    }
    out
}

/// What kind of hole a socket is, guessed from its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum HoleType {
    Vaginal,
    Anal,
    Oral,
    #[default]
    Other,
}

const ORAL: [&str; 6] = ["mouth", "oral", "throat", "blowjob", "lips", "face"];
const ANAL: [&str; 6] = ["anal", "anus", "ass", "butt", "rear", "booty"];
const VAGINAL: [&str; 8] = ["vagina", "vag", "pussy", "womb", "cervix", "vulva", "cunt", "front"];

/// Guesses the hole type from a socket name ("SPS Pussy", "Mouth", "AnalSocket"...).
pub fn classify(socket: &str) -> HoleType {
    let n = socket.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| n.contains(w));
    if has(&ORAL) {
        HoleType::Oral
    } else if has(&ANAL) {
        HoleType::Anal
    } else if has(&VAGINAL) {
        HoleType::Vaginal
    } else {
        HoleType::Other
    }
}

/// Which hole types count (global, for every avatar).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HoleTypes {
    pub vaginal: bool,
    pub anal: bool,
    pub oral: bool,
    pub other: bool,
}

impl Default for HoleTypes {
    fn default() -> Self {
        Self { vaginal: true, anal: false, oral: false, other: false }
    }
}

impl HoleTypes {
    pub fn allows(&self, t: HoleType) -> bool {
        match t {
            HoleType::Vaginal => self.vaginal,
            HoleType::Anal => self.anal,
            HoleType::Oral => self.oral,
            HoleType::Other => self.other,
        }
    }
}

/// Per-avatar SPS choices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AvatarSps {
    /// Pick sockets for this avatar instead of using the global hole types.
    pub custom: bool,
    /// Sockets that count when `custom` is on.
    pub sockets: Vec<String>,
    /// Hole type fixes for sockets with odd names.
    pub types: BTreeMap<String, HoleType>,
}

impl AvatarSps {
    pub fn type_of(&self, socket: &str) -> HoleType {
        self.types.get(socket).copied().unwrap_or_else(|| classify(socket))
    }
}

/// Does this socket count on this avatar?
pub fn socket_counts(s: &SpsSettings, avatar: Option<&AvatarSps>, socket: &str) -> bool {
    match avatar {
        Some(a) if a.custom => a.sockets.iter().any(|x| x == socket),
        Some(a) => s.types.allows(a.type_of(socket)),
        None => s.types.allows(classify(socket)),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SpsSettings {
    pub enabled: bool,
    /// Hole types that count on every avatar (unless an avatar picks its own sockets).
    pub types: HoleTypes,
    /// How deep counts, 0 to 1.
    pub depth: f64,
    /// Seconds of being inside (in total) before it counts.
    pub seconds: u32,
    /// Quiet seconds that end a session (and allow another WooHoo).
    pub gap_seconds: u32,
}

impl Default for SpsSettings {
    fn default() -> Self {
        Self { enabled: true, types: HoleTypes::default(), depth: 0.3, seconds: 20, gap_seconds: 120 }
    }
}

/// Live state of one socket (for the meters in Settings).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SocketLive {
    pub name: String,
    pub channel: String,
    pub value: f32,
    pub hole_type: HoleType,
    pub counts: bool,
}

#[derive(Debug, Default)]
pub struct Detector {
    /// socket -> (channel, value, when it last changed; None = since the last tick).
    values: BTreeMap<String, (String, f32, Option<i64>)>,
    last_ms: Option<i64>,
    inside_ms: i64,
    last_inside_ms: Option<i64>,
    fired: bool,
}

impl Detector {
    /// Forget everything (new avatar).
    pub fn reset(&mut self) {
        *self = Detector::default();
    }

    /// End the current session, so the next WooHoo counts again (after a reveal).
    pub fn end_session(&mut self) {
        self.inside_ms = 0;
        self.last_inside_ms = None;
        self.fired = false;
    }

    /// A socket value from VRChat. Returns true if this name was an SPS channel.
    pub fn on_param(&mut self, name: &str, v: f32) -> bool {
        let Some((socket, channel)) = split(name) else { return false };
        if !is_others_channel(channel) {
            return true;
        }
        // Keep the best channel per socket.
        let keep = self.values.get(socket).is_none_or(|(cur, _, _)| {
            OTHERS_CHANNELS.iter().position(|c| c == cur) >= OTHERS_CHANNELS.iter().position(|c| *c == channel)
        });
        if keep {
            self.values.insert(socket.to_string(), (channel.to_string(), v, None));
        }
        true
    }

    /// Advances time. Returns true once per session when it's a WooHoo.
    pub fn tick(&mut self, now_ms: i64, s: &SpsSettings, avatar: Option<&AvatarSps>) -> bool {
        let dt = self.last_ms.map(|t| (now_ms - t).clamp(0, 5_000)).unwrap_or(0);
        self.last_ms = Some(now_ms);
        if !s.enabled {
            return false;
        }
        for (_, _, t) in self.values.values_mut() {
            t.get_or_insert(now_ms);
        }
        // VRChat only sends changes: a value stuck for a whole gap (its 0 got
        // lost, or the partner left) doesn't count, so the session can end.
        let gap_ms = s.gap_seconds as i64 * 1000;
        let inside = self.values.iter().any(|(sock, (_, v, t))| {
            socket_counts(s, avatar, sock) && *v as f64 >= s.depth && t.is_some_and(|t| now_ms - t < gap_ms)
        });
        if inside {
            self.inside_ms += dt;
            self.last_inside_ms = Some(now_ms);
        } else if self.last_inside_ms.is_some_and(|t| now_ms - t >= s.gap_seconds as i64 * 1000) {
            // Quiet long enough: the session is over.
            self.inside_ms = 0;
            self.last_inside_ms = None;
            self.fired = false;
        }
        if !self.fired && self.inside_ms >= s.seconds as i64 * 1000 {
            self.fired = true;
            return true;
        }
        false
    }

    /// 0 to 1: how close the current session is to counting.
    pub fn progress(&self, s: &SpsSettings) -> f64 {
        if self.fired {
            return 1.0;
        }
        (self.inside_ms as f64 / (s.seconds.max(1) as f64 * 1000.0)).min(1.0)
    }

    pub fn live(&self, s: &SpsSettings, avatar: Option<&AvatarSps>) -> Vec<SocketLive> {
        self.values
            .iter()
            .map(|(name, (channel, value, _))| SocketLive {
                name: name.clone(),
                channel: channel.clone(),
                value: *value,
                hole_type: avatar.map(|a| a.type_of(name)).unwrap_or_else(|| classify(name)),
                counts: socket_counts(s, avatar, name),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn finds_sockets_and_best_channel() {
        let found = sockets(&names(&[
            "OGB/Orf/Vagina/PenOthers",
            "OGB/Orf/Vagina/PenOthersNewRoot",
            "OGB/Orf/Vagina/PenSelfNewRoot",
            "OGB/Orf/Mouth/PenOthersNewTip",
            "OGB/Pen/Toy/TouchOthers",
            "Pregnancy/Stage",
        ]));
        assert_eq!(found.get("Vagina").map(String::as_str), Some("PenOthersNewRoot"));
        assert_eq!(found.get("Mouth").map(String::as_str), Some("PenOthersNewTip"));
        assert_eq!(found.len(), 2, "plugs and self channels are ignored");
    }

    #[test]
    fn counts_after_enough_time_once_per_session() {
        let s = SpsSettings { seconds: 10, gap_seconds: 60, ..Default::default() };
        // "Vagina" is a vaginal socket, which counts by default.
        let mut d = Detector::default();
        d.tick(0, &s, None);
        assert!(d.on_param("OGB/Orf/Vagina/PenOthersNewRoot", 0.8));
        let mut fired = 0;
        for t in 1..=30 {
            if d.tick(t * 1000, &s, None) {
                fired += 1;
                assert_eq!(t, 10, "fires after 10 seconds inside");
            }
        }
        assert_eq!(fired, 1, "only once per session");
        // Quiet for a minute ends the session; a new one can count again.
        d.on_param("OGB/Orf/Vagina/PenOthersNewRoot", 0.0);
        for t in 31..=95 {
            assert!(!d.tick(t * 1000, &s, None));
        }
        d.on_param("OGB/Orf/Vagina/PenOthersNewRoot", 0.9);
        assert!((96..=110).any(|t| d.tick(t * 1000, &s, None)));
    }

    #[test]
    fn a_stuck_value_ends_the_session() {
        let s = SpsSettings { seconds: 10, gap_seconds: 60, ..Default::default() };
        let mut d = Detector::default();
        d.tick(0, &s, None);
        // The last value VRChat sent stays high (its 0 never came).
        d.on_param("OGB/Orf/Vagina/PenOthersNewRoot", 0.8);
        assert!((1..=30).any(|t| d.tick(t * 1000, &s, None)));
        for t in 31..=200 {
            assert!(!d.tick(t * 1000, &s, None));
        }
        assert_eq!(d.progress(&s), 0.0, "session over");
        d.on_param("OGB/Orf/Vagina/PenOthersNewRoot", 0.9);
        assert!((201..=215).any(|t| d.tick(t * 1000, &s, None)), "the next one counts");
    }

    #[test]
    fn a_reveal_ends_the_session() {
        let s = SpsSettings { seconds: 10, gap_seconds: 60, ..Default::default() };
        let mut d = Detector::default();
        d.tick(0, &s, None);
        d.on_param("OGB/Orf/Vagina/PenOthersNewRoot", 0.8);
        assert!((1..=15).any(|t| d.tick(t * 1000, &s, None)));
        d.end_session();
        d.on_param("OGB/Orf/Vagina/PenOthersNewRoot", 0.7);
        assert!((16..=30).any(|t| d.tick(t * 1000, &s, None)), "counts again without a gap");
    }

    #[test]
    fn shallow_self_and_unselected_dont_count() {
        let s = SpsSettings { seconds: 5, ..Default::default() };
        let mut d = Detector::default();
        d.tick(0, &s, None);
        d.on_param("OGB/Orf/Vagina/PenOthersNewRoot", 0.1); // too shallow
        d.on_param("OGB/Orf/Vagina/PenSelfNewRoot", 1.0); // your own
        d.on_param("OGB/Orf/Mouth/PenOthersNewRoot", 1.0); // oral: off by default
        assert!((1..=30).all(|t| !d.tick(t * 1000, &s, None)));
        assert!(d.live(&s, None).iter().any(|l| l.name == "Mouth" && !l.counts));
    }

    #[test]
    fn pauses_count_toward_the_total() {
        let s = SpsSettings { seconds: 10, gap_seconds: 120, ..Default::default() };
        let mut d = Detector::default();
        d.tick(0, &s, None);
        d.on_param("OGB/Orf/Pussy/PenOthers", 1.0);
        for t in 1..=6 {
            assert!(!d.tick(t * 1000, &s, None));
        }
        d.on_param("OGB/Orf/Pussy/PenOthers", 0.0);
        for t in 7..=30 {
            assert!(!d.tick(t * 1000, &s, None));
        }
        d.on_param("OGB/Orf/Pussy/PenOthers", 1.0);
        assert!((31..=40).any(|t| d.tick(t * 1000, &s, None)), "6 s + 4 s more = 10 s");
    }

    #[test]
    fn hole_types_from_names() {
        assert_eq!(classify("SPS Pussy"), HoleType::Vaginal);
        assert_eq!(classify("Vagina"), HoleType::Vaginal);
        assert_eq!(classify("AnalSocket"), HoleType::Anal);
        assert_eq!(classify("Mouth"), HoleType::Oral);
        assert_eq!(classify("Thighs"), HoleType::Other);
    }

    #[test]
    fn global_types_and_avatar_overrides() {
        let s = SpsSettings::default(); // vaginal only
        assert!(socket_counts(&s, None, "Pussy"));
        assert!(!socket_counts(&s, None, "Mouth"));
        // An avatar fixes an oddly named socket.
        let mut a = AvatarSps::default();
        a.types.insert("Hole1".into(), HoleType::Vaginal);
        assert!(socket_counts(&s, Some(&a), "Hole1"));
        // An avatar picks its own sockets: global types don't apply.
        let custom = AvatarSps { custom: true, sockets: vec!["Mouth".into()], ..Default::default() };
        assert!(socket_counts(&s, Some(&custom), "Mouth"));
        assert!(!socket_counts(&s, Some(&custom), "Pussy"));
    }

    #[test]
    fn off_means_never() {
        let s = SpsSettings { enabled: false, seconds: 1, ..Default::default() };
        let mut d = Detector::default();
        d.on_param("OGB/Orf/A/PenOthers", 1.0);
        assert!((0..20).all(|t| !d.tick(t * 1000, &s, None)));
    }
}
