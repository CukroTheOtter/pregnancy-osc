//! Follows the newest VRChat `output_log_*.txt` as it grows. Read-only.

use crate::logparse::{parse_line, LogLine};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Likely VRChat log folders on this machine, best guess first.
pub fn default_log_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(windows) {
        // %LOCALAPPDATA% is ...\AppData\Local; VRChat writes to ...\AppData\LocalLow.
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let mut low = PathBuf::from(local).into_os_string();
            low.push("Low");
            out.push(PathBuf::from(low).join("VRChat").join("VRChat"));
        }
        if let Some(home) = std::env::var_os("USERPROFILE") {
            out.push(PathBuf::from(home).join("AppData").join("LocalLow").join("VRChat").join("VRChat"));
        }
    } else if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        let tail = Path::new("steamapps/compatdata/438100/pfx/drive_c/users/steamuser/AppData/LocalLow/VRChat/VRChat");
        for steam in [
            ".steam/steam",
            ".local/share/Steam",
            ".var/app/com.valvesoftware.Steam/.local/share/Steam",
            ".var/app/com.valvesoftware.Steam/data/Steam",
        ] {
            out.push(home.join(steam).join(tail));
        }
    }
    out.dedup();
    out
}

/// The first default folder that exists.
pub fn find_log_dir() -> Option<PathBuf> {
    default_log_dirs().into_iter().find(|p| p.is_dir())
}

/// The avatar's name from VRChat's OSC config (`OSC/usr_*/Avatars/<id>.json`,
/// next to the logs). VRChat writes it when the avatar loads with OSC on, so
/// it matches the ID exactly, unlike the log, which only has the name.
pub fn osc_avatar_name(vrchat_dir: &Path, avatar_id: &str) -> Option<String> {
    osc_avatar_files(vrchat_dir, avatar_id)
        .find_map(|json| json.get("name").and_then(|n| n.as_str()).map(str::trim).filter(|n| !n.is_empty()).map(str::to_string))
}

/// The avatar's parameter names from the same file (built-ins like AFK
/// included), for avatars the app hasn't read over OSCQuery yet.
pub fn osc_avatar_params(vrchat_dir: &Path, avatar_id: &str) -> Option<Vec<String>> {
    osc_avatar_files(vrchat_dir, avatar_id).find_map(|json| {
        let list = json.get("parameters")?.as_array()?;
        let names: Vec<String> = list.iter().filter_map(|p| p.get("name")?.as_str().map(str::to_string)).collect();
        (!names.is_empty()).then_some(names)
    })
}

/// VRChat's OSC files for an avatar (one per VRChat account that wore it).
fn osc_avatar_files(vrchat_dir: &Path, avatar_id: &str) -> impl Iterator<Item = serde_json::Value> {
    let valid = avatar_id.starts_with("avtr_") && avatar_id[5..].chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    let users = if valid { std::fs::read_dir(vrchat_dir.join("OSC")).ok() } else { None };
    let file = format!("{avatar_id}.json");
    users.into_iter().flatten().flatten().filter_map(move |user| {
        let text = std::fs::read_to_string(user.path().join("Avatars").join(&file)).ok()?;
        serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()
    })
}

/// Newest `output_log_*.txt` in a folder.
pub fn newest_log(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.starts_with("output_log_") && n.ends_with(".txt")
        })
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p)
}

/// Incrementally reads new lines from the newest log.
pub struct LogWatcher {
    dir: PathBuf,
    file: Option<PathBuf>,
    offset: u64,
    partial: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct Poll {
    pub lines: Vec<LogLine>,
    /// A different log file was picked up (VRChat restarted).
    pub new_file: bool,
}

impl LogWatcher {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir, file: None, offset: 0, partial: Vec::new() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn current_file(&self) -> Option<&Path> {
        self.file.as_deref()
    }

