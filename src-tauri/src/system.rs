//! Process detection (is VRChat / SteamVR running?) and SteamVR autostart.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

pub const APP_KEY: &str = "com.pregsys.companion";

pub struct Procs {
    sys: System,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Running {
    pub vrchat: bool,
    pub steamvr: bool,
    /// XSOverlay and OVR Toolkit, for notifications in VR (see `vr_notify`).
    pub xsoverlay: bool,
    pub ovr_toolkit: bool,
}

impl Procs {
    pub fn new() -> Self {
        Self { sys: System::new() }
    }

    pub fn check(&mut self) -> Running {
        self.sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
        let mut r = Running::default();
        for p in self.sys.processes().values() {
            let n = p.name().to_string_lossy().to_lowercase();
            // Under Proton the Windows name shows up too.
            if n == "vrchat.exe" || n == "vrchat" {
                r.vrchat = true;
            }
            if n.starts_with("vrserver") || n.starts_with("vrmonitor") {
                r.steamvr = true;
            }
            if n.starts_with("xsoverlay") {
                r.xsoverlay = true;
            }
        }
        // OVR Toolkit counts when its notification API answers (its process
        // name doesn't matter then, and an API that's off doesn't count).
        r.ovr_toolkit = crate::vr_notify::ovr_toolkit_listening();
        r
    }
}

/// OpenVR's path file, which points at SteamVR's config folder.
fn openvr_paths_file() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("openvr").join("openvrpaths.vrpath"))
    } else {
        let cfg = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(cfg.join("openvr").join("openvrpaths.vrpath"))
    }
}

fn steamvr_config_dir() -> Option<PathBuf> {
    let text = std::fs::read_to_string(openvr_paths_file()?).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    let dir = v.get("config")?.as_array()?.first()?.as_str()?;
    Some(PathBuf::from(dir))
}

fn read_json(path: &Path) -> Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| json!({}))
}

fn write_json(path: &Path, v: &Value) -> Result<(), String> {
    let tmp = path.with_extension("pregsys.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// Writes our `.vrmanifest` next to the app's data.
fn write_manifest(dir: &Path) -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.to_string_lossy().to_string();
    let manifest = json!({
        "source": "builtin",
        "applications": [{
            "app_key": APP_KEY,
            "launch_type": "binary",
            "binary_path_windows": exe,
            "binary_path_linux": exe,
            "arguments": "--autostart",
            "is_dashboard_overlay": true,
            "strings": { "en_us": {
                "name": "Pregnancy OSC",
                "description": "Keeps your avatar's belly in sync over OSC"
            }}
        }]
    });
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = dir.join("pregsys-companion.vrmanifest");
    write_json(&path, &manifest)?;
    Ok(path)
}

/// Turns "Start with SteamVR" on or off. Returns a note for the user.
///
/// Registers our manifest in SteamVR's `appconfig.json` and sets the
/// app's autolaunch flag in `steamvr.vrsettings`. SteamVR rewrites these
/// files when it closes, so changes made while it runs may be lost.
pub fn set_steamvr_autostart(data_dir: &Path, enable: bool, steamvr_running: bool) -> Result<String, String> {
    let cfg = steamvr_config_dir().ok_or("SteamVR config folder not found. Is SteamVR installed?")?;
    let manifest = write_manifest(data_dir)?;
    let manifest_str = manifest.to_string_lossy().to_string();

    let appconfig = cfg.join("appconfig.json");
    let mut a = read_json(&appconfig);
    let list = a
        .as_object_mut()
        .ok_or("appconfig.json is not an object")?
        .entry("manifest_paths")
        .or_insert_with(|| json!([]));
    if let Some(arr) = list.as_array_mut() {
        arr.retain(|p| p.as_str() != Some(&manifest_str));
        if enable {
            arr.push(json!(manifest_str));
        }
    }
    write_json(&appconfig, &a)?;

    let settings = cfg.join("steamvr.vrsettings");
    if settings.exists() {
        let mut s = read_json(&settings);
        if let Some(o) = s.as_object_mut() {
            if enable {
                o.insert(APP_KEY.into(), json!({ "autolaunch": true }));
            } else {
                o.remove(APP_KEY);
            }
        }
        write_json(&settings, &s)?;
    }

    let mut note = if enable {
        "Registered with SteamVR. If it doesn't start next time, turn it on in SteamVR Settings > Startup / Shutdown > Choose Startup Overlay Apps.".to_string()
    } else {
        "Removed from SteamVR startup.".to_string()
    };
    if steamvr_running {
        note.push_str(" SteamVR is running now, so restart it for this to take effect.");
    }
    Ok(note)
}
