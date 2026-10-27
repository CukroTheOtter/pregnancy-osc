//! A tiny mDNS announcer for the app's OSCQuery service.
//!
//! mdns-sd only announces an address on the network adapter whose subnet
//! holds it, so 127.0.0.1 only ever went out on loopback, where VRChat
//! doesn't listen: VRChat never found the app. This announces 127.0.0.1 on
//! the real adapters every few seconds (like OGB) and answers this PC's
//! searches, laid out like VRChat's own answers. Nothing leaves this PC.

use socket2::{Domain, Protocol, SockRef, Socket, Type};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const PORT: u16 = 5353;
const TTL_PTR: u32 = 4500;
const TTL_HOST: u32 = 120;
/// Announced this often, like OGB: VRChat matches apps by name, so repeats
/// don't add us twice, and a VRChat that missed us finds us within seconds.
const REANNOUNCE: Duration = Duration::from_secs(5);

/// One advertised service, e.g. `_oscjson._tcp.local.` on `port`.
#[derive(Debug, Clone)]
pub struct Service {
    pub ty: String,
    pub instance: String,
    pub host: String,
    pub port: u16,
}

impl Service {
    fn fullname(&self) -> String {
        format!("{}.{}", self.instance, self.ty)
    }
}

/// Answers for our services until dropped (then says goodbye).
pub struct Announcer {
    stop: Arc<AtomicBool>,
}

/// This PC's IPv4 addresses (loopback included).
fn adapters() -> Vec<Ipv4Addr> {
    let mut ips: Vec<Ipv4Addr> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|i| match i.ip() {
            std::net::IpAddr::V4(ip) => Some(ip),
            _ => None,
        })
        .collect();
    ips.sort();
    ips.dedup();
    ips
}

/// Where to send: the real adapters (VRChat listens there, not on
/// loopback), or loopback when there are none.
fn send_ifs(ips: &[Ipv4Addr]) -> Vec<Ipv4Addr> {
    let real: Vec<Ipv4Addr> = ips.iter().copied().filter(|ip| !ip.is_loopback()).collect();
    if real.is_empty() { vec![Ipv4Addr::LOCALHOST] } else { real }
}

/// Sends `pkt` to the mDNS group out of each of `ips`.
fn send_on(sock: &UdpSocket, pkt: &[u8], ips: &[Ipv4Addr]) {
    let to = SocketAddr::V4(SocketAddrV4::new(GROUP, PORT));
    for ip in ips {
        if SockRef::from(sock).set_multicast_if_v4(ip).is_ok() {
            let _ = sock.send_to(pkt, to);
        }
    }
}

