//! OSCQuery: advertise our OSC server over mDNS so VRChat finds us with no
//! fixed ports, find VRChat, and read the current avatar's parameters.

use crate::osc::OscValue;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde_json::{json, Map, Value};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub const OSCJSON_TCP: &str = "_oscjson._tcp.local.";
pub const OSC_UDP: &str = "_osc._udp.local.";
const VRCHAT_PREFIX: &str = "VRChat-Client";

/// Where VRChat is, as found over mDNS.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VrcEndpoints {
    /// VRChat's OSC input (send here). Usually 127.0.0.1:9000.
    pub osc: Option<SocketAddr>,
    /// VRChat's OSCQuery HTTP server (read avatar parameters here).
    pub query: Option<SocketAddr>,
}

/// Builds the OSCQuery JSON tree for the addresses we want VRChat to send us.
pub fn build_tree(addresses: &[String]) -> Value {
    fn insert(node: &mut Map<String, Value>, parts: &[&str], full: &str, depth: usize) {
        let Some((head, rest)) = parts.split_first() else { return };
        let path = format!("/{}", full.trim_start_matches('/').split('/').take(depth + 1).collect::<Vec<_>>().join("/"));
        let contents = node
            .entry("CONTENTS")
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .unwrap();
        let child = contents
            .entry(head.to_string())
            .or_insert_with(|| json!({ "FULL_PATH": path, "ACCESS": 0 }))
            .as_object_mut()
            .unwrap();
        if rest.is_empty() {
            child.insert("ACCESS".into(), json!(2));
            let ty = if full == "/avatar/change" { "s" } else { "f" };
            child.insert("TYPE".into(), json!(ty));
        } else {
            insert(child, rest, full, depth + 1);
        }
    }
    let mut root = Map::new();
    root.insert("FULL_PATH".into(), json!("/"));
    root.insert("ACCESS".into(), json!(0));
    root.insert("DESCRIPTION".into(), json!("Pregnancy OSC"));
    for a in addresses {
        let parts: Vec<&str> = a.trim_start_matches('/').split('/').filter(|s| !s.is_empty()).collect();
        insert(&mut root, &parts, a, 0);
    }
    Value::Object(root)
}

fn find_node<'a>(tree: &'a Value, path: &str) -> Option<&'a Value> {
    let mut node = tree;
    for part in path.trim_matches('/').split('/').filter(|s| !s.is_empty()) {
        node = node.get("CONTENTS")?.get(part)?;
    }
    Some(node)
}

pub fn host_info(name: &str, osc_port: u16) -> Value {
    json!({
        "NAME": name,
        "OSC_IP": "127.0.0.1",
        "OSC_PORT": osc_port,
        "OSC_TRANSPORT": "UDP",
        "EXTENSIONS": { "ACCESS": true, "CLIPMODE": false, "RANGE": false, "TYPE": true, "VALUE": false }
    })
}

/// Our advertised OSCQuery service. Unregisters on drop.
pub struct OscQueryService {
    daemon: ServiceDaemon,
    names: Vec<String>,
    tree: Arc<Mutex<Value>>,
    pub http_port: u16,
}

impl OscQueryService {
    /// Starts the HTTP server and advertises `_oscjson._tcp` and `_osc._udp`.
    pub fn start(daemon: &ServiceDaemon, app_name: &str, osc_port: u16, addresses: Vec<String>) -> Result<Self, String> {
        // Loopback only: VRChat runs on this PC, and the firewall never blocks loopback.
        let server = tiny_http::Server::http("127.0.0.1:0").map_err(|e| e.to_string())?;
        let http_port = server.server_addr().to_ip().map(|a| a.port()).ok_or("no http port")?;
        let tree = Arc::new(Mutex::new(build_tree(&addresses)));
        let served = tree.clone();
        let info = host_info(app_name, osc_port);
        thread::Builder::new()
            .name("oscquery-http".into())
            .spawn(move || {
                for req in server.incoming_requests() {
                    let url = req.url().to_string();
                    let (path, query) = url.split_once('?').unwrap_or((&url, ""));
                    let body = if query.contains("HOST_INFO") {
                        Some(info.clone())
                    } else {
                        find_node(&served.lock().unwrap(), path).cloned()
                    };
                    let header = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
                    let _ = match body {
                        Some(b) => req.respond(tiny_http::Response::from_string(b.to_string()).with_header(header)),
                        None => req.respond(tiny_http::Response::from_string("not found").with_status_code(404)),
                    };
                }
            })
            .map_err(|e| e.to_string())?;

        let instance = format!("{app_name}-{}", std::process::id());
        let mut names = Vec::new();
        for (ty, port, kind) in [(OSCJSON_TCP, http_port, "oscjson"), (OSC_UDP, osc_port, "osc")] {
            // One host name per service type, like VRChat's own library uses.
            let host = format!("{instance}.{kind}.local.");
            // Announced at 127.0.0.1: with the PC's network addresses VRChat may try (and fail) to reach us over the LAN.
            let si = ServiceInfo::new(ty, &instance, &host, "127.0.0.1", port, None::<std::collections::HashMap<String, String>>)
                .map_err(|e| e.to_string())?;
            names.push(si.get_fullname().to_string());
            daemon.register(si).map_err(|e| e.to_string())?;
        }
        Ok(Self { daemon: daemon.clone(), names, tree, http_port })
    }

    /// Changes the addresses we ask VRChat for, keeping the same ports (VRChat
    /// only looks for apps at start-up, so a restart would lose it).
    pub fn set_addresses(&self, addresses: &[String]) {
        *self.tree.lock().unwrap() = build_tree(addresses);
    }
}

impl Drop for OscQueryService {
    fn drop(&mut self) {
        for n in &self.names {
            let _ = self.daemon.unregister(n);
        }
    }
}

