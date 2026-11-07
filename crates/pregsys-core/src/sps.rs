//! Detecting WooHoos from VRCFury SPS sockets.
//!
//! VRCFury's haptics (the ones OSC Goes Brrr reads) expose each socket as
//! `OGB/Orf/<socket>/<channel>` floats. "Others" channels mean another
//! player; "Self" channels are your own and never count.
//!
//! SPS3 (VRCFury 1.1442 betas) uses `OGB/Socket/<socket>/...` instead: no
//! ready-made depth, but how far the plug tip is (`DepthOthers_PointSeparation`,
//! meters) and the plug's length, worked out here like OGB 2.1.31 does.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PREFIX: &str = "OGB/Orf/";

/// VRCFury leaves the `OGB/` channels at 0 until an OSC app sets this true
/// (OSC Goes Brrr does). VRChat doesn't keep it, so it resets on avatar load.
pub const ENABLE_PARAM: &str = "OGB_ENABLED";
/// SPS3's version of it (only gates touch contacts for now; sent anyway, like OGB).
pub const ENABLE_PARAM_V3: &str = "OGB/Enabled";
/// On avatars uploaded with VRCFury's haptics off: no socket values at all.
pub const HAPTICS_DISABLED: &str = "OGB/HapticsDisabled";

/// VRCFury adds `OGB/Version/<n>` to every haptics avatar (bumped when its
/// senders or receivers change). The newest one this app understands:
/// a higher one means the app may need an update (keep `src/lib/sps.ts` in step).
pub const KNOWN_VERSION: u32 = 11;
const VERSION_PREFIX: &str = "OGB/Version/";

/// The avatar's SPS haptics version, if it says (older avatars don't).
pub fn version(params: &[String]) -> Option<u32> {
    params.iter().filter_map(|p| p.strip_prefix(VERSION_PREFIX)?.parse().ok()).max()
}

/// True when the avatar's SPS is newer than this app knows.
pub fn newer_than_known(params: &[String]) -> bool {
    version(params).is_some_and(|v| v > KNOWN_VERSION)
}

/// SPS3 sockets: `OGB/Socket/<socket>/<key>`.
pub const V3_PREFIX: &str = "OGB/Socket/";
/// The channel name SPS3 sockets show up with.
pub const V3_CHANNEL: &str = "DepthOthers";
const V3_SEPARATION: &str = "DepthOthers_PointSeparation";
const V3_LENGTH: &str = "DepthOthers_SenderLossyScaleZ";
const V3_NATIVE_TAG: &str = "DepthOthers_CollisionTag_SPS/Plug/";
const V3_RADIUS: &str = "DepthRadius";

/// Channels that mean "another player is inside", best first.
const OTHERS_CHANNELS: [&str; 3] = ["PenOthersNewRoot", "PenOthers", "PenOthersNewTip"];

/// Splits `OGB/Orf/<socket>/<channel>` into (socket, channel).
pub fn split(name: &str) -> Option<(&str, &str)> {
    let rest = name.strip_prefix(PREFIX)?;
    let (socket, channel) = rest.split_once('/')?;
    (!socket.is_empty() && !channel.is_empty()).then_some((socket, channel))
}

/// Splits `OGB/Socket/<socket>/<key>` into (socket, key). Keys can hold slashes.
pub fn split_v3(name: &str) -> Option<(&str, &str)> {
    let rest = name.strip_prefix(V3_PREFIX)?;
    let (socket, key) = rest.split_once('/')?;
    (!socket.is_empty() && !key.is_empty()).then_some((socket, key))
}

/// True if this channel means another player (not you) is inside.
pub fn is_others_channel(channel: &str) -> bool {
    OTHERS_CHANNELS.contains(&channel)
}

/// SPS3 keys the depth of another player's plug is worked out from.
fn is_v3_others_key(key: &str) -> bool {
    matches!(key, V3_CHANNEL | V3_SEPARATION | V3_LENGTH | V3_RADIUS) || key.starts_with(V3_NATIVE_TAG)
}

