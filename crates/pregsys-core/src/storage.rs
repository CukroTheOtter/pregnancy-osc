//! Versioned JSON storage with migrations, atomic saves and rolling backups.

use crate::model::{AppData, DATA_VERSION};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const KEEP_BACKUPS: usize = 7;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("data is not valid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("this data was saved by a newer app version ({0}); please update the app")]
    TooNew(u32),
}

pub struct Store {
    pub path: PathBuf,
}

/// Upgrades raw JSON from any older version to the current one.
pub fn migrate(mut v: Value) -> Result<Value, StorageError> {
    let mut version = v.get("version").and_then(Value::as_u64).unwrap_or(0) as u32;
    if version > DATA_VERSION {
        return Err(StorageError::TooNew(version));
    }
    while version < DATA_VERSION {
        match version {
            // 0 = files written before versioning existed. Nothing to rename yet.
            0 => {}
            _ => {}
        }
        version += 1;
    }
    if let Some(o) = v.as_object_mut() {
        o.insert("version".into(), Value::from(DATA_VERSION));
    }
    Ok(v)
}

/// Parses a backup or data file of any supported version.
pub fn parse(json: &str) -> Result<AppData, StorageError> {
    let raw: Value = serde_json::from_str(json)?;
    let migrated = migrate(raw)?;
    let mut data: AppData = serde_json::from_value(migrated)?;
    data.upgrade_links();
    Ok(data)
}

pub fn to_json(data: &AppData) -> Result<String, StorageError> {
    Ok(serde_json::to_string_pretty(data)?)
}

impl Store {
    pub fn new(dir: &Path) -> Self {
        Self { path: dir.join("data.json") }
    }

    fn backup_dir(&self) -> PathBuf {
        self.path.with_file_name("backups")
    }

    /// Loads data. A missing file gives fresh data. A broken file is kept
    /// aside as `data.broken-<time>.json` and fresh data is returned, so the
    /// app always starts.
    pub fn load(&self) -> Result<(AppData, Option<String>), StorageError> {
        if !self.path.exists() {
            return Ok((AppData::new(), None));
        }
        let text = fs::read_to_string(&self.path)?;
        match parse(&text) {
            Ok(d) => Ok((d, None)),
            Err(StorageError::TooNew(v)) => Err(StorageError::TooNew(v)),
            Err(e) => {
                let aside = self.path.with_file_name(format!("data.broken-{}.json", crate::now_ms()));
                fs::rename(&self.path, &aside)?;
                Ok((AppData::new(), Some(format!("Saved data could not be read ({e}). It was kept as {}.", aside.display()))))
            }
        }
    }

    /// Writes to a temp file then renames, so a crash never leaves half a file.
    pub fn save(&self, data: &AppData) -> Result<(), StorageError> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, to_json(data)?)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    /// Keeps one backup per day, newest `KEEP_BACKUPS` days.
    pub fn daily_backup(&self, today: &str) -> Result<(), StorageError> {
        if !self.path.exists() {
            return Ok(());
        }
        let dir = self.backup_dir();
        fs::create_dir_all(&dir)?;
        let target = dir.join(format!("data-{today}.json"));
        if !target.exists() {
            fs::copy(&self.path, &target)?;
        }
        let mut all: Vec<PathBuf> = fs::read_dir(&dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("data-")))
            .collect();
        all.sort();
        while all.len() > KEEP_BACKUPS {
            let _ = fs::remove_file(all.remove(0));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Pregnancy;

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::new(dir.path());
        let mut d = AppData::new();
        d.pregnancies.push(Pregnancy { id: "p1".into(), name: "Bean".into(), ..Default::default() });
        s.save(&d).unwrap();
        let (back, warn) = s.load().unwrap();
        assert!(warn.is_none());
        assert_eq!(back, d);
    }

    #[test]
    fn missing_file_is_fresh() {
        let dir = tempfile::tempdir().unwrap();
        let (d, _) = Store::new(dir.path()).load().unwrap();
        assert_eq!(d.version, DATA_VERSION);
    }

    #[test]
    fn broken_file_is_kept_aside() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::new(dir.path());
        fs::write(&s.path, "{ not json").unwrap();
        let (d, warn) = s.load().unwrap();
        assert!(warn.is_some());
        assert!(d.pregnancies.is_empty());
        assert!(fs::read_dir(dir.path()).unwrap().any(|e| e.unwrap().file_name().to_string_lossy().starts_with("data.broken-")));
    }

    #[test]
    fn unversioned_file_migrates() {
        let d = parse(r#"{"settings":{"userSpecies":"fox"}}"#).unwrap();
        assert_eq!(d.version, DATA_VERSION);
        assert_eq!(d.settings.user_species, "fox");
    }

    #[test]
    fn newer_version_is_refused() {
        assert!(matches!(parse(r#"{"version":999}"#), Err(StorageError::TooNew(999))));
    }

    #[test]
    fn backups_rotate() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::new(dir.path());
        s.save(&AppData::new()).unwrap();
        for day in 1..=10 {
            s.daily_backup(&format!("2025-01-{day:02}")).unwrap();
        }
        assert_eq!(fs::read_dir(dir.path().join("backups")).unwrap().count(), KEEP_BACKUPS);
    }
}