    /// Reads whatever was appended since the last poll. On a new file, reads
    /// it from the start so the instance state can be rebuilt.
    pub fn poll(&mut self) -> std::io::Result<Poll> {
        let mut out = Poll::default();
        let Some(newest) = newest_log(&self.dir) else { return Ok(out) };
        if self.file.as_ref() != Some(&newest) {
            self.file = Some(newest.clone());
            self.offset = 0;
            self.partial.clear();
            out.new_file = true;
        }
        let mut f = File::open(&newest)?;
        let len = f.metadata()?.len();
        if len < self.offset {
            // Truncated or replaced: start over.
            self.offset = 0;
            self.partial.clear();
            out.new_file = true;
        }
        if len == self.offset {
            return Ok(out);
        }
        f.seek(SeekFrom::Start(self.offset))?;
        let mut buf = Vec::with_capacity((len - self.offset).min(64 << 20) as usize);
        f.take(64 << 20).read_to_end(&mut buf)?;
        self.offset += buf.len() as u64;

        self.partial.extend_from_slice(&buf);
        // Keep an unfinished last line for the next poll.
        let cut = self.partial.iter().rposition(|b| *b == b'\n').map(|i| i + 1).unwrap_or(0);
        let complete: Vec<u8> = self.partial.drain(..cut).collect();
        let text = String::from_utf8_lossy(&complete);
        out.lines = text.lines().filter_map(parse_line).collect();
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avatar_name_from_vrchat_osc_config() {
        let dir = std::env::temp_dir().join(format!("pregsys-osc-{}", std::process::id()));
        let avatars = dir.join("OSC").join("usr_1").join("Avatars");
        std::fs::create_dir_all(&avatars).unwrap();
        std::fs::write(avatars.join("avtr_ab-12.json"), "\u{feff}{\"id\":\"avtr_ab-12\",\"name\":\"Fox （OwO Edition）\"}").unwrap();
        assert_eq!(osc_avatar_name(&dir, "avtr_ab-12").as_deref(), Some("Fox （OwO Edition）"));
        assert_eq!(osc_avatar_name(&dir, "avtr_ff"), None);
        assert_eq!(osc_avatar_name(&dir, "../avtr_ab-12"), None, "only real avatar IDs");
        assert_eq!(osc_avatar_params(&dir, "avtr_ab-12"), None, "no parameters listed");
        std::fs::write(avatars.join("avtr_cd.json"), r#"{"id":"avtr_cd","name":"Cat","parameters":[{"name":"AFK"},{"name":"Pregnancy/Stage"}]}"#).unwrap();
        assert_eq!(osc_avatar_params(&dir, "avtr_cd"), Some(vec!["AFK".to_string(), "Pregnancy/Stage".to_string()]));
        let _ = std::fs::remove_dir_all(&dir);
    }
    use crate::logparse::LogEvent;
    use std::io::Write;

    #[test]
    fn follows_appends_and_partial_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("output_log_2025-03-01_20-00-00.txt");
        let mut f = File::create(&path).unwrap();
        write!(f, "2025.03.01 21:00:05 Debug      -  [Behaviour] OnPlayerJoined A (usr_a)\r\n2025.03.01 21:00:06 Debug      -  [Behaviour] OnPlayerJo").unwrap();
        f.flush().unwrap();

        let mut w = LogWatcher::new(dir.path().to_path_buf());
        let p = w.poll().unwrap();
        assert!(p.new_file);
        assert_eq!(p.lines.len(), 1);

        write!(f, "ined B (usr_b)\r\n").unwrap();
        f.flush().unwrap();
        let p = w.poll().unwrap();
        assert!(!p.new_file);
        assert_eq!(p.lines.len(), 1);
        assert_eq!(p.lines[0].event, LogEvent::PlayerJoined { name: "B".into(), user_id: Some("usr_b".into()) });

        assert!(w.poll().unwrap().lines.is_empty());
    }

    #[test]
    fn empty_or_missing_dir_is_fine() {
        let mut w = LogWatcher::new(PathBuf::from("this/does/not/exist"));
        assert!(w.poll().unwrap().lines.is_empty());
    }
}