/// Parameters VRChat should send the app for detecting WooHoos (both formats).
pub fn wanted(name: &str) -> bool {
    split(name).is_some_and(|(_, c)| is_others_channel(c)) || split_v3(name).is_some_and(|(_, k)| is_v3_others_key(k))
}

/// Parameters only worth reading from OSCQuery: they never change, so VRChat never sends them.
pub fn is_constant(name: &str) -> bool {
    split_v3(name).is_some_and(|(_, k)| k == V3_RADIUS)
}

/// Sockets found among an avatar's parameter names, each with the
/// channel the app will watch (SPS3 first, if an avatar somehow has both).
pub fn sockets(params: &[String]) -> BTreeMap<String, String> {
    let rank = |c: &str| if c == V3_CHANNEL { 0 } else { OTHERS_CHANNELS.iter().position(|x| *x == c).map_or(9, |i| i + 1) };
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    for p in params {
        let found = split_v3(p).filter(|(_, k)| *k == V3_CHANNEL).or_else(|| split(p).filter(|(_, c)| is_others_channel(c)));
        let Some((socket, channel)) = found else { continue };
        if out.get(socket).is_none_or(|cur| rank(cur) > rank(channel)) {
            out.insert(socket.to_string(), channel.to_string());
        }
    }
    out
}

/// What an SPS3 socket has heard about another player's plug.
#[derive(Debug, Default)]
struct V3Socket {
    /// 0 to 1: how close the plug is (> 0 means it's in range).
    proximity: Option<f32>,
    /// Meters from the plug's tip to the socket.
    separation: Option<f32>,
    /// The plug's length in meters (SPS3 plugs only).
    sender_length: Option<f32>,
    /// The socket's own receiver radius (for older plugs).
    radius: Option<f32>,
    /// `SPS/Plug/...` tags: any on means an SPS3 plug, whose length can be trusted.
    native_tags: BTreeMap<String, bool>,
    /// An older plug's length, guessed when it came close.
    legacy_length: Option<f32>,
}

impl V3Socket {
    fn set(&mut self, key: &str, v: f32) {
        let v = v.is_finite().then_some(v);
        match key {
            V3_CHANNEL => self.proximity = v,
            V3_SEPARATION => self.separation = v,
            V3_LENGTH => self.sender_length = v,
            V3_RADIUS => self.radius = v,
            _ => match key.strip_prefix(V3_NATIVE_TAG) {
                Some(tag) => {
                    self.native_tags.insert(tag.to_string(), v.is_some_and(|v| v > 0.0));
                }
                None => return,
            },
        }
        // Like OGB: every change but the sender length refreshes the guess for older plugs.
        if key != V3_LENGTH {
            self.plug_length();
        }
    }

    fn in_range(&self) -> Option<(f32, f32)> {
        match (self.proximity, self.separation) {
            (Some(p), Some(s)) if p > 0.0 && s >= 0.0 => Some((p, s)),
            _ => None,
        }
    }

    /// The plug's length: an SPS3 plug says it; an older one is guessed from
    /// how far it was on first contact (kept while it's all the way in).
    fn plug_length(&mut self) -> Option<f32> {
        let Some((proximity, separation)) = self.in_range() else {
            self.legacy_length = None;
            return None;
        };
        if self.native_tags.values().any(|on| *on) {
            self.legacy_length = None;
            return self.sender_length.filter(|l| *l > 0.0);
        }
        if proximity >= 1.0 && self.legacy_length.is_some() {
            return self.legacy_length;
        }
        let Some(radius) = self.radius.filter(|r| *r > 0.0) else { return self.legacy_length };
        let guess = separation - radius * (1.0 - proximity);
        self.legacy_length = (guess > 0.0 && guess <= 3.0).then_some(guess);
        self.legacy_length
    }

