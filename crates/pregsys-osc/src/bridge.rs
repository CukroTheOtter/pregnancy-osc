//! One object that owns the OSC connection to VRChat: sending, receiving,
//! and OSCQuery discovery with a plain-port fallback.

use crate::osc::{Incoming, OscReceiver, OscSender, OscValue};
use crate::oscquery::{self, OscQueryService, VrcEndpoints};
use mdns_sd::ServiceDaemon;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::mpsc::TryRecvError;
use std::sync::{Arc, Mutex};

/// VRChat always sends here, whoever else it has found.
pub const VRCHAT_OUT_PORT: u16 = 9001;

#[derive(Debug, Clone)]
pub struct BridgeConfig {
    pub app_name: String,
    pub use_oscquery: bool,
    /// Fallback target when OSCQuery hasn't found VRChat.
    pub send_host: String,
    pub send_port: u16,
    /// Plain-OSC receive port (used when OSCQuery is off or fails). With
    /// OSCQuery, 0 means don't also listen on VRChat's port 9001.
    pub recv_port: u16,
    /// Addresses we want VRChat to send us (advertised over OSCQuery).
    pub listen_addresses: Vec<String>,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            app_name: "PregnancyOSC".into(),
            use_oscquery: true,
            send_host: "127.0.0.1".into(),
            send_port: 9000,
            recv_port: 9001,
            listen_addresses: vec!["/avatar/change".into()],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BridgeStatus {
    pub oscquery: bool,
    pub vrchat_found: bool,
    pub send_target: String,
    pub recv_port: u16,
    pub backup_port: Option<u16>,
    /// The app's own OSCQuery web port (what VRChat reads to find us).
    pub query_port: Option<u16>,
    /// VRChat's OSCQuery address, when found.
    pub vrchat_query: Option<String>,
    pub note: Option<String>,
}

pub struct Bridge {
    sender: OscSender,
    receiver: OscReceiver,
    /// VRChat's plain output port (9001), when free. VRChat sends there even
    /// when it hasn't found us over OSCQuery (it only looks at start-up or when
    /// OSC is toggled in its menu).
    backup: Option<OscReceiver>,
    fallback: SocketAddr,
    vrc: Option<Arc<Mutex<VrcEndpoints>>>,
    _service: Option<OscQueryService>,
    _daemon: Option<ServiceDaemon>,
    note: Option<String>,
}

fn resolve(host: &str, port: u16) -> SocketAddr {
    (host, port)
        .to_socket_addrs()
        .ok()
        .and_then(|mut a| a.next())
        .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], port)))
}

impl Bridge {
    pub fn start(cfg: &BridgeConfig) -> Result<Self, String> {
        let fallback = resolve(&cfg.send_host, cfg.send_port);
        let sender = OscSender::new(fallback).map_err(|e| format!("cannot open OSC send socket: {e}"))?;
        let mut note = None;

        if cfg.use_oscquery {
            match Self::start_oscquery(cfg) {
                Ok((receiver, service, daemon, vrc)) => {
                    // Always VRChat's own port, not the fixed-mode listen port.
                    let backup = (cfg.recv_port != 0).then(|| OscReceiver::bind("127.0.0.1", VRCHAT_OUT_PORT).ok()).flatten();
                    return Ok(Self { sender, receiver, backup, fallback, vrc: Some(vrc), _service: Some(service), _daemon: Some(daemon), note: None });
                }
                Err(e) => note = Some(format!("OSCQuery unavailable ({e}); using plain ports.")),
            }
        }
        let receiver = OscReceiver::bind("127.0.0.1", cfg.recv_port)
            .map_err(|e| format!("cannot listen on port {} (another OSC app may be using it): {e}", cfg.recv_port))?;
        // Fixed ports (e.g. through an OSC router): still find VRChat over
        // OSCQuery, but only to read the avatar. Sending stays on the fixed port.
        let (vrc, daemon) = match ServiceDaemon::new() {
            Ok(d) => (oscquery::watch_vrchat(&d).ok(), Some(d)),
            Err(_) => (None, None),
        };
        Ok(Self { sender, receiver, backup: None, fallback, vrc, _service: None, _daemon: daemon, note })
    }

    #[allow(clippy::type_complexity)]
    fn start_oscquery(cfg: &BridgeConfig) -> Result<(OscReceiver, OscQueryService, ServiceDaemon, Arc<Mutex<VrcEndpoints>>), String> {
        let daemon = ServiceDaemon::new().map_err(|e| e.to_string())?;
        let receiver = OscReceiver::bind("127.0.0.1", 0).map_err(|e| e.to_string())?;
        let service = OscQueryService::start(&daemon, &cfg.app_name, receiver.port, cfg.listen_addresses.clone())?;
        let vrc = oscquery::watch_vrchat(&daemon)?;
        Ok((receiver, service, daemon, vrc))
    }

    /// Changes the addresses advertised to VRChat, without restarting anything.
    pub fn set_listen_addresses(&self, addresses: &[String]) {
        if let Some(svc) = &self._service {
            svc.set_addresses(addresses);
        }
    }

    fn endpoints(&self) -> VrcEndpoints {
        self.vrc.as_ref().map(|v| v.lock().unwrap().clone()).unwrap_or_default()
    }

    /// Points the sender at VRChat if OSCQuery found it, else the fallback.
    /// With fixed ports the fallback always wins (it may be an OSC router).
    pub fn refresh_target(&self) {
        let t = if self._service.is_some() { self.endpoints().osc.unwrap_or(self.fallback) } else { self.fallback };
        if self.sender.target() != t {
            self.sender.set_target(t);
        }
    }

    pub fn send(&self, addr: &str, value: OscValue) {
        // UDP to a closed port is not an error worth reporting.
        let _ = self.sender.send(addr, &value);
    }

    /// Shows text in VRChat's chatbox right away (not in the typing box).
    pub fn chatbox(&self, text: &str, sound: bool) {
        let args = [OscValue::Str(text.to_string()), OscValue::Bool(true), OscValue::Bool(sound)];
        let _ = self.sender.send_args("/chatbox/input", &args);
    }

    /// Everything received since the last call.
    pub fn drain(&self) -> Result<Vec<Incoming>, String> {
        let mut out = Vec::new();
        loop {
            match self.receiver.rx.try_recv() {
                Ok(m) => out.push(m),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return Err("OSC receiver stopped".into()),
            }
        }
        if let Some(b) = &self.backup {
            while let Ok(m) = b.rx.try_recv() {
                out.push(m);
            }
        }
        Ok(out)
    }

    /// The avatar you're wearing and its parameters, via OSCQuery, if VRChat was found.
    pub fn avatar_info(&self) -> Option<oscquery::AvatarInfo> {
        let q = self.endpoints().query?;
        oscquery::fetch_avatar(q).ok()
    }

    pub fn status(&self) -> BridgeStatus {
        let ep = self.endpoints();
        BridgeStatus {
            oscquery: self._service.is_some(),
            vrchat_found: ep.osc.is_some() || ep.query.is_some(),
            send_target: self.sender.target().to_string(),
            recv_port: self.receiver.port,
            backup_port: self.backup.as_ref().map(|b| b.port),
            query_port: self._service.as_ref().map(|s| s.http_port),
            vrchat_query: ep.query.map(|a| a.to_string()),
            note: self.note.clone(),
        }
    }
}
