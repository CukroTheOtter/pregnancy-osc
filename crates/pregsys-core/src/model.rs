//! Saved app data. Mirrored in TypeScript at `src/lib/types.ts`.
//!
//! Field names are camelCase on disk so the web UI can use them directly.
//! Every struct uses `#[serde(default)]` so older files load cleanly.

use crate::instance::{InstanceTracker, Person};
use crate::movement::{AllowedKicks, KickFrequency, KickStrength};
use crate::sps::SpsSettings;
use crate::timeline::{DurationMode, Timeline};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DATA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SyncScope {
    /// One pregnancy follows the user onto every avatar.
    #[default]
    Global,
    /// Each avatar has its own pregnancy.
    PerAvatar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SpoilerMode {
    #[default]
    Surprise,
    Hints,
    All,
}

/// Whether babies get a gender (revealed at week 20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum GenderMode {
    /// No genders shown.
    Off,
    /// Each baby is randomly a girl or a boy.
    #[default]
    Random,
}

/// How likely twins and more are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Multiples {
    /// Twins about 3%, triplets about 0.1%.
    Realistic,
    /// Twins about 10%, triplets about 2%.
    #[default]
    Playful,
    /// Twins 25%, triplets 8%, quads 2%.
    Lots,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
    /// Dark with true black backgrounds (OLED screens).
    Black,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Theme {
    /// Color set: strawberry, lavender, mint, peach or sky (see src/app.css).
    pub palette: String,
    pub mode: ThemeMode,
    /// Your own accent color, used when `custom_accent` is on.
    pub accent: String,
    pub custom_accent: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self { palette: "strawberry".into(), mode: ThemeMode::System, accent: "#f59ac0".into(), custom_accent: false }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OscSettings {
    pub enabled: bool,
    pub use_oscquery: bool,
    pub send_host: String,
    pub send_port: u16,
    pub recv_port: u16,
}

impl Default for OscSettings {
    fn default() -> Self {
        Self { enabled: true, use_oscquery: true, send_host: "127.0.0.1".into(), send_port: 9000, recv_port: 9001 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub sync_scope: SyncScope,
    /// Sync: how long an in-game change waits before it's remembered (a
    /// switch in that time drops it; it's the next avatar's own value).
    pub sync_wait_ms: u32,
    /// The user's own species (used when an avatar profile has none).
    pub user_species: String,
    pub hybrid_chance: f64,
    pub spoiler_mode: SpoilerMode,
    pub theme: Theme,
    pub start_with_steamvr: bool,
    pub start_with_os: bool,
    pub start_in_tray: bool,
    pub exit_with_steamvr: bool,
    /// Avatars you wear are added to the Avatars list on their own.
    pub auto_add_avatars: bool,
    /// Closing the window keeps the app running in the tray (else it quits).
    pub close_to_tray: bool,
    pub heartbeat_sound: bool,
    pub sound_volume: f64,
    pub reveal_pulse: bool,
    pub kick_events: bool,
    pub notifications: bool,
    pub default_mode: DurationMode,
    pub default_speed: f64,
    /// Length for new pregnancies in custom mode (the last one picked).
    pub default_custom_ms: Option<i64>,
    /// Minutes with no joins/leaves before a lone other player counts as the partner.
    pub settle_minutes: u32,
    /// Belly defaults; avatar profiles can override each one.
    pub hide_belly: bool,
    pub belly_scale: f64,
    pub fetal_sounds: Option<bool>,
    pub osc: OscSettings,
    /// Messages in VRChat's chatbox when things happen.
    pub chatbox: crate::chatbox::ChatboxSettings,
    /// Manually chosen VRChat log folder (None = auto-detect).
    pub log_dir: Option<String>,
    pub read_logs: bool,
    /// Fetch partners' pictures from their public VRChat profile page.
    pub show_pictures: bool,
    /// Fertility cycle + grace period: in-game fun doesn't start a pregnancy
    /// right away; the outcome is rolled and revealed later.
    pub fertility: bool,
    /// Chance of pregnancy on the most fertile day.
    pub fertility_peak: f64,
    /// When the current cycle started (day 1).
    pub cycle_anchor_ms: Option<i64>,
    pub multiples: Multiples,
    /// Extra size (bellyExtra): twins and more make the belly bigger.
    pub bigger_for_multiples: bool,
    /// Extra size: the belly keeps growing past the due date.
    pub grow_when_overdue: bool,
    pub gender_mode: GenderMode,
    /// Belly movement: the app sends kicks to the avatar.
    pub kicks: bool,
    pub kick_frequency: KickFrequency,
    pub allowed_kicks: AllowedKicks,
    /// Kicks come more often (and stronger) while the partner is here.
    pub kicks_with_partner: bool,
    /// When labor starts, how long it takes, and whether it waits for the partner.
    pub labor: crate::labor::LaborSettings,
    /// Detecting WooHoos from SPS sockets.
    pub sps: SpsSettings,
    /// Hidden debug tools (tap the title 5 times).
    pub debug: bool,
    pub onboarded: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sync_scope: SyncScope::Global,
            sync_wait_ms: crate::driver::SYNC_HOLD_MS as u32,
            user_species: "human".into(),
            hybrid_chance: 0.05,
            spoiler_mode: SpoilerMode::Surprise,
            theme: Theme::default(),
            start_with_steamvr: false,
            start_with_os: false,
            start_in_tray: false,
            exit_with_steamvr: false,
            auto_add_avatars: true,
            close_to_tray: true,
            heartbeat_sound: false,
            sound_volume: 0.5,
            reveal_pulse: true,
            kick_events: true,
            notifications: true,
            default_mode: DurationMode::Custom,
            default_speed: 1.0,
            default_custom_ms: Some(30 * crate::timeline::DAY_MS),
            settle_minutes: 10,
            hide_belly: false,
            belly_scale: 1.0,
            fetal_sounds: None,
            osc: OscSettings::default(),
            chatbox: Default::default(),
            log_dir: None,
            read_logs: true,
            show_pictures: false,
            fertility: true,
            fertility_peak: 0.6,
            cycle_anchor_ms: None,
            multiples: Multiples::Playful,
            bigger_for_multiples: true,
            grow_when_overdue: true,
            gender_mode: GenderMode::Random,
            kicks: true,
            kick_frequency: KickFrequency::Normal,
            allowed_kicks: AllowedKicks::default(),
            kicks_with_partner: false,
            labor: Default::default(),
            sps: SpsSettings::default(),
            debug: false,
            onboarded: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum BodyKind {
    #[default]
    Human,
    Furry,
    Egg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum PartnerStatus {
    /// Exactly one other person was there; not confirmed yet.
    Guessed,
    Confirmed,
    #[default]
    Unknown,
    /// Typed in by hand.
    Manual,
    /// Picked at random from the people in the instance.
    Random,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SpeciesSource {
    /// Guessed from the avatar name.
    Guess,
    Confirmed,
    /// Unknown partner: use the user's species.
    #[default]
    Fallback,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Partner {
    pub status: PartnerStatus,
    pub name: Option<String>,
    pub user_id: Option<String>,
    pub avatar_name: Option<String>,
    pub species: Option<String>,
    pub species_source: SpeciesSource,
    /// Everyone shared with around conception (for the "who was it?" card).
    pub candidates: Vec<Person>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum BabyMode {
    /// Traits come from the seeded roll.
    #[default]
    Rolled,
    /// Traits were planned by hand; never changed automatically.
    Manual,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SpeciesBlend {
    /// species id -> weight (weights sum to 1). One entry = pure species.
    pub parts: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Baby {
    pub index: u32,
    pub name: Option<String>,
    pub mode: BabyMode,
    /// Manual species override (marked manual, never rerolled).
    pub manual_species: Option<SpeciesBlend>,
    /// Trait key -> value the user locked or set by hand.
    pub locks: BTreeMap<String, String>,
    /// Bumped by "reroll the rest" so unlocked traits get a new seed.
    pub reroll: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct OscHints {
    /// Species index for the optional `species` parameter.
    pub species_index: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum PregnancySource {
    #[default]
    Manual,
    /// Started automatically from in-game conception.
    Conception,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Pregnancy {
    pub id: String,
    pub name: String,
    pub created_ms: i64,
    pub timeline: Timeline,
    pub baby_count: u32,
    pub kind: BodyKind,
    /// The carrying parent's species (defaults from the avatar profile).
    pub mother_species: String,
    pub partner: Partner,
    pub babies: Vec<Baby>,
    /// Trait reveal key -> when it was revealed.
    pub reveals: BTreeMap<String, i64>,
    pub source: PregnancySource,
    /// Avatar this pregnancy belongs to (per-avatar mode).
    pub avatar_id: Option<String>,
    pub world_name: Option<String>,
    pub ended_ms: Option<i64>,
    /// Ended with a birth (counts toward "kids together").
    pub born: bool,
    /// The in-game moment it came from (links it to the fun log).
    pub conceived_ms: Option<i64>,
    /// "girl" or "boy" per baby, rolled at conception (shown from week 20).
    pub genders: Vec<String>,
    /// Recent kicks (newest last, capped).
    pub kicks: Vec<KickEvent>,
    /// Milestone keys already announced.
    pub milestones: Vec<String>,
    pub osc_hints: OscHints,
    /// Labor, once it has started (kept after the birth).
    pub labor: Option<crate::labor::Labor>,
}

impl Default for Pregnancy {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            created_ms: 0,
            timeline: Timeline::new(0, DurationMode::Realtime, 1.0, None),
            baby_count: 1,
            kind: BodyKind::Human,
            mother_species: "human".into(),
            partner: Partner::default(),
            babies: vec![Baby::default()],
            reveals: BTreeMap::new(),
            source: PregnancySource::Manual,
            avatar_id: None,
            world_name: None,
            ended_ms: None,
            born: false,
            conceived_ms: None,
            genders: Vec::new(),
            kicks: Vec::new(),
            milestones: Vec::new(),
            osc_hints: OscHints::default(),
            labor: None,
        }
    }
}

impl Pregnancy {
    pub fn is_active(&self) -> bool {
        self.ended_ms.is_none()
    }

    /// Makes `babies` match `baby_count`, keeping existing entries.
    pub fn sync_babies(&mut self) {
        let n = self.baby_count.clamp(1, 8);
        self.baby_count = n;
        self.babies.truncate(n as usize);
        while self.babies.len() < n as usize {
            let index = self.babies.len() as u32;
            self.babies.push(Baby { index, ..Default::default() });
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AvatarProfile {
    pub id: String,
    pub name: Option<String>,
    /// The app picked the name (VRChat's files or log), so it may correct it.
    /// Names typed by the user are never changed.
    pub auto_name: bool,
    pub species: Option<String>,
    pub kind: Option<BodyKind>,
    pub hide_belly: Option<bool>,
    pub belly_scale: Option<f64>,
    pub fetal_sounds: Option<bool>,
    /// Per-avatar mode: this avatar's pregnancy.
    pub pregnancy_id: Option<String>,
    /// Use the global pregnancy even in per-avatar mode.
    pub follow_global: bool,
    /// The group this avatar is in (avatars in a group share one pregnancy).
    pub group_id: Option<String>,
    /// Old way of sharing (before groups). Read from older files, turned
    /// into groups by `upgrade_links`, never written.
    #[serde(skip_serializing)]
    pub linked_to: Option<String>,
    /// Custom parameter names for this avatar: mapping key -> name.
    pub param_names: BTreeMap<String, String>,
    /// SPS socket choices for this avatar.
    pub sps: crate::sps::AvatarSps,
    pub last_seen_ms: i64,
    /// Parameter names this avatar has (from OSCQuery).
    pub params: Vec<String>,
}

impl AvatarProfile {
    /// VRChat showed Pregnancy System parameters on it (same rule as
    /// `hasSystem` in src/lib/avatars.ts). Not checked yet counts as no.
    pub fn has_system(&self) -> bool {
        self.params.iter().any(|n| n.starts_with("Pregnancy/") || self.param_names.values().any(|o| o == n))
    }
}

/// Avatars that share one pregnancy (per-avatar mode).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AvatarGroup {
    pub id: String,
    pub name: String,
    pub pregnancy_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct JournalEntry {
    pub id: String,
    pub time_ms: i64,
    pub pregnancy_id: Option<String>,
    /// "note", "milestone", "reveal", "event"
    pub kind: String,
    pub title: String,
    pub text: String,
    /// Optional image as a data URL (screenshots, keepsakes).
    pub image: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Keepsake {
    pub id: String,
    pub pregnancy_id: String,
    pub baby_index: u32,
    pub trait_key: String,
    pub value: String,
    pub week: f64,
    pub time_ms: i64,
}

/// In-game fun during the grace period. The outcome is already rolled but
/// not shown until `reveal_ms`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PendingConception {
    pub time_ms: i64,
    pub reveal_ms: i64,
    pub chance: f64,
    pub cycle_day: f64,
    /// Hidden from the UI until the reveal.
    pub positive: bool,
    /// Babies, rolled at the moment of fun (hidden until about week 12).
    pub baby_count: u32,
    pub partner: Partner,
    pub avatar_id: Option<String>,
    pub world_name: Option<String>,
}

/// One kick sent to the avatar.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct KickEvent {
    pub time_ms: i64,
    pub strength: KickStrength,
}

/// Kicks kept per pregnancy (enough for "today" and "last kick").
pub const KICKS_KEPT: usize = 300;

/// One "something happened" moment, kept for partner stats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FunEvent {
    pub time_ms: i64,
    pub partner_name: Option<String>,
    pub partner_user_id: Option<String>,
    pub avatar_id: Option<String>,
    pub world_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RevealResult {
    pub time_ms: i64,
    /// When the fun happened.
    pub event_ms: i64,
    pub positive: bool,
    pub seen: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AppData {
    pub version: u32,
    pub settings: Settings,
    pub pregnancies: Vec<Pregnancy>,
    /// Global mode: the pregnancy shown on every avatar.
    pub active_pregnancy_id: Option<String>,
    pub avatars: BTreeMap<String, AvatarProfile>,
    /// Avatar groups by id (see `AvatarGroup`).
    pub groups: BTreeMap<String, AvatarGroup>,
    /// Remembered species answers: lowercased avatar name (or user id) -> species id.
    pub species_answers: BTreeMap<String, String>,
    /// Optional community avatar-name -> species table, imported by the user.
    pub community_species: BTreeMap<String, String>,
    pub journal: Vec<JournalEntry>,
    pub keepsakes: Vec<Keepsake>,
    pub instance: InstanceTracker,
    pub current_avatar_id: Option<String>,
    /// Grace periods in progress (one per owner in per-avatar mode, see `owner`).
    pub waiting: Vec<PendingConception>,
    pub last_reveal: Option<RevealResult>,
    pub fun_log: Vec<FunEvent>,
    /// Last value seen in-game for each sync parameter (re-sent on avatar change).
    pub sync_values: BTreeMap<String, f64>,
    /// Unlocked achievements: id -> when (see `achievements`).
    pub achievements: BTreeMap<String, i64>,
    /// Debug: when Test Buddy was added (see `testbuddy`), until removed.
    pub test_buddy_since: Option<i64>,
}

impl AppData {
    pub fn new() -> Self {
        Self { version: DATA_VERSION, ..Default::default() }
    }

    pub fn pregnancy(&self, id: &str) -> Option<&Pregnancy> {
        self.pregnancies.iter().find(|p| p.id == id)
    }

    pub fn pregnancy_mut(&mut self, id: &str) -> Option<&mut Pregnancy> {
        self.pregnancies.iter_mut().find(|p| p.id == id)
    }

    /// The pregnancy that should show on this avatar right now.
    pub fn current_pregnancy(&self, avatar_id: Option<&str>) -> Option<&Pregnancy> {
        let id = self.current_pregnancy_id(avatar_id)?;
        self.pregnancy(&id).filter(|p| p.is_active())
    }

    /// Who holds the pregnancy this avatar shows: its group's id, or the
    /// avatar itself. A group id passed in comes back as is. `None` in
    /// global mode (everyone shares one).
    pub fn owner(&self, avatar_id: Option<&str>) -> Option<String> {
        if self.settings.sync_scope == SyncScope::Global {
            return None;
        }
        let id = avatar_id?;
        if self.groups.contains_key(id) {
            return Some(id.to_string());
        }
        match self.avatars.get(id).and_then(|p| p.group_id.as_deref()) {
            Some(g) if self.groups.contains_key(g) => Some(g.to_string()),
            _ => Some(id.to_string()),
        }
    }

    /// Avatars in a group, most recently worn first.
    pub fn group_members(&self, group_id: &str) -> Vec<String> {
        let mut m: Vec<&AvatarProfile> = self.avatars.values().filter(|a| a.group_id.as_deref() == Some(group_id)).collect();
        m.sort_by_key(|a| std::cmp::Reverse(a.last_seen_ms));
        m.into_iter().map(|a| a.id.clone()).collect()
    }

    /// Turns old avatar links (`linked_to`) into groups. Each chain of links
    /// becomes one group named after the avatar the others linked to, and
    /// takes over its pregnancy and grace period.
    pub fn upgrade_links(&mut self) {
        let ids: Vec<String> = self.avatars.keys().cloned().collect();
        for id in &ids {
            if self.avatars[id].linked_to.is_none() {
                continue;
            }
            // Follow the chain to its end (guards against loops).
            let mut root = id.clone();
            for _ in 0..8 {
                match self.avatars.get(&root).and_then(|p| p.linked_to.clone()) {
                    Some(next) if next != root && self.avatars.contains_key(&next) => root = next,
                    _ => break,
                }
            }
            if &root == id {
                continue;
            }
            let gid = match self.avatars[&root].group_id.clone() {
                Some(g) if self.groups.contains_key(&g) => g,
                _ => {
                    let g = format!("grp_{root}");
                    let r = self.avatars.get_mut(&root).unwrap();
                    let name = r.name.clone().unwrap_or_else(|| "Linked avatars".into());
                    let pregnancy_id = r.pregnancy_id.take();
                    r.group_id = Some(g.clone());
                    for w in &mut self.waiting {
                        if w.avatar_id.as_deref() == Some(root.as_str()) {
                            w.avatar_id = Some(g.clone());
                        }
                    }
                    self.groups.insert(g.clone(), AvatarGroup { id: g.clone(), name, pregnancy_id });
                    g
                }
            };
            self.avatars.get_mut(id).unwrap().group_id = Some(gid);
        }
        for a in self.avatars.values_mut() {
            a.linked_to = None;
        }
    }

    /// Avatars that show the same pregnancy as this one (including itself).
    pub fn linked_group(&self, avatar_id: &str) -> Vec<String> {
        let owner = self.owner(Some(avatar_id));
        self.avatars.keys().filter(|k| self.owner(Some(k)) == owner).cloned().collect()
    }

    /// The grace period that applies to this avatar, if any.
    pub fn pending_for(&self, avatar_id: Option<&str>) -> Option<&PendingConception> {
        let global = self.settings.sync_scope == SyncScope::Global;
        let owner = self.owner(avatar_id);
        self.waiting.iter().find(|w| global || w.avatar_id == owner)
    }

    pub fn current_pregnancy_id(&self, avatar_id: Option<&str>) -> Option<String> {
        let owner = self.owner(avatar_id);
        if let Some(g) = owner.as_deref().and_then(|o| self.groups.get(o)) {
            return g.pregnancy_id.clone();
        }
        let profile = owner.as_deref().and_then(|a| self.avatars.get(a));
        match self.settings.sync_scope {
            SyncScope::Global => self.active_pregnancy_id.clone(),
            SyncScope::PerAvatar => match profile {
                Some(p) if p.follow_global => self.active_pregnancy_id.clone(),
                Some(p) => p.pregnancy_id.clone(),
                // Unknown avatar (no OSC yet): show the global one in the UI.
                None if avatar_id.is_none() => self.active_pregnancy_id.clone(),
                None => None,
            },
        }
    }

    /// Effective belly settings for an avatar (profile overrides, else defaults).
    pub fn belly_settings(&self, avatar_id: Option<&str>) -> (bool, f64, Option<bool>) {
        let s = &self.settings;
        let p = avatar_id.and_then(|a| self.avatars.get(a));
        (
            p.and_then(|p| p.hide_belly).unwrap_or(s.hide_belly),
            p.and_then(|p| p.belly_scale).unwrap_or(s.belly_scale),
            p.and_then(|p| p.fetal_sounds).or(s.fetal_sounds),
        )
    }

    /// The user's species on this avatar (profile first, then the global default).
    pub fn user_species(&self, avatar_id: Option<&str>) -> String {
        avatar_id
            .and_then(|a| self.avatars.get(a))
            .and_then(|p| p.species.clone())
            .unwrap_or_else(|| self.settings.user_species.clone())
    }

    /// Links a pregnancy as current for the scope in use.
    pub fn set_current(&mut self, avatar_id: Option<&str>, preg_id: Option<String>) {
        match (self.settings.sync_scope, avatar_id) {
            (SyncScope::PerAvatar, Some(a)) => {
                let a = self.owner(Some(a)).unwrap_or_else(|| a.to_string());
                if let Some(g) = self.groups.get_mut(&a) {
                    g.pregnancy_id = preg_id;
                    return;
                }
                let p = self.avatars.entry(a.clone()).or_insert_with(|| AvatarProfile { id: a.to_string(), ..Default::default() });
                if p.follow_global {
                    self.active_pregnancy_id = preg_id;
                } else {
                    p.pregnancy_id = preg_id;
                }
            }
            _ => self.active_pregnancy_id = preg_id,
        }
    }
}

/// Short random-looking id (time + counter), good enough for local records.
pub fn new_id(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}_{:x}{:04x}", crate::now_ms(), n & 0xffff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_pregnancy_global_vs_per_avatar() {
        let mut d = AppData::new();
        d.pregnancies.push(Pregnancy { id: "g".into(), ..Default::default() });
        d.pregnancies.push(Pregnancy { id: "a".into(), ..Default::default() });
        d.active_pregnancy_id = Some("g".into());
        d.avatars.insert("avtr_1".into(), AvatarProfile { id: "avtr_1".into(), pregnancy_id: Some("a".into()), ..Default::default() });

        assert_eq!(d.current_pregnancy(Some("avtr_1")).unwrap().id, "g");
        d.settings.sync_scope = SyncScope::PerAvatar;
        assert_eq!(d.current_pregnancy(Some("avtr_1")).unwrap().id, "a");
        assert!(d.current_pregnancy(Some("avtr_unknown")).is_none());
        d.avatars.get_mut("avtr_1").unwrap().follow_global = true;
        assert_eq!(d.current_pregnancy(Some("avtr_1")).unwrap().id, "g");
    }

    #[test]
    fn linked_avatars_share_a_pregnancy() {
        let mut d = AppData::new();
        d.settings.sync_scope = SyncScope::PerAvatar;
        for id in ["a", "b", "c"] {
            d.avatars.insert(id.into(), AvatarProfile { id: id.into(), ..Default::default() });
        }
        d.groups.insert("g".into(), AvatarGroup { id: "g".into(), name: "Fluffs".into(), pregnancy_id: None });
        d.avatars.get_mut("a").unwrap().group_id = Some("g".into());
        d.avatars.get_mut("b").unwrap().group_id = Some("g".into());
        d.pregnancies.push(Pregnancy { id: "p".into(), ..Default::default() });
        // Starting it on one avatar puts it on the whole group.
        d.set_current(Some("b"), Some("p".into()));
        assert_eq!(d.current_pregnancy(Some("a")).unwrap().id, "p");
        assert_eq!(d.current_pregnancy(Some("b")).unwrap().id, "p");
        assert!(d.current_pregnancy(Some("c")).is_none());
        let mut g = d.linked_group("a");
        g.sort();
        assert_eq!(g, vec!["a", "b"]);
    }

    #[test]
    fn old_links_become_groups() {
        let mut d = AppData::new();
        d.settings.sync_scope = SyncScope::PerAvatar;
        d.pregnancies.push(Pregnancy { id: "p".into(), ..Default::default() });
        d.avatars.insert("a".into(), AvatarProfile { id: "a".into(), name: Some("Bun".into()), pregnancy_id: Some("p".into()), ..Default::default() });
        d.avatars.insert("b".into(), AvatarProfile { id: "b".into(), linked_to: Some("a".into()), ..Default::default() });
        d.avatars.insert("c".into(), AvatarProfile { id: "c".into(), linked_to: Some("b".into()), ..Default::default() });
        // A loop doesn't hang.
        d.avatars.insert("x".into(), AvatarProfile { id: "x".into(), linked_to: Some("y".into()), ..Default::default() });
        d.avatars.insert("y".into(), AvatarProfile { id: "y".into(), linked_to: Some("x".into()), ..Default::default() });
        d.waiting.push(PendingConception { avatar_id: Some("a".into()), ..Default::default() });
        d.upgrade_links();
        let g = d.owner(Some("c")).unwrap();
        assert_eq!(d.owner(Some("a")).as_deref(), Some(g.as_str()));
        assert_eq!(d.groups[&g].name, "Bun");
        assert_eq!(d.current_pregnancy(Some("b")).unwrap().id, "p");
        assert_eq!(d.waiting[0].avatar_id.as_deref(), Some(g.as_str()));
        assert!(d.avatars.values().all(|a| a.linked_to.is_none()));
        // Never written back out.
        assert!(!serde_json::to_string(&d).unwrap().contains("linkedTo"));
    }

    #[test]
    fn ended_pregnancy_is_not_current() {
        let mut d = AppData::new();
        d.pregnancies.push(Pregnancy { id: "g".into(), ended_ms: Some(5), ..Default::default() });
        d.active_pregnancy_id = Some("g".into());
        assert!(d.current_pregnancy(None).is_none());
    }

    #[test]
    fn profile_overrides() {
        let mut d = AppData::new();
        d.settings.belly_scale = 1.0;
        d.avatars.insert("x".into(), AvatarProfile { id: "x".into(), belly_scale: Some(0.8), species: Some("fox".into()), ..Default::default() });
        assert_eq!(d.belly_settings(Some("x")).1, 0.8);
        assert_eq!(d.belly_settings(Some("y")).1, 1.0);
        assert_eq!(d.user_species(Some("x")), "fox");
        assert_eq!(d.user_species(None), "human");
    }

    #[test]
    fn sync_babies() {
        let mut p = Pregnancy { baby_count: 3, ..Default::default() };
        p.babies[0].name = Some("Keep".into());
        p.sync_babies();
        assert_eq!(p.babies.len(), 3);
        assert_eq!(p.babies[2].index, 2);
        p.baby_count = 1;
        p.sync_babies();
        assert_eq!(p.babies[0].name.as_deref(), Some("Keep"));
    }

    #[test]
    fn empty_json_loads_with_defaults() {
        let d: AppData = serde_json::from_str("{}").unwrap();
        assert_eq!(d.settings.hybrid_chance, 0.05);
    }
}