    /// 0 to 1: how much of the plug is inside.
    fn depth(&mut self) -> f32 {
        let Some((_, separation)) = self.in_range() else { return 0.0 };
        match self.plug_length() {
            Some(len) if len.is_finite() && len > 0.0 => (1.0 - separation / len).clamp(0.0, 1.0),
            _ => 0.0,
        }
    }
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
    /// SPS3 sockets' raw values, turned into a depth in `values`.
    v3: BTreeMap<String, V3Socket>,
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
        if let Some((socket, key)) = split_v3(name) {
            if is_v3_others_key(key) {
                let s = self.v3.entry(socket.to_string()).or_default();
                // The radius is read again on every OSCQuery poll: the same one isn't news
                // (it would keep a stuck value looking fresh).
                if key == V3_RADIUS && s.radius == Some(v) {
                    return true;
                }
                s.set(key, v);
                let depth = s.depth();
                self.values.insert(socket.to_string(), (V3_CHANNEL.to_string(), depth, None));
            }
            return true;
        }
        let Some((socket, channel)) = split(name) else { return false };
        if !is_others_channel(channel) {
            return true;
        }
        // Keep the best channel per socket (an SPS3 one beats them all).
        let keep = self.values.get(socket).is_none_or(|(cur, _, _)| {
            cur != V3_CHANNEL && OTHERS_CHANNELS.iter().position(|c| c == cur) >= OTHERS_CHANNELS.iter().position(|c| *c == channel)
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
    fn sps_version() {
        assert_eq!(version(&names(&["Pregnancy/Stage", "OGB/Orf/A/PenOthers"])), None, "older avatars don't say");
        assert_eq!(version(&names(&["OGB/Version/11", "OGB/Socket/A/DepthOthers"])), Some(11));
        assert!(!newer_than_known(&names(&["OGB/Version/11"])));
        assert!(newer_than_known(&names(&["OGB/Version/12"])));
        assert!(!newer_than_known(&names(&["OGB/Version/x", "OGB/Orf/A/Version/9"])), "junk and old per-socket versions are ignored");
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

    fn near(a: f32, b: f32) {
        assert!((a - b).abs() < 0.001, "{a} is not {b}");
    }

    fn depth(d: &Detector, socket: &str) -> f32 {
        d.values.get(socket).map(|(_, v, _)| *v).unwrap_or(0.0)
    }

    #[test]
    fn finds_sps3_sockets() {
        let found = sockets(&names(&[
            "OGB/Socket/Front_Socket/DepthOthers",
            "OGB/Socket/Front_Socket/DepthOthers_PointSeparation",
            "OGB/Socket/Front_Socket/DepthSelf",
            "OGB/Socket/Front_Socket/DepthRadius",
            "OGB/Socket/Mouth/TouchOthers",
            "OGB/Plug/Toy/DepthOthers",
            "OGB/Orf/Old/PenOthers",
        ]));
        assert_eq!(found.get("Front_Socket").map(String::as_str), Some("DepthOthers"));
        assert_eq!(found.get("Old").map(String::as_str), Some("PenOthers"));
        assert_eq!(found.len(), 2, "touch-only sockets and plugs are ignored");
        assert!(wanted("OGB/Socket/A/DepthOthers_CollisionTag_SPS/Plug/_public"));
        assert!(wanted("OGB/Socket/A/DepthRadius"));
        assert!(wanted("OGB/Orf/A/PenOthersNewRoot"));
        assert!(!wanted("OGB/Socket/A/DepthSelf"));
        assert!(!wanted("OGB/Socket/A/DepthOthers_SenderLossyScaleX"));
        assert!(is_constant("OGB/Socket/A/DepthRadius"));
    }

    #[test]
    fn sps3_plug_depth_from_its_length() {
        // From OGB's tests: a 0.4 m plug with its tip 0.1 m away is 75% in.
        let mut d = Detector::default();
        assert!(d.on_param("OGB/Socket/S/DepthOthers_CollisionTag_SPS/Plug/_public", 1.0));
        d.on_param("OGB/Socket/S/DepthOthers_SenderLossyScaleZ", 0.4);
        d.on_param("OGB/Socket/S/DepthOthers_PointSeparation", 0.1);
        d.on_param("OGB/Socket/S/DepthOthers", 1.0);
        near(depth(&d, "S"), 0.75);
        d.on_param("OGB/Socket/S/DepthOthers_SenderLossyScaleZ", 0.8);
        near(depth(&d, "S"), 0.875);
        d.on_param("OGB/Socket/S/DepthOthers", 0.0);
        near(depth(&d, "S"), 0.0);
        // Without a native tag the sender scale means nothing (and there's no radius to guess from).
        d.on_param("OGB/Socket/S/DepthOthers_CollisionTag_SPS/Plug/_public", 0.0);
        d.on_param("OGB/Socket/S/DepthOthers", 1.0);
        near(depth(&d, "S"), 0.0);
    }

    #[test]
    fn older_plug_on_sps3_socket_guesses_its_length() {
        // Same steps as OGB's "legacy plug on an SPS3 socket" test.
        let mut d = Detector::default();
        d.on_param("OGB/Socket/S/DepthRadius", 0.2);
        d.on_param("OGB/Socket/S/DepthOthers_SenderLossyScaleZ", 99.0);
        d.on_param("OGB/Socket/S/DepthOthers", 0.5);
        d.on_param("OGB/Socket/S/DepthOthers_PointSeparation", 0.5);
        near(depth(&d, "S"), 0.0);
        d.on_param("OGB/Socket/S/DepthOthers", 1.0);
        d.on_param("OGB/Socket/S/DepthOthers_PointSeparation", 0.2);
        near(depth(&d, "S"), 0.5);
        d.on_param("OGB/Socket/S/DepthOthers_PointSeparation", 0.1);
        near(depth(&d, "S"), 0.75);
        d.on_param("OGB/Socket/S/DepthOthers", 0.0);
        d.on_param("OGB/Socket/S/DepthOthers_PointSeparation", -1.0);
        d.on_param("OGB/Socket/S/DepthOthers_PointSeparation", 0.3);
        d.on_param("OGB/Socket/S/DepthOthers", 1.0);
        near(depth(&d, "S"), 0.0);
        d.on_param("OGB/Socket/S/DepthOthers_PointSeparation", 0.15);
        near(depth(&d, "S"), 0.5);
    }

    #[test]
    fn sps3_woohoo_counts() {
        let s = SpsSettings { seconds: 10, gap_seconds: 60, ..Default::default() };
        let mut d = Detector::default();
        d.tick(0, &s, None);
        d.on_param("OGB/Socket/Vagina/DepthOthers_CollisionTag_SPS/Plug/_public", 1.0);
        d.on_param("OGB/Socket/Vagina/DepthOthers_SenderLossyScaleZ", 0.2);
        d.on_param("OGB/Socket/Vagina/DepthOthers_PointSeparation", 0.05);
        d.on_param("OGB/Socket/Vagina/DepthOthers", 1.0);
        assert!((1..=15).any(|t| d.tick(t * 1000, &s, None)));
        assert!(d.live(&s, None).iter().any(|l| l.name == "Vagina" && l.channel == "DepthOthers" && l.counts));
        // Self and touch values are swallowed, not passed on.
        assert!(d.on_param("OGB/Socket/Vagina/DepthSelf", 1.0));
        assert!(d.on_param("OGB/Socket/Vagina/TouchOthers", 1.0));
        assert_eq!(d.live(&s, None).len(), 1);
    }

    #[test]
    fn rereading_the_radius_doesnt_keep_a_stuck_value_fresh() {
        let s = SpsSettings { seconds: 10, gap_seconds: 60, ..Default::default() };
        let mut d = Detector::default();
        d.tick(0, &s, None);
        d.on_param("OGB/Socket/Pussy/DepthRadius", 0.2);
        d.on_param("OGB/Socket/Pussy/DepthOthers_CollisionTag_SPS/Plug/_public", 1.0);
        d.on_param("OGB/Socket/Pussy/DepthOthers_SenderLossyScaleZ", 0.2);
        d.on_param("OGB/Socket/Pussy/DepthOthers_PointSeparation", 0.05);
        d.on_param("OGB/Socket/Pussy/DepthOthers", 1.0);
        assert!((1..=30).any(|t| d.tick(t * 1000, &s, None)));
        // OSCQuery polls keep reporting the same radius while the plug's 0 got lost.
        for t in 31..=200 {
            d.on_param("OGB/Socket/Pussy/DepthRadius", 0.2);
            assert!(!d.tick(t * 1000, &s, None));
        }
        assert_eq!(d.progress(&s), 0.0, "session over");
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
