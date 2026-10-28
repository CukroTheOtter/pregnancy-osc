//! The editable OSC parameter mapping (`config/osc-parameters.json`).
//!
//! Parameter names are never hardcoded in logic. Everything goes through a
//! logical key (`stage`, `hideBelly`, ...) that this file maps to the real
//! avatar parameter name.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// The default mapping shipped with the app.
pub const DEFAULT_MAPPING_JSON: &str = include_str!("../../../config/osc-parameters.json");

pub const OSC_PARAM_PREFIX: &str = "/avatar/parameters/";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParamType {
    Float,
    Int,
    Bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamDef {
    /// Real avatar parameter name. Empty = turned off.
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ParamType,
    #[serde(default)]
    pub synced: bool,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub optional: bool,
    /// Other names the same parameter may have on some avatars.
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Heartbeat {
    pub interval_ms: u64,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverdueCfg {
    /// How far past the due date (as a fraction of the full term) the
    /// overdue belly keeps growing before it stops at its maximum.
    pub max_overdue_fraction: f64,
}

/// A parameter the app remembers and re-sends on every avatar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncParam {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ParamType,
    /// Built-in entries (the SDK's own toggles): always there, can be
    /// renamed but not removed, and the app finds them by this, not by name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<SyncRole>,
}

/// What a built-in Sync entry is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SyncRole {
    HideBelly,
    FetalSounds,
}

impl SyncRole {
    pub const ALL: [SyncRole; 2] = [SyncRole::HideBelly, SyncRole::FetalSounds];

    /// The end of its default name, used to spot a renamed one in older files.
    fn suffix(self) -> &'static str {
        match self {
            SyncRole::HideBelly => "HideBelly",
            SyncRole::FetalSounds => "FetalSounds",
        }
    }
}

/// Values sent to the FetalMovement parameter for each kick strength, and
/// how long before it goes back to 0. The SDK's kick layer plays 1, 2 or 3.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KickValues {
    pub weak: i32,
    pub strong: i32,
    pub extreme: i32,
    pub hold_ms: i64,
    /// Sent for each contraction during labor (held for 30 s).
    pub contraction: i32,
}

impl Default for KickValues {
    fn default() -> Self {
        Self { weak: 1, strong: 2, extreme: 3, hold_ms: 1500, contraction: 4 }
    }
}

impl KickValues {
    pub fn value(&self, s: crate::movement::KickStrength) -> i32 {
        match s {
            crate::movement::KickStrength::Weak => self.weak,
            crate::movement::KickStrength::Strong => self.strong,
            crate::movement::KickStrength::Extreme => self.extreme,
        }
    }
}

/// Current version of the parameter file layout.
pub const MAPPING_VERSION: u32 = 7;

