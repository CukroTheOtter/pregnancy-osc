//! Plain OSC over UDP: send to VRChat, receive from VRChat.

use rosc::{OscMessage, OscPacket, OscType};
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc::{self, Receiver};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum OscValue {
    Float(f32),
    Int(i32),
    Bool(bool),
    Str(String),
}

impl OscValue {
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            OscValue::Float(f) => Some(*f),
            OscValue::Int(i) => Some(*i as f32),
            OscValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            OscValue::Str(_) => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            OscValue::Bool(b) => Some(*b),
            OscValue::Int(i) => Some(*i != 0),
            OscValue::Float(f) => Some(*f > 0.5),
            OscValue::Str(_) => None,
        }
    }

    fn to_osc(&self) -> OscType {
        match self {
            OscValue::Float(f) => OscType::Float(*f),
            OscValue::Int(i) => OscType::Int(*i),
            OscValue::Bool(b) => OscType::Bool(*b),
            OscValue::Str(s) => OscType::String(s.clone()),
        }
    }

    fn from_osc(t: &OscType) -> Option<Self> {
        Some(match t {
            OscType::Float(f) => OscValue::Float(*f),
            OscType::Double(d) => OscValue::Float(*d as f32),
            OscType::Int(i) => OscValue::Int(*i),
            OscType::Long(l) => OscValue::Int(*l as i32),
            OscType::Bool(b) => OscValue::Bool(*b),
            OscType::String(s) => OscValue::Str(s.clone()),
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Incoming {
    pub addr: String,
    pub value: OscValue,
}

pub fn encode(addr: &str, value: &OscValue) -> Vec<u8> {
    encode_args(addr, std::slice::from_ref(value))
}

pub fn encode_args(addr: &str, values: &[OscValue]) -> Vec<u8> {
    let msg = OscPacket::Message(OscMessage { addr: addr.to_string(), args: values.iter().map(OscValue::to_osc).collect() });
    rosc::encoder::encode(&msg).unwrap_or_default()
}

/// Decodes one UDP datagram, flattening bundles.
pub fn decode(buf: &[u8]) -> Vec<Incoming> {
    fn walk(p: OscPacket, out: &mut Vec<Incoming>) {
        match p {
            OscPacket::Message(m) => {
                if let Some(v) = m.args.first().and_then(OscValue::from_osc) {
                    out.push(Incoming { addr: m.addr, value: v });
                }
            }
            OscPacket::Bundle(b) => b.content.into_iter().for_each(|p| walk(p, out)),
        }
    }
    let mut out = Vec::new();
    if let Ok((_, packet)) = rosc::decoder::decode_udp(buf) {
        walk(packet, &mut out);
    }
    out
}

/// Sends OSC messages to one target (VRChat).
#[derive(Clone)]
pub struct OscSender {
    sock: Arc<UdpSocket>,
    target: Arc<Mutex<SocketAddr>>,
}

impl OscSender {
    pub fn new(target: SocketAddr) -> std::io::Result<Self> {
        let sock = UdpSocket::bind(("127.0.0.1", 0)).or_else(|_| UdpSocket::bind(("0.0.0.0", 0)))?;
        Ok(Self { sock: Arc::new(sock), target: Arc::new(Mutex::new(target)) })
    }

    pub fn target(&self) -> SocketAddr {
        *self.target.lock().unwrap()
    }

    pub fn set_target(&self, t: SocketAddr) {
        *self.target.lock().unwrap() = t;
    }

    pub fn send(&self, addr: &str, value: &OscValue) -> std::io::Result<()> {
        self.send_args(addr, std::slice::from_ref(value))
    }

    /// One message with several arguments (e.g. the chatbox).
    pub fn send_args(&self, addr: &str, values: &[OscValue]) -> std::io::Result<()> {
        self.sock.send_to(&encode_args(addr, values), self.target()).map(|_| ())
    }
}

/// Listens for OSC on a UDP port. Port 0 picks a free port. The socket
/// closes when this is dropped.
pub struct OscReceiver {
    pub port: u16,
    pub rx: Receiver<Incoming>,
    stop: Arc<AtomicBool>,
}

impl Drop for OscReceiver {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

impl OscReceiver {
    pub fn bind(ip: &str, port: u16) -> std::io::Result<Self> {
        let sock = UdpSocket::bind((ip, port))?;
        let port = sock.local_addr()?.port();
        sock.set_read_timeout(Some(Duration::from_millis(500)))?;
        let (tx, rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        thread::Builder::new().name("osc-recv".into()).spawn(move || {
            let mut buf = [0u8; 8192];
            // Checked every read timeout (0.5 s), so the socket is freed soon after drop.
            while !stopped.load(Ordering::SeqCst) {
                match sock.recv_from(&mut buf) {
                    Ok((n, _)) => {
                        for m in decode(&buf[..n]) {
                            if tx.send(m).is_err() {
                                return;
                            }
                        }
                    }
                    Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                    // Windows reports ICMP "port unreachable" as a recv error; ignore it.
                    Err(_) => thread::sleep(Duration::from_millis(50)),
                }
            }
        })?;
        Ok(Self { port, rx, stop })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_over_udp() {
        let r = OscReceiver::bind("127.0.0.1", 0).unwrap();
        let s = OscSender::new(SocketAddr::from(([127, 0, 0, 1], r.port))).unwrap();
        s.send("/avatar/parameters/Pregnancy/Stage", &OscValue::Float(0.5)).unwrap();
        s.send("/avatar/parameters/Pregnancy/HideBelly", &OscValue::Bool(true)).unwrap();
        let a = r.rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(a.addr, "/avatar/parameters/Pregnancy/Stage");
        assert_eq!(a.value, OscValue::Float(0.5));
        let b = r.rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(b.value.as_bool(), Some(true));
    }

    #[test]
    fn dropping_frees_the_port() {
        let r = OscReceiver::bind("127.0.0.1", 0).unwrap();
        let port = r.port;
        drop(r);
        std::thread::sleep(Duration::from_millis(800));
        assert!(OscReceiver::bind("127.0.0.1", port).is_ok(), "port is free again");
    }

    #[test]
    fn garbage_is_ignored() {
        assert!(decode(b"not osc at all").is_empty());
    }
}
