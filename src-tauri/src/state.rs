//! State shared between the UI commands and the background engine.

use pregsys_core::instance::Person;
use pregsys_core::sps::SocketLive;
use pregsys_core::mapping::Mapping;
use pregsys_core::model::AppData;
use pregsys_core::storage::Store;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Mutex;

pub enum EngineCmd {
    /// Settings or data changed: re-send everything.
    Resync,
    /// A Sync value was set in the app: drop any in-game change still held
    /// for it (it's older), then re-send.
    SyncSet(String),
    /// Mapping file or OSC settings changed: restart the connection.
    Restart,
    /// Short true/false pulse on an event parameter.
    Pulse(String, i64),
    /// Debug: pretend a WooHoo happened. `Some(roll)` forces the outcome.
    DebugWoohoo(Option<f64>),
    /// Debug: kick right now.
    DebugKick(pregsys_core::movement::KickStrength),
    /// Exit: hand off to the avatar, save, then reply.
    Shutdown(Sender<()>),
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub vrchat_running: bool,
    pub steamvr_running: bool,
    pub osc_active: bool,
    pub oscquery: bool,
    pub vrchat_found: bool,
    pub send_target: Option<String>,
    pub recv_port: Option<u16>,
    /// Also listening on VRChat's own output port (9001).
    pub backup_port: Option<u16>,
    /// The app's OSCQuery web port.
    pub query_port: Option<u16>,
    /// VRChat's OSCQuery address.
    pub vrchat_query: Option<String>,
    /// When anything last arrived from VRChat.
    pub last_heard_ms: Option<i64>,
    pub avatar_id: Option<String>,
    pub avatar_params: Option<Vec<String>>,
    /// Switching avatars: the new one is still loading.
    pub avatar_loading: bool,
    /// The Hide Belly Sync parameter's name (it can be renamed).
    pub hide_belly: Option<String>,
    pub last_stage: Option<f32>,
    pub last_extra: Option<f32>,
    pub log_dir: Option<String>,
    pub log_file: Option<String>,
    pub world_name: Option<String>,
    pub others: Vec<Person>,
    pub note: Option<String>,
    pub tray_available: bool,
    /// SPS sockets found on the current avatar, live values, and how close a session is to counting.
    pub sps_found: Vec<String>,
    pub sps_live: Vec<SocketLive>,
    pub sps_progress: f64,
    /// VRChat says you're AFK (labor waits).
    pub afk: bool,
    /// Standing up (not seated or lying down), for "resting holds off labor".
    pub up: bool,
    /// VRChat's Upright value, for tuning (debug).
    pub upright: Option<f32>,
    /// Where "up or resting" comes from: "steamvr" or "vrchat".
    pub posture_source: String,
    /// SteamVR: headset below its highest (m), and lying down by the hip tracker.
    pub vr_drop_m: Option<f32>,
    pub vr_lying: Option<bool>,
    /// SteamVR: full-body trackers, and whether one is set as your waist.
    pub vr_trackers: u32,
    pub vr_waist: bool,
    /// SteamVR: the waist tracker is being tracked right now.
    pub vr_hip_tracked: bool,
    /// The worn avatar can show labor (it has the labor parameter).
    pub labor_supported: bool,
    /// Another avatar sharing this pregnancy that can show labor (when this one can't).
    pub labor_avatar: Option<String>,
    /// The current pregnancy's partner is in the instance (None = can't tell).
    pub partner_here: Option<bool>,
    /// Labor time so far on the current pregnancy (for the progress bar).
    pub labor_elapsed_ms: Option<i64>,
    /// Labor is waiting right now (the saved copy only updates now and then).
    pub labor_paused: Option<bool>,
    pub mapping_path: String,
    pub data_dir: String,
}

/// Your posture from SteamVR's tracking (see `steamvr_posture`).
#[derive(Debug, Clone, Copy)]
pub struct VrPosture {
    pub reading: pregsys_core::posture::Reading,
    /// Full-body trackers SteamVR has.
    pub trackers: u32,
    /// One of them is set as your waist (SteamVR > Manage Trackers).
    pub waist: bool,
    /// The waist tracker is being tracked right now.
    pub hip_tracked: bool,
    pub at_ms: i64,
}

pub struct Shared {
    pub data: Mutex<AppData>,
    pub store: Store,
    pub data_dir: PathBuf,
    pub mapping_path: PathBuf,
    pub mapping: Mutex<Mapping>,
    pub status: Mutex<Status>,
    pub engine: Mutex<Option<Sender<EngineCmd>>>,
    pub dirty: AtomicBool,
    pub load_warning: Mutex<Option<String>>,
    pub tray_available: AtomicBool,
    pub quitting: AtomicBool,
    /// Posture from SteamVR, while "Resting holds off labor" is on and SteamVR runs.
    pub vr_posture: Mutex<Option<VrPosture>>,
}

impl Shared {
    pub fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::SeqCst);
    }

    pub fn send(&self, cmd: EngineCmd) {
        if let Some(tx) = self.engine.lock().unwrap().as_ref() {
            let _ = tx.send(cmd);
        }
    }

    /// Saves now if anything changed.
    pub fn save_if_dirty(&self) -> Result<(), String> {
        if self.dirty.swap(false, Ordering::SeqCst) {
            let data = self.data.lock().unwrap().clone();
            if let Err(e) = self.store.save(&data) {
                self.dirty.store(true, Ordering::SeqCst);
                return Err(e.to_string());
            }
        }
        Ok(())
    }
}
