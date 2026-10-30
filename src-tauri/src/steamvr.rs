//! The app's SteamVR connection, for two things:
//! - Posture for "Resting holds off labor" (see `pregsys_core::posture`):
//!   where your headset and waist tracker are. Read only.
//! - SteamVR notifications, the fallback for notifications in VR (see
//!   `vr_notify`): shown from a hidden, empty overlay that only owns them.
//!
//! It connects only while SteamVR runs and one of those is wanted: as a
//! background app for posture alone (never starts SteamVR or keeps it
//! running), or as an overlay app when notifications are on (after checking
//! SteamVR is up as a background app first, so it never starts SteamVR
//! either). Settings changes reconnect it the right way.
//!
//! Uses Valve's `openvr_api.dll` (shipped next to the app) through its C
//! function tables, so there's nothing to build. Windows only.

#[cfg(not(windows))]
pub fn spawn(_app: tauri::AppHandle, _shared: std::sync::Arc<crate::state::Shared>) {}

#[cfg(windows)]
pub use win::spawn;

#[cfg(windows)]
mod win {
    use crate::state::Shared;
    use pregsys_core::posture::{MAX_STANDING_M, MIN_STANDING_M};
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tauri::{AppHandle, Manager};
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    // From openvr_capi.h (OpenVR 2.5.1).
    /// VRApplication_Overlay: overlays and notifications.
    const APP_OVERLAY: i32 = 2;
    /// VRApplication_Background: doesn't start SteamVR, doesn't keep it running.
    const APP_BACKGROUND: i32 = 3;
    const SYSTEM_VERSION: &[u8] = b"FnTable:IVRSystem_022\0";
    const SETTINGS_VERSION: &[u8] = b"FnTable:IVRSettings_003\0";
    const OVERLAY_VERSION: &[u8] = b"FnTable:IVROverlay_027\0";
    const NOTIFICATIONS_VERSION: &[u8] = b"FnTable:IVRNotifications_002\0";
    /// Standing space: the floor is at 0 (moves with playspace movers).
    const UNIVERSE_STANDING: i32 = 1;
    /// Raw tracking space: the room as set up, before any playspace mover.
    const UNIVERSE_RAW: i32 = 2;
    const MAX_DEVICES: usize = 64;
    const CLASS_HMD: i32 = 1;
    const CLASS_TRACKER: i32 = 3;
    /// ETrackingResult: Running_OK.
    const TRACKING_OK: i32 = 200;
    /// EDeviceActivityLevel: Unknown, and in use (the headset's on your head).
    const ACTIVITY_UNKNOWN: i32 = -1;
    const ACTIVITY_IN_USE: i32 = 1;
    const PROP_REGISTERED_TYPE: i32 = 1036;
    const PROP_CONTROLLER_TYPE: i32 = 7000;
    const EV_QUIT: u32 = 700;
    /// Room setup done, or the standing pose reset: the floor may have moved.
    const EV_ROOM_SETUP_COMMITTED: u32 = 807;
    const EV_STANDING_ZERO_RESET: u32 = 808;
    /// EVRNotificationType_Transient, EVRNotificationStyle_None.
    const NOTIFICATION_TRANSIENT: i32 = 0;
    const NOTIFICATION_STYLE_NONE: i32 = 0;
    /// The hidden overlay that owns our notifications.
    const NOTICE_OVERLAY_KEY: &[u8] = b"com.pregsys.companion.notices\0";
    const NOTICE_OVERLAY_NAME: &[u8] = b"Pregnancy OSC\0";
    // Function table slots (counted from the header's FnTable structs).
    const SYS_POSES: usize = 11;
    const SYS_ACTIVITY: usize = 15;
    const SYS_DEVICE_CLASS: usize = 19;
    const SYS_STRING_PROP: usize = 27;
    const SYS_POLL_NEXT_EVENT: usize = 29;
    const SYS_ACK_QUIT: usize = 43;
    const SET_GET_STRING: usize = 8;
    const OVERLAY_CREATE: usize = 1;
    const NOTIFICATIONS_CREATE: usize = 0;

