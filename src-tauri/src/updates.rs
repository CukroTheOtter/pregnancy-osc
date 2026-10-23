//! Updates from the GitHub releases (signed, checked by tauri-plugin-updater).

use crate::state::{EngineCmd, Shared};
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

/// The update found by the last check, ready to install.
#[derive(Default)]
pub struct Pending(Mutex<Option<Update>>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    version: String,
    notes: Option<String>,
}

#[tauri::command]
pub fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub async fn check_update(app: AppHandle, pending: State<'_, Pending>) -> Result<Option<UpdateInfo>, String> {
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| format!("Could not check for updates: {e}"))?;
    let info = update.as_ref().map(|u| UpdateInfo { version: u.version.clone(), notes: u.body.clone() });
    *pending.0.lock().unwrap() = update;
    Ok(info)
}

#[tauri::command]
pub async fn install_update(app: AppHandle, pending: State<'_, Pending>) -> Result<(), String> {
    let update = pending.0.lock().unwrap().take().ok_or("No update to install. Check again.")?;
    let bytes = update
        .download(|_, _| {}, || {})
        .await
        .map_err(|e| format!("Could not download the update: {e}"))?;

    // The installer closes the app, so hand the avatar back and save first.
    let shared = app.state::<Arc<Shared>>().inner().clone();
    shared.quitting.store(true, Ordering::SeqCst);
    let (tx, rx) = mpsc::channel();
    shared.send(EngineCmd::Shutdown(tx));
    let _ = rx.recv_timeout(Duration::from_secs(3));
    let _ = shared.save_if_dirty();

    if let Err(e) = update.install(bytes) {
        log::error!("Could not install the update: {e}");
    }
    // Windows exits inside install(); elsewhere (or if it failed) start again.
    app.restart();
}
