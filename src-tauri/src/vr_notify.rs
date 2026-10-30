//! Notifications in your headset (Settings > Notifications > In VR): every
//! notification the app shows also goes to XSOverlay and OVR Toolkit while
//! they run, or else to SteamVR's own notifications (the fallback, sent by the
//! SteamVR thread, see `steamvr`). Where it goes: `VrNotices::targets`.
//!
//! - XSOverlay: its UDP notification API on 127.0.0.1:42069 (JSON).
//! - OVR Toolkit: its WebSocket API at ws://127.0.0.1:11450/api.

use crate::state::Shared;
use base64::Engine as _;
use pregsys_core::model::{VrApps, VrTarget};
use serde_json::json;
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::time::{Duration, Instant};

const XSOVERLAY: &str = "127.0.0.1:42069";
const OVR_TOOLKIT_PORT: u16 = 11450;
const OVR_TOOLKIT_URL: &str = "ws://127.0.0.1:11450/api";
/// Shown with the notification in XSOverlay.
const ICON: &[u8] = include_bytes!("../icons/128x128.png");
/// SteamVR notices waiting longer than this (SteamVR not connected yet) are dropped.
pub const STEAMVR_QUEUE_MAX_AGE: Duration = Duration::from_secs(15);

/// OVR Toolkit's notification API is answering.
pub fn ovr_toolkit_listening() -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], OVR_TOOLKIT_PORT));
    TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok()
}

/// What's running right now (from the engine's process check).
fn running(shared: &Shared) -> VrApps {
    let st = shared.status.lock().unwrap();
    VrApps { xsoverlay: st.xsoverlay_running, ovr_toolkit: st.ovr_toolkit_running, steamvr: st.steamvr_running }
}

/// Where a notification would go right now.
pub fn targets(shared: &Shared) -> Vec<VrTarget> {
    let settings = shared.data.lock().unwrap().settings.vr_notices.clone();
    settings.targets(running(shared))
}

/// Shows a notification in your headset, wherever the settings say, and
/// returns where (empty: nowhere, so the desktop one should show). Never
/// blocks: overlay tools get it from a short-lived thread.
pub fn send(shared: &Shared, title: &str, body: &str) -> Vec<VrTarget> {
    let targets = targets(shared);
    for &target in &targets {
        let (title, body) = (title.to_string(), body.to_string());
        match target {
            VrTarget::Steamvr => queue_steamvr(shared, &title, &body),
            other => {
                let _ = std::thread::Builder::new().name("vr-notify".into()).spawn(move || {
                    if let Err(e) = send_to(other, &title, &body) {
                        log::warn!("VR notification ({other:?}): {e}");
                    }
                });
            }
        }
    }
    targets
}

/// Sends to one overlay tool now (SteamVR goes through the queue instead).
pub fn send_to(target: VrTarget, title: &str, body: &str) -> Result<(), String> {
    match target {
        VrTarget::Xsoverlay => xsoverlay(title, body),
        VrTarget::OvrToolkit => ovr_toolkit(title, body),
        VrTarget::Steamvr => Err("SteamVR notices go through the SteamVR thread".into()),
    }
}

/// Hands a notice to the SteamVR thread (it shows them while connected).
pub fn queue_steamvr(shared: &Shared, title: &str, body: &str) {
    let text = if body.is_empty() { title.to_string() } else { format!("{title}\n{body}") };
    let mut q = shared.vr_notice_queue.lock().unwrap();
    q.retain(|(_, at)| at.elapsed() < STEAMVR_QUEUE_MAX_AGE);
    q.push((text, Instant::now()));
}

fn xsoverlay(title: &str, body: &str) -> Result<(), String> {
    let msg = json!({
        "messageType": 1,
        "title": title,
        "content": body,
        "height": if body.is_empty() { 80.0 } else { 120.0 },
        "timeout": 5.0,
        "volume": 0.5,
        "audioPath": "default",
        "opacity": 1.0,
        "useBase64Icon": true,
        "icon": base64::engine::general_purpose::STANDARD.encode(ICON),
        "sourceApp": "Pregnancy OSC",
    });
    let socket = UdpSocket::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    socket.send_to(msg.to_string().as_bytes(), XSOVERLAY).map_err(|e| e.to_string())?;
    Ok(())
}

fn ovr_toolkit(title: &str, body: &str) -> Result<(), String> {
    let inner = json!({ "title": title, "body": body, "icon": null }).to_string();
    let msg = json!({ "messageType": "SendNotification", "json": inner }).to_string();
    let (mut ws, _) = tungstenite::connect(OVR_TOOLKIT_URL).map_err(|e| e.to_string())?;
    ws.send(tungstenite::Message::text(msg)).map_err(|e| e.to_string())?;
    let _ = ws.close(None);
    // Let the close finish (OVR Toolkit answers it), but don't wait long.
    if let tungstenite::stream::MaybeTlsStream::Plain(s) = ws.get_ref() {
        let _ = s.set_read_timeout(Some(Duration::from_millis(500)));
    }
    while ws.read().is_ok() {}
    Ok(())
}
