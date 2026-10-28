//! Reads where your headset and waist tracker are from SteamVR, for
//! "Resting holds off labor" (see `pregsys_core::posture`). Read only: the
//! app connects as a background SteamVR app, only while SteamVR runs and that
//! setting is on, and never starts SteamVR or keeps it running.
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
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tauri::{AppHandle, Manager};
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    // From openvr_capi.h (OpenVR 2.5.1).
    /// VRApplication_Background: doesn't start SteamVR, doesn't keep it running.
    const APP_BACKGROUND: i32 = 3;
    const SYSTEM_VERSION: &[u8] = b"FnTable:IVRSystem_022\0";
    const SETTINGS_VERSION: &[u8] = b"FnTable:IVRSettings_003\0";
    /// Raw tracking space: the room as set up, before any playspace mover.
    const UNIVERSE_RAW: i32 = 2;
    const MAX_DEVICES: usize = 64;
    const CLASS_HMD: i32 = 1;
    const CLASS_TRACKER: i32 = 3;
    const PROP_REGISTERED_TYPE: i32 = 1036;
    const PROP_CONTROLLER_TYPE: i32 = 7000;
    const EV_QUIT: u32 = 700;
    // Function table slots (counted from the header's FnTable structs).
    const SYS_POSES: usize = 11;
    const SYS_DEVICE_CLASS: usize = 19;
    const SYS_STRING_PROP: usize = 27;
    const SYS_POLL_NEXT_EVENT: usize = 29;
    const SYS_ACK_QUIT: usize = 43;
    const SET_GET_STRING: usize = 8;

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
    /// Reads the posture this often.
    const POSE_EVERY: Duration = Duration::from_millis(250);
    /// Looks for the waist tracker (roles can change) this often.
    const TRACKERS_EVERY: Duration = Duration::from_secs(5);

    type InitFn = unsafe extern "system" fn(*mut i32, i32) -> isize;
    type ShutdownFn = unsafe extern "system" fn();
    type GetIfaceFn = unsafe extern "system" fn(*const u8, *mut i32) -> isize;

    struct Api {
        shutdown: ShutdownFn,
        sys: *const usize,
        settings: *const usize,
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

        /// Every device's pose in raw tracking space.
        fn poses(&self) -> Vec<Pose> {
            let f: unsafe extern "system" fn(i32, f32, *mut Pose, u32) = unsafe { Self::f(self.sys, SYS_POSES) };
            let mut out: Vec<Pose> = vec![unsafe { std::mem::zeroed() }; MAX_DEVICES];
            unsafe { f(UNIVERSE_RAW, 0.0, out.as_mut_ptr(), MAX_DEVICES as u32) };
            out
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
    }

    fn c_string(buf: &[u8]) -> String {
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        String::from_utf8_lossy(&buf[..end]).to_string()
    }

    /// A pose's position (x, y, z, y up), if it's tracked right now.
    fn position(p: &Pose) -> Option<[f32; 3]> {
        (p.valid && p.connected).then_some([p.m[0][3], p.m[1][3], p.m[2][3]])
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Loads OpenVR and connects as a background app (only while SteamVR runs).
    fn connect(app: &AppHandle) -> Result<Api, String> {
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
        unsafe { init(&mut err, APP_BACKGROUND) };
        if err != 0 {
            return Err(format!("SteamVR said no (error {err})"));
        }
        let sys = unsafe { iface(SYSTEM_VERSION.as_ptr(), &mut err) } as *const usize;
        let settings = unsafe { iface(SETTINGS_VERSION.as_ptr(), &mut err) } as *const usize;
        if sys.is_null() || settings.is_null() || err != 0 {
            unsafe { shutdown() };
            return Err(format!("SteamVR is too old (error {err})"));
        }
        Ok(Api { shutdown, sys, settings })
    }

    pub fn spawn(app: AppHandle, shared: Arc<Shared>) {
        let _ = std::thread::Builder::new().name("steamvr".into()).spawn(move || run(app, shared));
    }

    /// SteamVR is running and "Resting holds off labor" is on.
    fn wanted(shared: &Shared) -> bool {
        let on = shared.data.lock().unwrap().settings.labor.resting_holds;
        on && shared.status.lock().unwrap().steamvr_running
    }

    fn run(app: AppHandle, shared: Arc<Shared>) {
        loop {
            std::thread::sleep(RETRY);
            if !wanted(&shared) {
                continue;
            }
            let api = match connect(&app) {
                Ok(a) => a,
                Err(e) => {
                    log::warn!("SteamVR: {e}");
                    continue;
                }
            };
            session(&shared, &api);
            *shared.vr_posture.lock().unwrap() = None;
            unsafe { (api.shutdown)() };
        }
    }

    /// Connected: reads your posture until SteamVR quits or it's no longer wanted.
    fn session(shared: &Shared, api: &Api) {
        let mut posture = pregsys_core::posture::VrPosture::default();
        let mut last_check = Instant::now();
        let mut last_trackers = Instant::now() - TRACKERS_EVERY;
        let mut trackers = (0u32, None::<usize>);
        loop {
            while let Some(e) = api.poll_system() {
                if e.kind == EV_QUIT {
                    api.ack_quit();
                    return;
                }
            }
            if last_check.elapsed() >= Duration::from_secs(1) {
                last_check = Instant::now();
                if !wanted(shared) {
                    return;
                }
            }
            if last_trackers.elapsed() >= TRACKERS_EVERY {
                last_trackers = Instant::now();
                trackers = api.trackers();
            }
            let poses = api.poses();
            let head = (0..MAX_DEVICES).find(|&i| api.class(i) == CLASS_HMD).and_then(|i| position(&poses[i]));
            let hip = trackers.1.and_then(|i| position(&poses[i]));
            let now = pregsys_core::now_ms();
            *shared.vr_posture.lock().unwrap() = head.map(|h| crate::state::VrPosture {
                reading: posture.update(h, hip, now),
                trackers: trackers.0,
                waist: trackers.1.is_some(),
                hip_tracked: hip.is_some(),
                at_ms: now,
            });
            std::thread::sleep(POSE_EVERY);
        }
    }
}
