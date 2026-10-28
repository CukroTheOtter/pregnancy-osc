//! The app's tab in the SteamVR dashboard (Settings > SteamVR > "Show in the
//! SteamVR dashboard"). The tab shows the app window itself: it's copied into
//! the overlay while the tab is open, and the laser's clicks and scrolls are
//! passed back to the window. While in VR the window can stay parked off-screen
//! (it has to keep drawing to be copied, so it's never minimized).
//!
//! Uses Valve's `openvr_api.dll` (shipped next to the app) through its C
//! function tables, so there's nothing to build. Windows only.

#[cfg(not(windows))]
pub fn spawn(_app: tauri::AppHandle, _shared: std::sync::Arc<crate::state::Shared>) {}

#[cfg(not(windows))]
pub fn unpark(_app: &tauri::AppHandle) {}

#[cfg(windows)]
pub use win::{spawn, unpark};

#[cfg(windows)]
mod win {
    use crate::state::Shared;
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    use tauri::{AppHandle, Manager, PhysicalPosition};
    use windows_sys::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        ClientToScreen, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
        DIB_RGB_COLORS,
    };
    use windows_sys::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumChildWindows, GetClassNameW, GetClientRect, GetWindowRect, PostMessageW, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    };

    // From openvr_capi.h (OpenVR 2.5.1).
    const APP_OVERLAY: i32 = 2;
    const OVERLAY_VERSION: &[u8] = b"FnTable:IVROverlay_027\0";
    const SYSTEM_VERSION: &[u8] = b"FnTable:IVRSystem_022\0";
    const EV_MOUSE_MOVE: u32 = 300;
    const EV_MOUSE_DOWN: u32 = 301;
    const EV_MOUSE_UP: u32 = 302;
    const EV_SCROLL: u32 = 305;
    const EV_SHOWN: u32 = 500;
    const EV_HIDDEN: u32 = 501;
    const EV_QUIT: u32 = 700;
    const MOUSE_LEFT: u32 = 1;
    /// Windows' "left button down" flag for mouse messages.
    const MK_LBUTTON: u32 = 1;
    const INPUT_MOUSE: i32 = 1;
    const FLAG_DISCRETE_SCROLL: i32 = 64;
    // Function table slots (counted from the header's FnTable structs).
    const SYS_POLL_NEXT_EVENT: usize = 29;
    const SYS_ACK_QUIT: usize = 43;
    const OV_DESTROY: usize = 2;
    const OV_SET_FLAG: usize = 10;
    const OV_SET_WIDTH: usize = 21;
    const OV_POLL_EVENT: usize = 46;
    const OV_SET_INPUT: usize = 48;
    const OV_SET_MOUSE_SCALE: usize = 50;
    const OV_SET_RAW: usize = 60;
    const OV_SET_FROM_FILE: usize = 61;
    const OV_CREATE_DASHBOARD: usize = 65;

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

    impl VrEvent {
        fn floats(&self) -> (f32, f32, u32) {
            // VREvent_Mouse_t / VREvent_Scroll_t: two floats then a u32.
            let b: [u8; 48] = unsafe { std::mem::transmute(self.data) };
            let f = |i: usize| f32::from_le_bytes(b[i..i + 4].try_into().unwrap());
            (f(0), f(4), u32::from_le_bytes(b[8..12].try_into().unwrap()))
        }
    }

    /// Captures at most this often while the tab is open.
    const FRAME: Duration = Duration::from_millis(66);
    /// Looks for SteamVR this often.
    const RETRY: Duration = Duration::from_secs(5);
    /// Parked window position (off every screen).
    const PARK: PhysicalPosition<i32> = PhysicalPosition { x: -20_000, y: -20_000 };

    /// The window while it's parked for VR: where it was and whether it showed.
    struct Parked {
        pos: PhysicalPosition<i32>,
        visible: bool,
        minimized: bool,
    }

    static PARKED: Mutex<Option<Parked>> = Mutex::new(None);
    /// The user brought the window back from the tray while in VR: don't park it again this session.
    static LET_BE: AtomicBool = AtomicBool::new(false);

    type InitFn = unsafe extern "system" fn(*mut i32, i32) -> isize;
    type ShutdownFn = unsafe extern "system" fn();
    type GetIfaceFn = unsafe extern "system" fn(*const u8, *mut i32) -> isize;

    struct Api {
        shutdown: ShutdownFn,
        sys: *const usize,
        ov: *const usize,
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

        fn poll_overlay(&self, h: u64) -> Option<VrEvent> {
            let f: unsafe extern "system" fn(u64, *mut VrEvent, u32) -> bool = unsafe { Self::f(self.ov, OV_POLL_EVENT) };
            let mut e: VrEvent = unsafe { std::mem::zeroed() };
            unsafe { f(h, &mut e, std::mem::size_of::<VrEvent>() as u32) }.then_some(e)
        }

        fn ack_quit(&self) {
            let f: unsafe extern "system" fn() = unsafe { Self::f(self.sys, SYS_ACK_QUIT) };
            unsafe { f() }
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Loads OpenVR and starts as an overlay app. Only called while SteamVR runs
    /// (starting OpenVR could otherwise launch it).
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
        unsafe { init(&mut err, APP_OVERLAY) };
        if err != 0 {
            return Err(format!("SteamVR said no (error {err})"));
        }
        let sys = unsafe { iface(SYSTEM_VERSION.as_ptr(), &mut err) } as *const usize;
        let ov = unsafe { iface(OVERLAY_VERSION.as_ptr(), &mut err) } as *const usize;
        if sys.is_null() || ov.is_null() || err != 0 {
            unsafe { shutdown() };
            return Err(format!("SteamVR is too old (error {err})"));
        }
        Ok(Api { shutdown, sys, ov })
    }

    /// The dashboard tab and its icon.
    fn create_tab(api: &Api, icon: &std::path::Path) -> Result<u64, String> {
        let create: unsafe extern "system" fn(*const u8, *const u8, *mut u64, *mut u64) -> i32 = unsafe { Api::f(api.ov, OV_CREATE_DASHBOARD) };
        let (mut main, mut thumb) = (0u64, 0u64);
        let e = unsafe { create(b"pregnancyosc.dashboard\0".as_ptr(), b"Pregnancy OSC\0".as_ptr(), &mut main, &mut thumb) };
        if e != 0 {
            return Err(format!("couldn't add the dashboard tab (error {e})"));
        }
        unsafe {
            let from_file: unsafe extern "system" fn(u64, *const u8) -> i32 = Api::f(api.ov, OV_SET_FROM_FILE);
            let path = format!("{}\0", icon.to_string_lossy());
            from_file(thumb, path.as_ptr());
            let width: unsafe extern "system" fn(u64, f32) -> i32 = Api::f(api.ov, OV_SET_WIDTH);
            width(main, 1.6);
            let input: unsafe extern "system" fn(u64, i32) -> i32 = Api::f(api.ov, OV_SET_INPUT);
            input(main, INPUT_MOUSE);
            let flag: unsafe extern "system" fn(u64, i32, bool) -> i32 = Api::f(api.ov, OV_SET_FLAG);
            flag(main, FLAG_DISCRETE_SCROLL, true);
        }
        Ok(main)
    }

    fn hwnd(app: &AppHandle) -> Option<HWND> {
        app.get_webview_window("main")?.hwnd().ok().map(|h| h.0 as HWND)
    }

    /// The window's client area as RGBA, with its size.
    fn capture(hwnd: HWND) -> Option<(Vec<u8>, u32, u32)> {
        unsafe {
            let mut r: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut r);
            let (w, h) = ((r.right - r.left).max(0), (r.bottom - r.top).max(0));
            if w == 0 || h == 0 {
                return None;
            }
            let screen = GetDC(std::ptr::null_mut());
            let dc = CreateCompatibleDC(screen);
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..std::mem::zeroed()
            };
            let mut bits: *mut c_void = std::ptr::null_mut();
            let bmp = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
            let old = SelectObject(dc, bmp);
            // Client area only, with the web view's own drawing.
            const PW_CLIENTONLY_FULL: PRINT_WINDOW_FLAGS = 1 | 2;
            let ok = PrintWindow(hwnd, dc, PW_CLIENTONLY_FULL) != 0 && !bits.is_null();
            let mut out = Vec::new();
            if ok {
                let src = std::slice::from_raw_parts(bits as *const u8, (w * h * 4) as usize);
                out.reserve_exact(src.len());
                for px in src.chunks_exact(4) {
                    out.extend_from_slice(&[px[2], px[1], px[0], 255]); // BGRA -> RGBA
                }
            }
            SelectObject(dc, old);
            DeleteObject(bmp);
            DeleteDC(dc);
            ReleaseDC(std::ptr::null_mut(), screen);
            ok.then_some((out, w as u32, h as u32))
        }
    }

    /// The web view's own window (it takes the mouse), and where it sits in the client area.
    fn input_target(hwnd: HWND) -> Option<(HWND, i32, i32)> {
        unsafe extern "system" fn find(child: HWND, out: LPARAM) -> i32 {
            let mut name = [0u16; 64];
            let n = GetClassNameW(child, name.as_mut_ptr(), name.len() as i32) as usize;
            if String::from_utf16_lossy(&name[..n]) == "Chrome_RenderWidgetHostHWND" {
                *(out as *mut HWND) = child;
                return 0;
            }
            1
        }
        let mut target: HWND = std::ptr::null_mut();
        unsafe { EnumChildWindows(hwnd, Some(find), &mut target as *mut HWND as LPARAM) };
        if target.is_null() {
            return None;
        }
        unsafe {
            let mut origin = POINT { x: 0, y: 0 };
            ClientToScreen(hwnd, &mut origin);
            let mut r: RECT = std::mem::zeroed();
            GetWindowRect(target, &mut r);
            Some((target, r.left - origin.x, r.top - origin.y))
        }
    }

    fn lparam(x: i32, y: i32) -> LPARAM {
        ((y as u16 as u32) << 16 | x as u16 as u32) as LPARAM
    }

    /// Passes a laser event on to the web view. `size` is the captured client size.
    fn forward(hwnd: HWND, e: &VrEvent, size: (u32, u32), down: &mut bool) {
        let Some((target, ox, oy)) = input_target(hwnd) else { return };
        let (fx, fy, button) = e.floats();
        // Overlay mouse coordinates start at the bottom left.
        let x = fx.round() as i32 - ox;
        let y = (size.1 as f32 - fy).round() as i32 - oy;
        unsafe {
            match e.kind {
                EV_MOUSE_MOVE => {
                    PostMessageW(target, WM_MOUSEMOVE, if *down { MK_LBUTTON as usize } else { 0 }, lparam(x, y));
                }
                EV_MOUSE_DOWN if button == MOUSE_LEFT => {
                    *down = true;
                    PostMessageW(target, WM_MOUSEMOVE, 0, lparam(x, y));
                    PostMessageW(target, WM_LBUTTONDOWN, MK_LBUTTON as usize, lparam(x, y));
                }
                EV_MOUSE_UP if button == MOUSE_LEFT => {
                    *down = false;
                    PostMessageW(target, WM_LBUTTONUP, 0, lparam(x, y));
                }
                EV_SCROLL => {
                    // Wheel messages use screen coordinates.
                    let mut r: RECT = std::mem::zeroed();
                    GetWindowRect(target, &mut r);
                    let delta = (fy * 120.0).round() as i16;
                    let wparam = ((delta as u16 as usize) << 16) as usize;
                    PostMessageW(target, WM_MOUSEWHEEL, wparam, lparam(r.left + 20, r.top + 20));
                }
                _ => {}
            }
        }
    }

    /// Moves the window off-screen (still drawing, so it can be copied) and off the taskbar.
    fn park(app: &AppHandle) {
        let Some(w) = app.get_webview_window("main") else { return };
        let mut parked = PARKED.lock().unwrap();
        if parked.is_some() || LET_BE.load(Ordering::SeqCst) {
            return;
        }
        *parked = Some(Parked {
            pos: w.outer_position().unwrap_or(PhysicalPosition { x: 100, y: 100 }),
            visible: w.is_visible().unwrap_or(false),
            minimized: w.is_minimized().unwrap_or(false),
        });
        let _ = w.set_skip_taskbar(true);
        let _ = w.set_position(PARK);
        let _ = w.unminimize();
        let _ = w.show();
    }

    /// The user opened the window from the tray: put it back, and leave it
    /// alone for the rest of this VR session.
    pub fn unpark(app: &AppHandle) {
        if PARKED.lock().unwrap().is_some() {
            LET_BE.store(true, Ordering::SeqCst);
            restore(app);
        }
    }

    /// Puts the window back the way it was.
    fn restore(app: &AppHandle) {
        let Some(p) = PARKED.lock().unwrap().take() else { return };
        let Some(w) = app.get_webview_window("main") else { return };
        let _ = w.set_position(p.pos);
        let _ = w.set_skip_taskbar(false);
        if !p.visible {
            let _ = w.hide();
        } else if p.minimized {
            let _ = w.minimize();
        }
    }

    fn set_status(shared: &Shared, on: bool) {
        shared.status.lock().unwrap().steamvr_dashboard = on;
    }

    pub fn spawn(app: AppHandle, shared: Arc<Shared>) {
        let _ = std::thread::Builder::new().name("steamvr-overlay".into()).spawn(move || run(app, shared));
    }

    fn wanted(shared: &Shared) -> bool {
        let on = shared.data.lock().unwrap().settings.steamvr_dashboard;
        on && shared.status.lock().unwrap().steamvr_running
    }

    fn run(app: AppHandle, shared: Arc<Shared>) {
        // The tab's icon, written once where SteamVR can read it.
        let icon = shared.data_dir.join("steamvr-icon.png");
        let _ = std::fs::write(&icon, include_bytes!("../icons/128x128.png"));
        loop {
            std::thread::sleep(RETRY);
            if !wanted(&shared) {
                continue;
            }
            let api = match connect(&app) {
                Ok(a) => a,
                Err(e) => {
                    log::warn!("SteamVR dashboard: {e}");
                    continue;
                }
            };
            match create_tab(&api, &icon) {
                Ok(main) => session(&app, &shared, &api, main),
                Err(e) => log::warn!("SteamVR dashboard: {e}"),
            }
            set_status(&shared, false);
            unsafe { (api.shutdown)() };
            restore(&app);
            LET_BE.store(false, Ordering::SeqCst);
        }
    }

    /// The tab is up: copy the window while it's open, pass clicks back, and
    /// stop when SteamVR quits or the setting is turned off.
    fn session(app: &AppHandle, shared: &Shared, api: &Api, main: u64) {
        set_status(shared, true);
        let set_raw: unsafe extern "system" fn(u64, *const u8, u32, u32, u32) -> i32 = unsafe { Api::f(api.ov, OV_SET_RAW) };
        let set_scale: unsafe extern "system" fn(u64, *const [f32; 2]) -> i32 = unsafe { Api::f(api.ov, OV_SET_MOUSE_SCALE) };
        let destroy: unsafe extern "system" fn(u64) -> i32 = unsafe { Api::f(api.ov, OV_DESTROY) };
        let mut open = false;
        let mut size = (0u32, 0u32);
        let mut down = false;
        let mut last_frame = Instant::now() - FRAME;
        let mut last_check = Instant::now() - Duration::from_secs(1);
        let mut hide_in_vr = false;
        loop {
            while let Some(e) = api.poll_system() {
                if e.kind == EV_QUIT {
                    api.ack_quit();
                    return;
                }
            }
            if last_check.elapsed() >= Duration::from_secs(1) {
                last_check = Instant::now();
                let s = shared.data.lock().unwrap().settings.clone();
                if !s.steamvr_dashboard || !shared.status.lock().unwrap().steamvr_running {
                    unsafe { destroy(main) };
                    return;
                }
                hide_in_vr = s.hide_window_in_vr;
            }
            // Parked while in VR (if that's on), and whenever the tab is open but the
            // window isn't showing (a hidden window can't be copied).
            let shown = app.get_webview_window("main").is_some_and(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false));
            let parked = PARKED.lock().unwrap().is_some();
            if hide_in_vr || (open && !shown) {
                park(app);
            } else if parked && !open {
                restore(app);
            }
            let Some(hwnd) = hwnd(app) else { return };
            while let Some(e) = api.poll_overlay(main) {
                match e.kind {
                    EV_SHOWN => open = true,
                    EV_HIDDEN => open = false,
                    EV_MOUSE_MOVE | EV_MOUSE_DOWN | EV_MOUSE_UP | EV_SCROLL if size.0 > 0 => forward(hwnd, &e, size, &mut down),
                    _ => {}
                }
            }
            if open && last_frame.elapsed() >= FRAME {
                last_frame = Instant::now();
                if let Some((px, w, h)) = capture(hwnd) {
                    if (w, h) != size {
                        size = (w, h);
                        unsafe { set_scale(main, &[w as f32, h as f32]) };
                    }
                    unsafe { set_raw(main, px.as_ptr(), w, h, 4) };
                }
            }
            std::thread::sleep(Duration::from_millis(if open { 11 } else { 50 }));
        }
    }
}