/// Watches mDNS for VRChat and keeps `VrcEndpoints` up to date.
pub fn watch_vrchat(daemon: &ServiceDaemon) -> Result<Arc<Mutex<VrcEndpoints>>, String> {
    let state = Arc::new(Mutex::new(VrcEndpoints::default()));
    for ty in [OSCJSON_TCP, OSC_UDP] {
        let rx = daemon.browse(ty).map_err(|e| e.to_string())?;
        let st = state.clone();
        thread::Builder::new()
            .name(format!("mdns-{ty}"))
            .spawn(move || {
                while let Ok(ev) = rx.recv() {
                    match ev {
                        ServiceEvent::ServiceResolved(s) if s.fullname.starts_with(VRCHAT_PREFIX) => {
                            // VRChat runs on this machine; talk to it over loopback.
                            let addr = SocketAddr::from(([127, 0, 0, 1], s.port));
                            let mut g = st.lock().unwrap();
                            if s.ty_domain.starts_with("_oscjson") { g.query = Some(addr) } else { g.osc = Some(addr) }
                        }
                        ServiceEvent::ServiceRemoved(t, name) if name.starts_with(VRCHAT_PREFIX) => {
                            let mut g = st.lock().unwrap();
                            if t.starts_with("_oscjson") { g.query = None } else { g.osc = None }
                        }
                        _ => {}
                    }
                }
            })
            .map_err(|e| e.to_string())?;
    }
    Ok(state)
}

/// One avatar parameter as reported by VRChat's OSCQuery.
#[derive(Debug, Clone, PartialEq)]
pub struct AvatarParam {
    pub name: String,
    pub value: Option<OscValue>,
}

/// Collects every parameter under `/avatar/parameters` from an OSCQuery tree.
pub fn parse_avatar_params(tree: &Value) -> Vec<AvatarParam> {
    fn walk(node: &Value, out: &mut Vec<AvatarParam>) {
        if let (Some(path), Some(ty)) = (node.get("FULL_PATH").and_then(Value::as_str), node.get("TYPE").and_then(Value::as_str)) {
            if let Some(name) = path.strip_prefix("/avatar/parameters/") {
                let v = node.get("VALUE").and_then(|v| v.get(0));
                let value = v.and_then(|v| match ty {
                    "f" => v.as_f64().map(|f| OscValue::Float(f as f32)),
                    "i" => v.as_i64().map(|i| OscValue::Int(i as i32)),
                    "T" | "F" => v.as_bool().map(OscValue::Bool),
                    _ => None,
                });
                out.push(AvatarParam { name: name.to_string(), value });
            }
        }
        if let Some(c) = node.get("CONTENTS").and_then(Value::as_object) {
            c.values().for_each(|n| walk(n, out));
        }
    }
    let mut out = Vec::new();
    walk(tree, &mut out);
    out
}

/// The avatar you're wearing (`/avatar/change` holds its ID) and its parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct AvatarInfo {
    pub avatar_id: Option<String>,
    pub params: Vec<AvatarParam>,
}

/// The current avatar ID from an OSCQuery `/avatar` tree.
pub fn parse_current_avatar(tree: &Value) -> Option<String> {
    let v = tree.get("CONTENTS")?.get("change")?.get("VALUE")?.get(0)?.as_str()?;
    v.starts_with("avtr_").then(|| v.to_string())
}

/// Reads the current avatar and its parameters from VRChat's OSCQuery server.
pub fn fetch_avatar(query: SocketAddr) -> Result<AvatarInfo, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(3)))
        .build()
        .into();
    let body = agent
        .get(&format!("http://{query}/avatar"))
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    let tree: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    Ok(AvatarInfo { avatar_id: parse_current_avatar(&tree), params: parse_avatar_params(&tree) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_has_nested_paths() {
        let t = build_tree(&["/avatar/change".into(), "/avatar/parameters/Pregnancy/Stage".into()]);
        let stage = find_node(&t, "/avatar/parameters/Pregnancy/Stage").unwrap();
        assert_eq!(stage["FULL_PATH"], "/avatar/parameters/Pregnancy/Stage");
        assert_eq!(stage["ACCESS"], 2);
        assert_eq!(find_node(&t, "/avatar/change").unwrap()["TYPE"], "s");
        assert_eq!(find_node(&t, "/avatar/parameters").unwrap()["FULL_PATH"], "/avatar/parameters");
    }

    #[test]
    fn parses_vrchat_avatar_tree() {
        let tree = json!({
            "FULL_PATH": "/avatar", "CONTENTS": {
                "change": {"FULL_PATH": "/avatar/change", "TYPE": "s", "VALUE": ["avtr_123"]},
                "parameters": {"FULL_PATH": "/avatar/parameters", "CONTENTS": {
                    "Pregnancy": {"FULL_PATH": "/avatar/parameters/Pregnancy", "CONTENTS": {
                        "Stage": {"FULL_PATH": "/avatar/parameters/Pregnancy/Stage", "TYPE": "f", "VALUE": [0.25]},
                        "HideBelly": {"FULL_PATH": "/avatar/parameters/Pregnancy/HideBelly", "TYPE": "T", "VALUE": [false]}
                    }},
                    "VelocityX": {"FULL_PATH": "/avatar/parameters/VelocityX", "TYPE": "f", "VALUE": [0.0]}
                }}
            }
        });
        assert_eq!(parse_current_avatar(&tree).as_deref(), Some("avtr_123"));
        let mut p = parse_avatar_params(&tree);
        p.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(p.len(), 3);
        assert_eq!(p[1].name, "Pregnancy/Stage");
        assert_eq!(p[1].value, Some(OscValue::Float(0.25)));
        assert_eq!(p[0].value, Some(OscValue::Bool(false)));
    }
}