    /// `VREvent_t` on Windows: 3 u32s, padding, then a 48-byte union.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct VrEvent {
        kind: u32,
        device: u32,
        age: f32,
        _pad: u32,
        data: [u64; 6],
    }

    /// `TrackedDevicePose_t`: a 3x4 pose matrix, velocities, result and flags.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Pose {
        m: [[f32; 4]; 3],
        velocity: [f32; 3],
        angular: [f32; 3],
        result: i32,
        valid: bool,
        connected: bool,
    }

    /// Looks for SteamVR this often.
    const RETRY: Duration = Duration::from_secs(5);
    /// Reads the posture (and shows waiting notices) this often.
    const TICK: Duration = Duration::from_millis(250);
    /// Looks for the waist tracker (roles can change) this often.
    const TRACKERS_EVERY: Duration = Duration::from_secs(5);
    /// The saved normal height is updated when a proven one is this much higher.
    const SAVE_RISE_M: f32 = 0.02;
    /// The headset above the floor while worn: outside this, the floor is wrong.
    const FLOOR_SANE: std::ops::RangeInclusive<f32> = 0.2..=2.6;

    type InitFn = unsafe extern "system" fn(*mut i32, i32) -> isize;
    type ShutdownFn = unsafe extern "system" fn();
    type GetIfaceFn = unsafe extern "system" fn(*const u8, *mut i32) -> isize;

    struct Api {
        shutdown: ShutdownFn,
        sys: *const usize,
        settings: *const usize,
        /// Overlay mode only: the notifications table and our hidden overlay.
        notices: Option<(*const usize, u64)>,
    }

    impl Api {
        /// A function from a table slot.
        unsafe fn f<T: Copy>(table: *const usize, i: usize) -> T {
            let p = *table.add(i);
            std::mem::transmute_copy(&p)
        }

        fn poll_system(&self) -> Option<VrEvent> {
            let f: unsafe extern "system" fn(*mut VrEvent, u32) -> bool = unsafe { Self::f(self.sys, SYS_POLL_NEXT_EVENT) };
            let mut e: VrEvent = unsafe { std::mem::zeroed() };
            unsafe { f(&mut e, std::mem::size_of::<VrEvent>() as u32) }.then_some(e)
        }

        fn ack_quit(&self) {
            let f: unsafe extern "system" fn() = unsafe { Self::f(self.sys, SYS_ACK_QUIT) };
            unsafe { f() }
        }

        /// Every device's pose in a tracking space.
        fn poses(&self, universe: i32) -> Vec<Pose> {
            let f: unsafe extern "system" fn(i32, f32, *mut Pose, u32) = unsafe { Self::f(self.sys, SYS_POSES) };
            let mut out: Vec<Pose> = vec![unsafe { std::mem::zeroed() }; MAX_DEVICES];
            unsafe { f(universe, 0.0, out.as_mut_ptr(), MAX_DEVICES as u32) };
            out
        }

        /// The device is being worn or used (unknown counts as yes).
        fn in_use(&self, i: usize) -> bool {
            let f: unsafe extern "system" fn(u32) -> i32 = unsafe { Self::f(self.sys, SYS_ACTIVITY) };
            matches!(unsafe { f(i as u32) }, ACTIVITY_IN_USE | ACTIVITY_UNKNOWN)
        }

        fn class(&self, i: usize) -> i32 {
            let f: unsafe extern "system" fn(u32) -> i32 = unsafe { Self::f(self.sys, SYS_DEVICE_CLASS) };
            unsafe { f(i as u32) }
        }

        fn string_prop(&self, i: usize, prop: i32) -> String {
            let f: unsafe extern "system" fn(u32, i32, *mut u8, u32, *mut i32) -> u32 = unsafe { Self::f(self.sys, SYS_STRING_PROP) };
            let mut buf = [0u8; 256];
            let mut err = 0;
            unsafe { f(i as u32, prop, buf.as_mut_ptr(), buf.len() as u32, &mut err) };
            c_string(&buf)
        }

        fn setting(&self, section: &str, key: &str) -> String {
            let f: unsafe extern "system" fn(*const u8, *const u8, *mut u8, u32, *mut i32) = unsafe { Self::f(self.settings, SET_GET_STRING) };
            let (section, key) = (format!("{section}\0"), format!("{key}\0"));
            let mut buf = [0u8; 256];
            let mut err = 0;
            unsafe { f(section.as_ptr(), key.as_ptr(), buf.as_mut_ptr(), buf.len() as u32, &mut err) };
            if err != 0 {
                return String::new();
            }
            c_string(&buf)
        }

        /// Full-body trackers, and which one is your waist (SteamVR >
        /// Manage Trackers saves roles under "trackers", keyed by device path).
        fn trackers(&self) -> (u32, Option<usize>) {
            let mut count = 0;
            let mut waist = None;
            for i in 0..MAX_DEVICES {
                if self.class(i) != CLASS_TRACKER {
                    continue;
                }
                count += 1;
                let role = self.setting("trackers", &format!("/devices/{}", self.string_prop(i, PROP_REGISTERED_TYPE)));
                let kind = self.string_prop(i, PROP_CONTROLLER_TYPE).to_lowercase();
                if role.contains("Waist") || kind.contains("waist") {
                    waist = Some(i);
                }
            }
            (count, waist)
        }

        /// Shows a notification in the headset (overlay mode only).
        fn notify(&self, text: &str) -> Result<(), String> {
            let Some((table, overlay)) = self.notices else { return Err("not connected for notifications".into()) };
            let f: unsafe extern "system" fn(u64, u64, i32, *const u8, i32, *const u8, *mut u32) -> i32 = unsafe { Self::f(table, NOTIFICATIONS_CREATE) };
            let text = format!("{}\0", text.replace('\0', ""));
            let mut id = 0u32;
            let err = unsafe { f(overlay, 0, NOTIFICATION_TRANSIENT, text.as_ptr(), NOTIFICATION_STYLE_NONE, std::ptr::null(), &mut id) };
            if err == 0 { Ok(()) } else { Err(format!("SteamVR notification error {err}")) }
        }
    }

    fn c_string(buf: &[u8]) -> String {
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        String::from_utf8_lossy(&buf[..end]).to_string()
    }

    /// A pose's position (x, y, z, y up), if it's tracked properly right now
    /// (not "valid but out of range" or calibrating, where glitches live).
    fn position(p: &Pose) -> Option<[f32; 3]> {
        (p.valid && p.connected && p.result == TRACKING_OK).then_some([p.m[0][3], p.m[1][3], p.m[2][3]])
    }

    /// How upright a pose is: its up direction's vertical share (1 = level).
    fn upright(p: &Pose) -> f32 {
        p.m[1][1]
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Loads OpenVR and connects (only while SteamVR runs). For notifications
    /// it connects as an overlay app with a hidden overlay to own them.
    fn connect(app: &AppHandle, notices: bool) -> Result<Api, String> {
        let dll = app.path().resource_dir().map_err(|e| e.to_string())?.join("openvr_api.dll");
        let lib = unsafe { LoadLibraryW(wide(&dll.to_string_lossy()).as_ptr()) };
        if lib.is_null() {
            return Err(format!("openvr_api.dll not found at {}", dll.display()));
        }
        let get = |name: &[u8]| unsafe { GetProcAddress(lib, name.as_ptr()) }.ok_or_else(|| "openvr_api.dll is missing a function".to_string());
        let init: InitFn = unsafe { std::mem::transmute(get(b"VR_InitInternal\0")?) };
        let shutdown: ShutdownFn = unsafe { std::mem::transmute(get(b"VR_ShutdownInternal\0")?) };
        let iface: GetIfaceFn = unsafe { std::mem::transmute(get(b"VR_GetGenericInterface\0")?) };
        let mut err = 0;
        // Always as a background app first: it fails if SteamVR isn't up, so
        // the overlay connection below can never start SteamVR.
        unsafe { init(&mut err, APP_BACKGROUND) };
        if err != 0 {
            return Err(format!("SteamVR said no (error {err})"));
        }
        if notices {
            unsafe { shutdown() };
            unsafe { init(&mut err, APP_OVERLAY) };
            if err != 0 {
                return Err(format!("SteamVR said no to an overlay (error {err})"));
            }
        }
        let sys = unsafe { iface(SYSTEM_VERSION.as_ptr(), &mut err) } as *const usize;
        let settings = unsafe { iface(SETTINGS_VERSION.as_ptr(), &mut err) } as *const usize;
        if sys.is_null() || settings.is_null() || err != 0 {
            unsafe { shutdown() };
            return Err(format!("SteamVR is too old (error {err})"));
        }
        let mut api = Api { shutdown, sys, settings, notices: None };
        if notices {
            let overlay = unsafe { iface(OVERLAY_VERSION.as_ptr(), &mut err) } as *const usize;
            let table = unsafe { iface(NOTIFICATIONS_VERSION.as_ptr(), &mut err) } as *const usize;
            if overlay.is_null() || table.is_null() || err != 0 {
                unsafe { shutdown() };
                return Err(format!("SteamVR has no notifications (error {err})"));
            }
            let create: unsafe extern "system" fn(*const u8, *const u8, *mut u64) -> i32 = unsafe { Api::f(overlay, OVERLAY_CREATE) };
            let mut handle = 0u64;
            let e = unsafe { create(NOTICE_OVERLAY_KEY.as_ptr(), NOTICE_OVERLAY_NAME.as_ptr(), &mut handle) };
            if e != 0 {
                unsafe { shutdown() };
                return Err(format!("SteamVR overlay error {e}"));
            }
            api.notices = Some((table, handle));
        }
        Ok(api)
    }

    pub fn spawn(app: AppHandle, shared: Arc<Shared>) {
        let _ = std::thread::Builder::new().name("steamvr".into()).spawn(move || run(app, shared));
    }

    /// What the connection is for right now.
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Want {
        /// "Resting holds off labor" and "Use SteamVR tracking" are on (off:
        /// the engine goes by VRChat's values alone).
        posture: bool,
        /// SteamVR notifications are on (Settings > Notifications > In VR).
        notices: bool,
    }

    /// None while SteamVR isn't running or nothing needs it.
    fn want(shared: &Shared) -> Option<Want> {
        let w = {
            let s = &shared.data.lock().unwrap().settings;
            Want { posture: s.labor.resting_holds && s.vr_posture, notices: s.vr_notices.steamvr }
        };
        let running = shared.status.lock().unwrap().steamvr_running;
        (running && (w.posture || w.notices)).then_some(w)
    }

    fn run(app: AppHandle, shared: Arc<Shared>) {
        loop {
            std::thread::sleep(RETRY);
            let Some(w) = want(&shared) else { continue };
            let api = match connect(&app, w.notices) {
                Ok(a) => a,
                Err(e) => {
                    log::warn!("SteamVR: {e}");
                    continue;
                }
            };
            session(&shared, &api, w);
            *shared.vr_posture.lock().unwrap() = None;
            unsafe { (api.shutdown)() };
        }
    }

    /// Connected: reads your posture and shows notices (whichever is wanted)
    /// until SteamVR quits or what's wanted changes.
    fn session(shared: &Shared, api: &Api, w: Want) {
        let mut posture = w.posture.then(|| PostureReader::new(shared));
        let mut last_check = Instant::now();
        loop {
            while let Some(e) = api.poll_system() {
                match e.kind {
                    EV_QUIT => {
                        api.ack_quit();
                        return;
                    }
                    EV_ROOM_SETUP_COMMITTED | EV_STANDING_ZERO_RESET => {
                        if let Some(p) = posture.as_mut() {
                            p.floor = None;
                        }
                    }
                    _ => {}
                }
            }
            if last_check.elapsed() >= Duration::from_secs(1) {
                last_check = Instant::now();
                if want(shared) != Some(w) {
                    return;
                }
                if let Some(p) = posture.as_mut() {
                    p.check_settings(shared);
                }
            }
            if let Some(p) = posture.as_mut() {
                p.read(shared, api);
            }
            if w.notices {
                show_notices(shared, api);
            }
            std::thread::sleep(TICK);
        }
    }

    /// Shows the notices `vr_notify` queued for SteamVR (old ones are dropped).
    fn show_notices(shared: &Shared, api: &Api) {
        let waiting: Vec<_> = std::mem::take(&mut *shared.vr_notice_queue.lock().unwrap());
        for (text, at) in waiting {
            if at.elapsed() < crate::vr_notify::STEAMVR_QUEUE_MAX_AGE {
                if let Err(e) = api.notify(&text) {
                    log::warn!("SteamVR: {e}");
                }
            }
        }
    }

    /// "Set height automatically" (Settings > Labor and birth > Resting).
    fn height_auto(shared: &Shared) -> bool {
        shared.data.lock().unwrap().settings.vr_height_auto
    }

    /// Your saved standing height above the floor (`VrPosture::seed` checks it).
    fn saved_standing(shared: &Shared) -> Option<f32> {
        shared.data.lock().unwrap().vr_standing_m
    }

    /// Reads your posture each tick, for "Resting holds off labor".
    struct PostureReader {
        posture: pregsys_core::posture::VrPosture,
        last_trackers: Instant,
        trackers: (u32, Option<usize>),
        /// The floor's height in raw space (raw minus standing), taken once
        /// (and again after a room setup), so a playspace mover can't shift it.
        floor: Option<f32>,
        saved: Option<f32>,
        ever_worn: bool,
    }

    impl PostureReader {
        fn new(shared: &Shared) -> Self {
            let mut posture = pregsys_core::posture::VrPosture::default();
            posture.set_auto(height_auto(shared));
            Self {
                posture,
                last_trackers: Instant::now() - TRACKERS_EVERY,
                trackers: (0, None),
                floor: None,
                saved: saved_standing(shared),
                ever_worn: false,
            }
        }

        /// Once a second: "Forget" and "Set height automatically".
        fn check_settings(&mut self, shared: &Shared) {
            // "Forget": learn your height again from here.
            let now_saved = saved_standing(shared);
            if self.saved.is_some() && now_saved.is_none() {
                self.posture = Default::default();
                self.floor = None;
            }
            self.saved = now_saved;
            self.posture.set_auto(height_auto(shared));
        }

        fn read(&mut self, shared: &Shared, api: &Api) {
            if self.last_trackers.elapsed() >= TRACKERS_EVERY {
                self.last_trackers = Instant::now();
                self.trackers = api.trackers();
            }
            let poses = api.poses(UNIVERSE_RAW);
            let hmd = (0..MAX_DEVICES).find(|&i| api.class(i) == CLASS_HMD);
            let head = hmd.and_then(|i| position(&poses[i]));
            // Until SteamVR first says it's on your head, trust it is (some
            // headsets may never say so).
            let in_use = hmd.is_some_and(|i| api.in_use(i));
            self.ever_worn |= in_use;
            let worn = in_use || !self.ever_worn;
            if let (None, Some(i), Some(h), true) = (self.floor, hmd, head, worn) {
                // Raw and standing space only differ by where the floor is (and
                // a turn). A worn headset under the floor or way above it means
                // the room setup's floor is off: no floor, so nothing is saved
                // or seeded (try again next reading).
                if let Some(s) = position(&api.poses(UNIVERSE_STANDING)[i]).filter(|s| FLOOR_SANE.contains(&s[1])) {
                    self.floor = Some(h[1] - s[1]);
                    self.posture.set_floor(h[1] - s[1]);
                    if let Some(top) = self.saved {
                        self.posture.seed(top);
                    }
                }
            }
            let up = hmd.map_or(1.0, |i| upright(&poses[i]));
            let hip = self.trackers.1.and_then(|i| position(&poses[i]));
            let now = pregsys_core::now_ms();
            // "Set height now": your headset's height above the floor, right now.
            if let Some(tx) = shared.vr_set_height.lock().unwrap().take() {
                let measured = match (head, self.floor) {
                    (None, _) => Err("Your headset isn't being tracked right now.".to_string()),
                    _ if !in_use && self.ever_worn => Err("Put your headset on first.".to_string()),
                    (_, None) => Err("SteamVR's floor isn't set (or looks wrong), so your height can't be measured. Run Room Setup in SteamVR.".to_string()),
                    (Some(h), Some(f)) => {
                        let m = h[1] - f;
                        if (MIN_STANDING_M..=MAX_STANDING_M).contains(&m) {
                            Ok(m)
                        } else {
                            Err(format!("That reads as {m:.2} m above the floor, which can't be right. Check SteamVR's floor (Room Setup)."))
                        }
                    }
                };
                if let Ok(m) = measured {
                    self.posture.set_height(m);
                    self.saved = Some(m);
                    shared.data.lock().unwrap().vr_standing_m = Some(m);
                    shared.mark_dirty();
                }
                let _ = tx.send(measured);
            }
            let reading = head.map(|h| self.posture.update(h, up, hip, worn, now));
            // Saves your normal height when a higher one is proven. It only
            // goes up (a seated session mustn't lower it); "Forget" resets it.
            if let Some(h) = self.posture.proven_m() {
                if self.saved.is_none_or(|s| h > s + SAVE_RISE_M) {
                    self.saved = Some(h);
                    shared.data.lock().unwrap().vr_standing_m = Some(h);
                    shared.mark_dirty();
                }
            }
            *shared.vr_posture.lock().unwrap() = reading.map(|reading| crate::state::VrPosture {
                reading,
                trackers: self.trackers.0,
                waist: self.trackers.1.is_some(),
                hip_tracked: hip.is_some(),
                at_ms: now,
            });
        }
    }
}