impl Announcer {
    pub fn start(services: Vec<Service>) -> Result<Self, String> {
        let sock = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).map_err(|e| e.to_string())?;
        // Shared with VRChat, Bonjour and mdns-sd, which all listen here too.
        sock.set_reuse_address(true).map_err(|e| e.to_string())?;
        sock.bind(&SocketAddr::from(([0, 0, 0, 0], PORT)).into()).map_err(|e| e.to_string())?;
        sock.set_multicast_loop_v4(true).map_err(|e| e.to_string())?;
        // TTL 0: our packets never leave this PC. 127.0.0.1 means nothing to
        // other PCs, and their VRChat would keep trying (and failing) to connect.
        sock.set_multicast_ttl_v4(0).map_err(|e| e.to_string())?;
        sock.set_read_timeout(Some(Duration::from_millis(500))).map_err(|e| e.to_string())?;
        let sock: UdpSocket = sock.into();

        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let list = services;
        thread::Builder::new()
            .name("mdns-announce".into())
            .spawn(move || {
                let mut joined: Vec<Ipv4Addr> = Vec::new();
                let mut last_scan: Option<Instant> = None;
                let mut last_hello: Option<Instant> = None;
                // One service per packet: VRChat only reads the first SRV in each.
                let each = |goodbye: bool| list.iter().map(|s| packet(&[s], goodbye)).collect::<Vec<_>>();
                let hello = each(false);
                let mut buf = [0u8; 9000];
                while !stopped.load(Ordering::SeqCst) {
                    // Adapters can come and go (Wi-Fi, VPN): check now and then.
                    if last_scan.is_none_or(|t| t.elapsed() >= Duration::from_secs(10)) {
                        last_scan = Some(Instant::now());
                        let now = adapters();
                        for ip in &now {
                            if !joined.contains(ip) {
                                let _ = sock.join_multicast_v4(&GROUP, ip);
                            }
                        }
                        joined = now;
                    }
                    if last_hello.is_none_or(|t| t.elapsed() >= REANNOUNCE) {
                        last_hello = Some(Instant::now());
                        for p in &hello {
                            send_on(&sock, p, &send_ifs(&joined));
                        }
                    }
                    let Ok((n, from)) = sock.recv_from(&mut buf) else { continue };
                    // Only searches from this PC: other PCs can't reach 127.0.0.1.
                    let local = match from.ip() {
                        std::net::IpAddr::V4(ip) => ip.is_loopback() || joined.contains(&ip),
                        _ => false,
                    };
                    if !local {
                        continue;
                    }
                    let Some(asked) = parse_query(&buf[..n]) else { continue };
                    for s in list.iter().filter(|s| asked.iter().any(|q| wanted(s, q))) {
                        let pkt = packet(&[s], false);
                        send_on(&sock, &pkt, &send_ifs(&joined));
                        // A one-shot query (not from port 5353) wants the answer back directly.
                        if from.port() != PORT {
                            let _ = sock.send_to(&pkt, from);
                        }
                    }
                }
                // Goodbye: TTL 0 records tell everyone the services are gone.
                for p in each(true) {
                    send_on(&sock, &p, &send_ifs(&joined));
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { stop })
    }
}

impl Drop for Announcer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn same(a: &str, b: &str) -> bool {
    a.trim_end_matches('.').eq_ignore_ascii_case(b.trim_end_matches('.'))
}

/// Does a question for `name` concern `s`?
fn wanted(s: &Service, name: &str) -> bool {
    same(name, &s.ty) || same(name, &s.fullname()) || same(name, &s.host)
}

/// The names asked about in an mDNS query (None for answers and junk).
fn parse_query(p: &[u8]) -> Option<Vec<String>> {
    if p.len() < 12 || p[2] & 0x80 != 0 {
        return None; // too short, or a response
    }
    let qd = u16::from_be_bytes([p[4], p[5]]) as usize;
    let mut at = 12;
    let mut out = Vec::new();
    for _ in 0..qd {
        let (name, next) = read_name(p, at)?;
        at = next + 4; // type and class
        if at > p.len() {
            return None;
        }
        out.push(name);
    }
    Some(out)
}

/// Reads a DNS name at `at` (following compression pointers). Returns it
/// and where the data after it starts.
fn read_name(p: &[u8], mut at: usize) -> Option<(String, usize)> {
    let mut labels: Vec<String> = Vec::new();
    let mut end = None;
    for _ in 0..64 {
        let len = *p.get(at)? as usize;
        if len == 0 {
            return Some((labels.join("."), end.unwrap_or(at + 1)));
        }
        if len & 0xC0 == 0xC0 {
            let ptr = ((len & 0x3F) << 8) | *p.get(at + 1)? as usize;
            end.get_or_insert(at + 2);
            at = ptr;
            continue;
        }
        labels.push(String::from_utf8_lossy(p.get(at + 1..at + 1 + len)?).into_owned());
        at += 1 + len;
    }
    None
}

fn put_name(out: &mut Vec<u8>, name: &str) {
    for label in name.trim_end_matches('.').split('.') {
        out.push(label.len() as u8);
        out.extend_from_slice(label.as_bytes());
    }
    out.push(0);
}

fn put_record(out: &mut Vec<u8>, name: &str, ty: u16, ttl: u32, data: &[u8]) {
    put_name(out, name);
    out.extend_from_slice(&ty.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes()); // class IN
    out.extend_from_slice(&ttl.to_be_bytes());
    out.extend_from_slice(&(data.len() as u16).to_be_bytes());
    out.extend_from_slice(data);
}

/// A response like VRChat's own: the PTR for each of `hits` as an answer,
/// and its SRV, TXT and A (127.0.0.1) as additional records (VRChat only
/// looks for the SRV there). `goodbye` sends them with TTL 0.
fn packet(hits: &[&Service], goodbye: bool) -> Vec<u8> {
    let ttl = |t: u32| if goodbye { 0 } else { t };
    let mut out = vec![0, 0, 0x84, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    out[6..8].copy_from_slice(&(hits.len() as u16).to_be_bytes());
    out[10..12].copy_from_slice(&((hits.len() * 3) as u16).to_be_bytes());
    for s in hits {
        let mut ptr = Vec::new();
        put_name(&mut ptr, &s.fullname());
        put_record(&mut out, &s.ty, 12, ttl(TTL_PTR), &ptr);
    }
    for s in hits {
        let full = s.fullname();
        put_record(&mut out, &full, 16, ttl(TTL_PTR), b"\x09txtvers=1");
        let mut srv = vec![0, 0, 0, 0];
        srv.extend_from_slice(&s.port.to_be_bytes());
        put_name(&mut srv, &s.host);
        put_record(&mut out, &full, 33, ttl(TTL_PTR), &srv);
        put_record(&mut out, &s.host, 1, ttl(TTL_HOST), &[127, 0, 0, 1]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svc() -> Service {
        Service { ty: "_oscjson._tcp.local.".into(), instance: "PregnancyOSC-1".into(), host: "PregnancyOSC-1.oscjson.local.".into(), port: 1234 }
    }

    #[test]
    fn reads_a_query_with_compression() {
        // One question for _oscjson._tcp.local PTR, then a second pointing back at "local".
        let mut q = vec![0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0];
        put_name(&mut q, "_oscjson._tcp.local");
        q.extend_from_slice(&[0, 12, 0, 1]);
        q.extend_from_slice(&[4, b'h', b'o', b's', b't', 0xC0, 12 + 9 + 5]); // "host" + pointer to "local"
        q.extend_from_slice(&[0, 1, 0, 1]);
        let names = parse_query(&q).unwrap();
        assert_eq!(names, vec!["_oscjson._tcp.local", "host.local"]);
        assert!(wanted(&svc(), &names[0]));
        assert!(!wanted(&svc(), &names[1]));
    }

    #[test]
    fn answers_are_not_queries() {
        let p = packet(&[&svc()], false);
        assert!(parse_query(&p).is_none());
    }

    #[test]
    fn packet_is_laid_out_like_vrchats() {
        let p = packet(&[&svc()], false);
        assert_eq!(u16::from_be_bytes([p[6], p[7]]), 1, "PTR as the answer");
        assert_eq!(u16::from_be_bytes([p[10], p[11]]), 3, "SRV, TXT and A as additional records");
        assert!(p.ends_with(&[0, 4, 127, 0, 0, 1]), "A record for 127.0.0.1 last");
    }
}