/// The app's own menu buttons: made for this app by the SDK, so their names
/// are fixed (no renaming, no per-avatar names).
pub const FIXED_KEYS: [&str; 3] = ["controlStart", "controlPause", "controlStop"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mapping {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub send_interval_ms: u64,
    pub min_stage_change: f64,
    pub heartbeat: Heartbeat,
    pub overdue: OverdueCfg,
    pub parameters: BTreeMap<String, ParamDef>,
    /// Parameters that follow you onto every avatar.
    #[serde(default)]
    pub sync: Vec<SyncParam>,
    /// FetalMovement values per kick strength.
    #[serde(default)]
    pub kicks: KickValues,
}

#[derive(Debug, thiserror::Error)]
pub enum MappingError {
    #[error("could not read mapping file: {0}")]
    Io(#[from] std::io::Error),
    #[error("mapping file is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl Default for Mapping {
    fn default() -> Self {
        serde_json::from_str(DEFAULT_MAPPING_JSON).expect("bundled mapping is valid")
    }
}

/// True if a name is an app parameter's name or backup name.
fn is_app_name(params: &BTreeMap<String, ParamDef>, name: &str) -> bool {
    params.values().any(|d| !d.name.is_empty() && (d.name == name || d.aliases.iter().any(|a| a == name)))
}

impl Mapping {
    pub fn from_json(json: &str) -> Result<Self, MappingError> {
        let mut m: Mapping = serde_json::from_str(json)?;
        m.upgrade();
        m.fill_missing();
        m.drop_protected_sync();
        m.apply_locks();
        Ok(m)
    }

    /// What can't be changed: the built-in Sync entries stay, and the menu
    /// buttons keep their names.
    pub fn apply_locks(&mut self) {
        self.ensure_builtin_sync();
        let defaults = Mapping::default();
        for key in FIXED_KEYS {
            if let Some(d) = defaults.parameters.get(key) {
                self.parameters.insert(key.to_string(), d.clone());
            }
        }
    }

    /// The built-in Sync entry for a role.
    pub fn sync_role(&self, role: SyncRole) -> Option<&SyncParam> {
        self.sync.iter().find(|s| s.role == Some(role))
    }

    /// The built-in Sync entries are always there, on/off, once each. One the
    /// user renamed keeps its name; one that went missing comes back.
    pub fn ensure_builtin_sync(&mut self) {
        let defaults: Vec<SyncParam> = serde_json::from_str::<Mapping>(DEFAULT_MAPPING_JSON).expect("bundled mapping is valid").sync;
        for (i, role) in SyncRole::ALL.into_iter().enumerate() {
            let def = defaults.iter().find(|s| s.role == Some(role)).expect("bundled mapping has every role");
            // Only the first entry with a role keeps it.
            let mut seen = false;
            for s in self.sync.iter_mut().filter(|s| s.role == Some(role)) {
                if seen {
                    s.role = None;
                }
                seen = true;
            }
            if !seen {
                // An older file: the default name, else one renamed but ending the same.
                let found = self.sync.iter().position(|s| s.role.is_none() && s.name == def.name).or_else(|| {
                    self.sync.iter().position(|s| s.role.is_none() && s.kind == ParamType::Bool && s.name.ends_with(role.suffix()))
                });
                match found {
                    Some(j) => self.sync[j].role = Some(role),
                    None => self.sync.insert(i.min(self.sync.len()), def.clone()),
                }
            }
            if let Some(s) = self.sync.iter_mut().find(|s| s.role == Some(role)) {
                s.kind = ParamType::Bool;
                s.name = s.name.trim().to_string();
                if s.name.is_empty() || is_app_name(&self.parameters, &s.name) {
                    s.name = def.name.clone();
                }
            }
        }
    }

    /// True if a name belongs to an app parameter (its name or a backup name).
    pub fn is_app_param(&self, name: &str) -> bool {
        self.key_for_name(name.trim()).is_some()
    }

    /// Sync entries that clash with app parameters (they'd fight the app).
    pub fn protected_sync(&self) -> Vec<String> {
        self.sync.iter().filter(|s| self.is_app_param(&s.name)).map(|s| s.name.clone()).collect()
    }

    /// Removes sync entries that clash with app parameters.
    pub fn drop_protected_sync(&mut self) {
        let bad = self.protected_sync();
        self.sync.retain(|s| !bad.contains(&s.name));
    }

    /// Brings older parameter files up to date. Names the user changed are kept.
    fn upgrade(&mut self) {
        if self.version >= MAPPING_VERSION {
            return;
        }
        let defaults = Mapping::default();
        // Older -> v4: one "Pregnancy/" prefix; the app's flags are AppActive and AppHeartbeat.
        let renames = [
            ("appActive", ["Pregnancy/Active", "PregnancyOSC/Active"]),
            ("appHeartbeat", ["Pregnancy/Heartbeat", "PregnancyOSC/Heartbeat"]),
            ("fertileLevel", ["PregnancyOSC/FertileLevel", "PregnancyOSC/FertileLevel"]),
            ("waiting", ["PregnancyOSC/Waiting", "PregnancyOSC/Waiting"]),
            ("babyCount", ["PregnancyOSC/BabyCount", "Pregnancy/BabyCount"]),
        ];
        for (key, old) in renames {
            if let (Some(p), Some(d)) = (self.parameters.get_mut(key), defaults.parameters.get(key)) {
                if old.contains(&p.name.as_str()) {
                    p.name = d.name.clone();
                }
                p.aliases = d.aliases.clone();
            }
        }
        // v1 -> v2: settings that follow you (hide belly, womb sounds) move to the sync list.
        if self.sync.is_empty() {
            for key in ["hideBelly", "fetalSounds"] {
                if let Some(p) = self.parameters.get(key).filter(|p| !p.name.trim().is_empty()) {
                    let role = if key == "hideBelly" { SyncRole::HideBelly } else { SyncRole::FetalSounds };
                    self.sync.push(SyncParam { name: p.name.clone(), kind: p.kind, role: Some(role) });
                }
            }
        }
        // v5 -> v6: the Overdue toggle became Extra size (bellyExtra), which the app writes.
        for key in ["hideBelly", "fetalSounds", "progressionSpeed", "eggMode", "species", "revealPulse", "kick", "overdue"] {
            self.parameters.remove(key);
        }
        // v6 -> v7: labor (`labor`, added by fill_missing) and the contraction code (defaults to 4).
        // v4 -> v5: the SDK's kick layer plays kick codes 1 / 2 / 3. Values the user changed are kept.
        if (self.kicks.weak, self.kicks.strong, self.kicks.extreme) == (25, 60, 90) {
            self.kicks = KickValues { hold_ms: self.kicks.hold_ms, contraction: self.kicks.contraction, ..KickValues::default() };
        }
        self.version = MAPPING_VERSION;
    }

    pub fn sync_def(&self, name: &str) -> Option<&SyncParam> {
        if self.is_app_param(name) {
            return None;
        }
        self.sync.iter().find(|s| !s.name.is_empty() && s.name == name)
    }

    pub fn load(path: &Path) -> Result<Self, MappingError> {
        Self::from_json(&std::fs::read_to_string(path)?)
    }

    /// Loads the user's mapping, writing the default first if it doesn't exist.
    pub fn load_or_create(path: &Path) -> Result<Self, MappingError> {
        if !path.exists() {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(path, DEFAULT_MAPPING_JSON)?;
        }
        Self::load(path)
    }

    /// Adds any keys a newer app version knows about but the user's file lacks.
    fn fill_missing(&mut self) {
        let defaults = Mapping::default();
        for (k, v) in defaults.parameters {
            self.parameters.entry(k).or_insert(v);
        }
    }

    pub fn get(&self, key: &str) -> Option<&ParamDef> {
        self.parameters.get(key).filter(|p| !p.name.trim().is_empty())
    }

    /// Full OSC address for a logical key, e.g. `/avatar/parameters/Pregnancy/Stage`.
    pub fn address(&self, key: &str) -> Option<String> {
        self.get(key).map(|p| format!("{OSC_PARAM_PREFIX}{}", p.name))
    }

    /// Picks the name the avatar actually has, given the avatar's parameter
    /// list (from OSCQuery). Returns `None` if the avatar has none of them.
    pub fn resolve<'a>(&'a self, key: &str, avatar_params: &[String]) -> Option<&'a str> {
        let def = self.get(key)?;
        std::iter::once(&def.name)
            .chain(def.aliases.iter())
            .find(|n| avatar_params.iter().any(|a| a == *n))
            .map(|s| s.as_str())
    }

    /// Reverse lookup: which logical key does this parameter name belong to?
    pub fn key_for_name(&self, name: &str) -> Option<&str> {
        self.parameters
            .iter()
            .find(|(_, d)| !d.name.is_empty() && (d.name == name || d.aliases.iter().any(|a| a == name)))
            .map(|(k, _)| k.as_str())
    }

    /// Synced bits used by the enabled parameters (bool = 1, int/float = 8).
    pub fn synced_bits(&self, include_optional: bool) -> u32 {
        self.parameters
            .values()
            .filter(|p| !p.name.is_empty() && p.synced && (include_optional || !p.optional))
            .map(|p| match p.kind {
                ParamType::Bool => 1,
                _ => 8,
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mapping_parses() {
        let m = Mapping::default();
        assert_eq!(m.address("stage").unwrap(), "/avatar/parameters/Pregnancy/Stage");
        assert!(!m.get("appActive").unwrap().synced);
    }

    #[test]
    fn empty_name_turns_parameter_off() {
        let mut m = Mapping::default();
        m.parameters.get_mut("waiting").unwrap().name.clear();
        assert!(m.get("waiting").is_none());
        assert!(m.address("waiting").is_none());
    }

    #[test]
    fn resolve_uses_aliases() {
        let m = Mapping::default();
        let avatar = vec!["PregSys_Pregnancy_Stage".to_string()];
        assert_eq!(m.resolve("stage", &avatar), Some("PregSys_Pregnancy_Stage"));
        assert_eq!(m.resolve("bellyExtra", &avatar), None);
        assert_eq!(m.key_for_name("PregSys_Pregnancy_Stage"), Some("stage"));
    }

    #[test]
    fn missing_keys_are_filled_from_defaults() {
        let json = r#"{"version":2,"sendIntervalMs":500,"minStageChange":0.01,
            "heartbeat":{"intervalMs":1000,"timeoutMs":5000},
            "overdue":{"maxOverdueFraction":0.05},
            "parameters":{"stage":{"name":"Belly","type":"float","synced":true}}}"#;
        let m = Mapping::from_json(json).unwrap();
        assert_eq!(m.get("stage").unwrap().name, "Belly");
        assert!(m.get("bellyExtra").is_some());
    }

    #[test]
    fn bit_budget() {
        let m = Mapping::default();
        // stage 8
        assert_eq!(m.synced_bits(false), 8);
        // + bellyExtra 8 + babyCount 8 + fertileLevel 8 + waiting 1 + fetalMovement 8 + labor 8
        assert_eq!(m.synced_bits(true), 49);
    }

    #[test]
    fn old_files_get_the_new_names() {
        let json = r#"{"version":1,"sendIntervalMs":1000,"minStageChange":0.004,
            "heartbeat":{"intervalMs":1000,"timeoutMs":5000},"overdue":{"maxOverdueFraction":0.05},
            "parameters":{
              "stage":{"name":"MyBelly","type":"float","synced":true},
              "appActive":{"name":"Pregnancy/AppActive","type":"bool"},
              "babyCount":{"name":"PregnancyOSC/BabyCount","type":"int","synced":true,"optional":true},
              "hideBelly":{"name":"Custom/Hide","type":"bool","synced":true},
              "fetalSounds":{"name":"","type":"bool","synced":true}}}"#;
        let m = Mapping::from_json(json).unwrap();
        assert_eq!(m.version, MAPPING_VERSION);
        assert_eq!(m.get("stage").unwrap().name, "MyBelly", "user's own names are kept");
        assert_eq!(m.get("appActive").unwrap().name, "Pregnancy/AppActive");
        assert_eq!(m.get("babyCount").unwrap().name, "Pregnancy/BabyCount");
        assert!(m.parameters.get("hideBelly").is_none());
        assert_eq!(m.sync_role(SyncRole::HideBelly).map(|s| s.name.as_str()), Some("Custom/Hide"), "renamed: kept, with its role");
        assert_eq!(m.sync_role(SyncRole::FetalSounds).map(|s| s.name.as_str()), Some("Pregnancy/FetalSounds"), "off: put back");
        assert_eq!(m.sync.len(), 2);
    }

    #[test]
    fn app_params_cant_be_synced() {
        let json = DEFAULT_MAPPING_JSON.replace(
            r#""sync": ["#,
            r#""sync": [{"name": "Pregnancy/Stage", "type": "float"}, {"name": "Pregnancy/AppActive", "type": "bool"},"#,
        );
        let m = Mapping::from_json(&json).unwrap();
        assert!(m.sync.iter().all(|s| s.name != "Pregnancy/Stage" && s.name != "Pregnancy/AppActive"), "dropped on load");
        assert!(m.sync_def("Pregnancy/Stage").is_none());
        assert!(m.sync_def("Pregnancy/HideBelly").is_some());
    }

    #[test]
    fn v2_active_becomes_app_active() {
        let json = DEFAULT_MAPPING_JSON
            .replace(r#""version": 7"#, r#""version": 2"#)
            .replace(r#""name": "Pregnancy/AppActive""#, r#""name": "Pregnancy/Active""#)
            .replace(r#""name": "Pregnancy/AppHeartbeat""#, r#""name": "Pregnancy/Heartbeat""#);
        let m = Mapping::from_json(&json).unwrap();
        assert_eq!(m.get("appActive").unwrap().name, "Pregnancy/AppActive");
        assert_eq!(m.get("appHeartbeat").unwrap().name, "Pregnancy/AppHeartbeat");
        assert!(m.get("appActive").unwrap().aliases.contains(&"Pregnancy/Active".to_string()));
    }

    #[test]
    fn old_kick_values_become_codes() {
        let old = DEFAULT_MAPPING_JSON
            .replace(r#""version": 7"#, r#""version": 4"#)
            .replace(r#""weak": 1"#, r#""weak": 25"#)
            .replace(r#""strong": 2"#, r#""strong": 60"#)
            .replace(r#""extreme": 3"#, r#""extreme": 90"#);
        let m = Mapping::from_json(&old).unwrap();
        assert_eq!((m.kicks.weak, m.kicks.strong, m.kicks.extreme), (1, 2, 3));
        let mine = old.replace(r#""weak": 25"#, r#""weak": 10"#);
        assert_eq!(Mapping::from_json(&mine).unwrap().kicks.weak, 10, "user's own values are kept");
    }

    #[test]
    fn overdue_becomes_extra_size() {
        let old = DEFAULT_MAPPING_JSON.replace(r#""version": 7"#, r#""version": 5"#).replace(
            r#""bellyExtra": {"#,
            r#""overdue": {"name": "Pregnancy/Overdue", "type": "bool", "synced": true}, "bellyExtra": {"#,
        );
        let m = Mapping::from_json(&old).unwrap();
        assert!(m.parameters.get("overdue").is_none());
        assert_eq!(m.get("bellyExtra").unwrap().name, "Pregnancy/BellyExtra");
    }

    #[test]
    fn everything_uses_one_prefix() {
        let m = Mapping::default();
        assert!(m.parameters.values().all(|p| p.name.starts_with("Pregnancy/")));
        assert!(m.sync.iter().all(|p| p.name.starts_with("Pregnancy/")));
    }

    #[test]
    fn built_in_sync_can_be_renamed_not_removed() {
        // Hide Belly renamed (an older file, no roles), Fetal Sounds removed, one of your own added.
        let mut old = Mapping::default();
        old.sync = vec![
            SyncParam { name: "Custom/HideBelly".into(), kind: ParamType::Bool, role: None },
            SyncParam { name: "Custom/Glasses".into(), kind: ParamType::Bool, role: None },
        ];
        let json = serde_json::to_string(&old).unwrap();
        let mut m = Mapping::from_json(&json).unwrap();
        assert_eq!(m.sync_role(SyncRole::HideBelly).unwrap().name, "Custom/HideBelly");
        assert_eq!(m.sync_role(SyncRole::FetalSounds).unwrap().name, "Pregnancy/FetalSounds", "put back");
        assert!(m.sync.iter().any(|s| s.name == "Custom/Glasses" && s.role.is_none()));
        // Saving without them, or as a number, or with no name: they stay, on/off, named.
        m.sync.retain(|s| s.role != Some(SyncRole::FetalSounds));
        m.sync.iter_mut().filter(|s| s.role == Some(SyncRole::HideBelly)).for_each(|s| (s.kind, s.name) = (ParamType::Int, " ".into()));
        m.apply_locks();
        assert_eq!(m.sync_role(SyncRole::HideBelly).map(|s| (s.name.as_str(), s.kind)), Some(("Pregnancy/HideBelly", ParamType::Bool)));
        assert!(m.sync_role(SyncRole::FetalSounds).is_some());
        assert_eq!(m.sync.len(), 3);
    }

    #[test]
    fn menu_buttons_keep_their_names() {
        let json = DEFAULT_MAPPING_JSON.replace("Pregnancy/Control/Start", "Mine/Start");
        let m = Mapping::from_json(&json).unwrap();
        assert_eq!(m.get("controlStart").unwrap().name, "Pregnancy/Control/Start");
    }
}
