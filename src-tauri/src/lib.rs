//! Pregnancy OSC desktop app.

mod commands;
mod engine;
mod pictures;
mod state;
mod system;
mod updates;

use pregsys_core::mapping::Mapping;
use pregsys_core::storage::Store;
use state::{EngineCmd, Shared, Status};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, RunEvent, WindowEvent};

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Hands the avatar back, saves, then exits.
pub fn request_quit(app: &AppHandle) {
    let shared = app.state::<Arc<Shared>>().inner().clone();
    if shared.quitting.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel();
        shared.send(EngineCmd::Shutdown(tx));
        let _ = rx.recv_timeout(Duration::from_secs(3));
        let _ = shared.save_if_dirty();
        app.exit(0);
    });
}

pub fn notify(app: &AppHandle, title: &str, body: &str) {
    let on = app.state::<Arc<Shared>>().data.lock().unwrap().settings.notifications;
    if on {
        use tauri_plugin_notification::NotificationExt;
        let _ = app.notification().builder().title(title).body(body).show();
    }
}

pub fn set_os_autostart(app: &AppHandle, enable: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    if enable { al.enable() } else { al.disable() }.map_err(|e| format!("Could not change start with OS: {e}"))
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Pregnancy OSC", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let mut b = TrayIconBuilder::with_id("main")
        .tooltip("Pregnancy OSC")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "open" => show_main(app),
            "quit" => request_quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        b = b.icon(icon.clone());
    }
    b.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    let tray_arg = args.iter().any(|a| a == "--tray");
    let autostart_arg = args.iter().any(|a| a == "--autostart");

    let app = tauri::Builder::default()
        // Must be first: a second launch just focuses the running window.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--autostart"])))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(updates::Pending::default())
        .setup(move |app| {
            let handle = app.handle().clone();
            // App dirs follow the OS (XDG on Linux, Flatpak-aware).
            let data_dir = app.path().app_data_dir()?;
            let config_dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let mapping_path = config_dir.join("osc-parameters.json");
            let mapping = Mapping::load_or_create(&mapping_path).unwrap_or_default();

            let store = Store::new(&data_dir);
            let (data, warning) = match store.load() {
                Ok(x) => x,
                Err(e) => return Err(e.to_string().into()),
            };
            let mut data = data;
            pregsys_core::actions::ensure_cycle(&mut data, pregsys_core::now_ms());
            pregsys_core::actions::ensure_all_genders(&mut data);
            let today = chrono_today();
            let _ = store.daily_backup(&today);

            let shared = Arc::new(Shared {
                data: Mutex::new(data),
                store,
                data_dir: data_dir.clone(),
                mapping_path: mapping_path.clone(),
                mapping: Mutex::new(mapping),
                status: Mutex::new(Status {
                    mapping_path: mapping_path.to_string_lossy().to_string(),
                    data_dir: data_dir.to_string_lossy().to_string(),
                    ..Default::default()
                }),
                engine: Mutex::new(None),
                dirty: AtomicBool::new(false),
                load_warning: Mutex::new(warning),
                tray_available: AtomicBool::new(false),
                quitting: AtomicBool::new(false),
            });
            app.manage(shared.clone());

            // Some Linux desktops have no tray; the app must still work.
            let tray_ok = build_tray(&handle).is_ok();
            shared.tray_available.store(tray_ok, Ordering::SeqCst);

            engine::spawn(handle.clone(), shared.clone());

            let start_in_tray = shared.data.lock().unwrap().settings.start_in_tray;
            let hide = tray_arg || (autostart_arg && start_in_tray);
            if let Some(w) = app.get_webview_window("main") {
                if !hide {
                    let _ = w.show();
                } else if !tray_ok {
                    // No tray to hide in: start minimized instead.
                    let _ = w.show();
                    let _ = w.minimize();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let shared = app.state::<Arc<Shared>>();
                if shared.tray_available.load(Ordering::SeqCst) && !shared.quitting.load(Ordering::SeqCst) {
                    // Keep running in the tray.
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    api.prevent_close();
                    request_quit(app);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_data,
            commands::get_status,
            commands::take_load_warning,
            commands::save_settings,
            commands::create_pregnancy,
            commands::save_pregnancy,
            commands::end_pregnancy,
            commands::delete_pregnancy,
            commands::set_current_pregnancy,
            commands::set_partner,
            commands::save_avatar,
            commands::delete_avatar,
            commands::create_group,
            commands::set_avatar_group,
            commands::rename_group,
            commands::delete_group,
            commands::set_species_answer,
            commands::forget_person,
            commands::import_community_table,
            commands::save_journal_entry,
            commands::delete_journal_entry,
            commands::add_keepsake,
            commands::pulse,
            commands::export_backup,
            commands::import_backup,
            commands::reset_data,
            commands::get_mapping,
            commands::save_mapping,
            commands::reset_mapping,
            commands::reload_mapping,
            commands::open_path,
            commands::set_sync_value,
            commands::open_system_page,
            commands::default_log_dirs,
            commands::write_file,
            commands::partner_picture,
            commands::avatar_preview,
            commands::add_avatar,
            commands::dismiss_reveal,
            commands::debug_woohoo,
            commands::debug_reveal_now,
            commands::debug_set_cycle_day,
            commands::debug_add_person,
            commands::debug_remove_person,
            commands::debug_picture,
            commands::debug_kick,
            commands::give_birth,
            commands::restart_cycle,
            commands::quit_app,
            updates::app_version,
            updates::check_update,
            updates::install_update,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Pregnancy OSC");

    app.run(|app, event| {
        if let RunEvent::ExitRequested { api, .. } = event {
            let shared = app.state::<Arc<Shared>>();
            if !shared.quitting.load(Ordering::SeqCst) {
                api.prevent_exit();
                request_quit(app);
            }
        }
    });
}

/// Local date as YYYY-MM-DD for backup names.
fn chrono_today() -> String {
    let secs = pregsys_core::now_ms() / 1000;
    let days = secs.div_euclid(86_400);
    // Civil-from-days (UTC is fine for a backup file name).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", if m <= 2 { y + 1 } else { y }, m, d)
}
