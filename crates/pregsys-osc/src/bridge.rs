//! One object that owns the OSC connection to VRChat: sending, receiving,
//! and OSCQuery discovery with a plain-port fallback.

use crate::osc::{Incoming, OscReceiver, OscSender, OscValue};
use crate::oscquery::{self, OscQueryService, VrcEndpoints, VrcWatch};
use mdns_sd::ServiceDaemon;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::mpsc::TryRecvError;
use std::sync::{Arc, Mutex};

/// VRChat always sends here, whoever else it has found.
pub const VRCHAT_OUT_PORT: u16 = 9001;
/// VRChat's usual input port, for when OSCQuery hasn't found it.
pub const VRCHAT_IN_PORT: u16 = 9000;
/// Our OSCQuery port (web and OSC on the same number), the first free one
/// from here. The same port every run, with the same name, means VRChat
/// sees the same app after a restart (OGB does this too, from 33776).
pub const OSCQUERY_PORT: u16 = 33790;
const OSCQUERY_PORT_TRIES: u16 = 40;

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
            app_name: "PregnancyOSC".into(), // one app, one name: never per process
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
    /// VRChat's OSC looks on: its OSCQuery answers (or, with no mDNS at all,
    /// there's no way to tell, so yes).
    pub linked: bool,
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
    vrc: Option<Arc<Mutex<VrcWatch>>>,
    _service: Option<OscQueryService>,
    _daemon: Option<ServiceDaemon>,
    note: Option<String>,
}

impl Drop for Bridge {
    fn drop(&mut self) {
        // The mDNS daemon runs its own thread until told to stop.
        if let Some(d) = &self._daemon {
            let _ = d.shutdown();
        }
    }
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
        // Automatic mode: VRChat's own port, not the (hidden) fixed send port.
        let fallback = if cfg.use_oscquery { SocketAddr::from(([127, 0, 0, 1], VRCHAT_IN_PORT)) } else { resolve(&cfg.send_host, cfg.send_port) };
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
    fn start_oscquery(cfg: &BridgeConfig) -> Result<(OscReceiver, OscQueryService, ServiceDaemon, Arc<Mutex<VrcWatch>>), String> {
        let daemon = ServiceDaemon::new().map_err(|e| e.to_string())?;
        let (receiver, server) = Self::bind_oscquery_port()?;
        // VRChat remembers an app's port by its name: on any other port, use another name.
        let name = if receiver.port == OSCQUERY_PORT { cfg.app_name.clone() } else { format!("{}-{}", cfg.app_name, receiver.port) };
        let service = OscQueryService::start(server, &name, receiver.port, cfg.listen_addresses.clone())?;
        let vrc = match oscquery::watch_vrchat(&daemon) {
            Ok(v) => v,
            Err(e) => {
                let _ = daemon.shutdown();
                return Err(e);
            }
        };
        Ok((receiver, service, daemon, vrc))
    }

    /// The OSC receiver and web server, on one port: ours if free, else the
    /// next free one, else any. Loopback only: VRChat runs on this PC, and the
    /// firewall never blocks loopback.
    fn bind_oscquery_port() -> Result<(OscReceiver, tiny_http::Server), String> {
        for port in (OSCQUERY_PORT..OSCQUERY_PORT + OSCQUERY_PORT_TRIES).chain([0]) {
            let Ok(receiver) = OscReceiver::bind("127.0.0.1", port) else { continue };
            if let Ok(server) = tiny_http::Server::http(("127.0.0.1", receiver.port)) {
                return Ok((receiver, server));
            }
        }
        Err("no free port for OSCQuery".into())
    }

    /// Changes the addresses advertised to VRChat, without restarting anything.
    pub fn set_listen_addresses(&self, addresses: &[String]) {
        if let Some(svc) = &self._service {
            svc.set_addresses(addresses);
        }
    }

    fn endpoints(&self) -> VrcEndpoints {
        self.vrc.as_ref().map(|v| v.lock().unwrap().endpoints.clone()).unwrap_or_default()
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
        // With OSC off VRChat still answers, but with an empty tree (no
        // avatar, no values), and says nothing over mDNS.
        let info = oscquery::fetch_avatar(q).ok().filter(|i| i.avatar_id.is_some() || !i.params.is_empty());
        if let Some(v) = &self.vrc {
            let mut g = v.lock().unwrap();
            if g.endpoints.query == Some(q) {
                g.alive = info.is_some();
            }
        }
        info
    }

    /// VRChat's OSC looks on (see `BridgeStatus::linked`).
    pub fn linked(&self) -> bool {
        self.vrc.as_ref().is_none_or(|v| v.lock().unwrap().alive)
    }

    pub fn status(&self) -> BridgeStatus {
        let ep = self.endpoints();
        let found = ep.osc.is_some() || ep.query.is_some();
        BridgeStatus {
            oscquery: self._service.is_some(),
            vrchat_found: found,
            linked: self.linked(),
            send_target: self.sender.target().to_string(),
            recv_port: self.receiver.port,
            backup_port: self.backup.as_ref().map(|b| b.port),
            query_port: self._service.as_ref().map(|s| s.http_port),
            vrchat_query: ep.query.map(|a| a.to_string()),
            note: self.note.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oscquery_port_is_the_same_after_a_rebuild() {
        let (r, s) = Bridge::bind_oscquery_port().unwrap();
        let port = r.port;
        assert_eq!(s.server_addr().to_ip().unwrap().port(), port, "web and OSC share the port");
        assert!((OSCQUERY_PORT..OSCQUERY_PORT + OSCQUERY_PORT_TRIES).contains(&port));
        drop((r, s));
        let (r2, _s2) = Bridge::bind_oscquery_port().unwrap();
        assert_eq!(r2.port, port, "freed right away, so the same port again");
    }
}
