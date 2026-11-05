//! Debug mode only: what the engine is thinking, written to files for testing.
//! `debug/state.json` is rewritten every second; `debug/trace.jsonl` gets one
//! line per thing that happened (what came in, what went out, and why).

use pregsys_core::driver::{ParamValue, Send};
use pregsys_core::now_ms;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The trace moves to `trace.old.jsonl` at this size, so at most twice this is kept.
const MAX_BYTES: u64 = 5 * 1024 * 1024;
/// Sent every few seconds whatever happens, so they'd only hide the rest.
const HEARTBEAT: [&str; 2] = ["appActive", "appHeartbeat"];
/// The belly is re-sent every second; this few unchanged values is that refresh.
const REFRESH_SIZE: usize = 2;
/// A float moving less than this isn't a change worth a line (the belly grows
/// a tiny bit all the time).
const SMALL_CHANGE: f32 = 0.001;

pub struct Trace {
    dir: PathBuf,
    on: bool,
    file: Option<(File, u64)>,
    last_heard: HashMap<String, (f32, &'static str)>,
    last_sent: HashMap<String, ParamValue>,
}

fn same(a: ParamValue, b: ParamValue) -> bool {
    match (a, b) {
        (ParamValue::Float(a), ParamValue::Float(b)) => (a - b).abs() < SMALL_CHANGE,
        _ => a == b,
    }
}

fn to_json(v: ParamValue) -> Value {
    match v {
        ParamValue::Float(f) => json!(rounded(f as f64)),
        ParamValue::Int(i) => json!(i),
        ParamValue::Bool(b) => json!(b),
    }
}

/// 0.35 instead of 0.3499999940395355.
fn rounded(f: f64) -> f64 {
    (f * 10_000.0).round() / 10_000.0
}

fn round_floats(value: &mut Value) {
    match value {
        Value::Number(n) if n.is_f64() => {
            if let Some(f) = n.as_f64() {
                *value = json!(rounded(f));
            }
        }
        Value::Array(items) => items.iter_mut().for_each(round_floats),
        Value::Object(fields) => fields.values_mut().for_each(round_floats),
        _ => {}
    }
}

impl Trace {
    pub fn new(data_dir: &Path) -> Self {
        Self { dir: data_dir.join("debug"), on: false, file: None, last_heard: HashMap::new(), last_sent: HashMap::new() }
    }

    /// Follows debug mode. Nothing is written while it's off.
    pub fn set_on(&mut self, on: bool) {
        if on == self.on {
            return;
        }
        if on {
            self.on = true;
            self.note("trace", &format!("Trace on (app {})", env!("CARGO_PKG_VERSION")));
        } else {
            self.note("trace", "Trace off");
            self.on = false;
            self.file = None;
            self.last_heard.clear();
            self.last_sent.clear();
        }
    }

    /// Something the engine did or decided.
    pub fn note(&mut self, kind: &str, text: &str) {
        self.line(json!({ "t": now_ms(), "kind": kind, "text": text }));
    }

    /// A value VRChat reported and what became of it. Repeats and tiny changes are left out.
    pub fn heard(&mut self, name: &str, value: f32, result: &'static str) {
        if !self.on {
            return;
        }
        if self.last_heard.get(name).is_some_and(|&(v, r)| r == result && (v - value).abs() < SMALL_CHANGE) {
            return;
        }
        self.last_heard.insert(name.to_string(), (value, result));
        self.line(json!({ "t": now_ms(), "kind": "received", "name": name, "value": rounded(value as f64), "result": result }));
    }

    /// A new avatar: its values are worth a line even when they match the last one's.
    pub fn new_avatar(&mut self) {
        self.last_heard.clear();
    }

    /// One tick's sends. Heartbeats and the belly's once-a-second refresh are left out.
    pub fn sent(&mut self, sends: &[Send]) {
        if !self.on {
            return;
        }
        let items: Vec<&Send> = sends.iter().filter(|s| !HEARTBEAT.contains(&s.key.as_str())).collect();
        let changed = items.iter().any(|s| !self.last_sent.get(&s.key).is_some_and(|&v| same(v, s.value)));
        if items.is_empty() || (!changed && items.len() <= REFRESH_SIZE) {
            return;
        }
        let mut values = Map::new();
        for s in items {
            self.last_sent.insert(s.key.clone(), s.value);
            values.insert(s.name.clone(), to_json(s.value));
        }
        self.line(json!({ "t": now_ms(), "kind": "sent", "values": values }));
    }

    /// Replaces `state.json` in one go, so a reader never sees half of it.
    pub fn state(&self, mut state: Value) {
        if !self.on {
            return;
        }
        round_floats(&mut state);
        let write = || -> std::io::Result<()> {
            fs::create_dir_all(&self.dir)?;
            let tmp = self.dir.join("state.json.tmp");
            fs::write(&tmp, serde_json::to_vec_pretty(&state)?)?;
            fs::rename(tmp, self.dir.join("state.json"))
        };
        if let Err(e) = write() {
            log::warn!("Could not write the debug state: {e}");
        }
    }

    fn line(&mut self, value: Value) {
        if !self.on {
            return;
        }
        if let Err(e) = self.append(&value) {
            log::warn!("Could not write the debug trace: {e}");
            self.file = None;
        }
    }

    fn append(&mut self, value: &Value) -> std::io::Result<()> {
        let path = self.dir.join("trace.jsonl");
        if self.file.as_ref().is_some_and(|(_, size)| *size >= MAX_BYTES) {
            self.file = None;
            fs::rename(&path, self.dir.join("trace.old.jsonl"))?;
        }
        if self.file.is_none() {
            fs::create_dir_all(&self.dir)?;
            let file = OpenOptions::new().create(true).append(true).open(&path)?;
            let size = file.metadata()?.len();
            self.file = Some((file, size));
        }
        let mut text = value.to_string();
        text.push('\n');
        if let Some((file, size)) = self.file.as_mut() {
            file.write_all(text.as_bytes())?;
            *size += text.len() as u64;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dir(PathBuf);

    impl Dir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("pregsys-trace-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            Self(dir)
        }

        fn lines(&self) -> Vec<Value> {
            fs::read_to_string(self.0.join("debug/trace.jsonl"))
                .unwrap_or_default()
                .lines()
                .map(|l| serde_json::from_str(l).unwrap())
                .collect()
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn send(key: &str, value: ParamValue) -> Send {
        Send { key: key.into(), name: format!("P_{key}"), value }
    }

    #[test]
    fn off_writes_nothing() {
        let dir = Dir::new("off");
        let mut t = Trace::new(&dir.0);
        t.note("x", "y");
        t.heard("a", 1.0, "param");
        t.sent(&[send("stage", ParamValue::Float(0.5))]);
        t.state(json!({}));
        assert!(!dir.0.join("debug").exists());
    }

    #[test]
    fn sends_leave_out_heartbeats_and_refreshes() {
        let dir = Dir::new("sent");
        let mut t = Trace::new(&dir.0);
        t.set_on(true);
        let stage = send("stage", ParamValue::Float(0.5));
        t.sent(&[send("appHeartbeat", ParamValue::Bool(true))]); // heartbeat
        t.sent(&[stage.clone()]); // new: a line
        t.sent(&[stage.clone()]); // refresh
        t.sent(&[send("stage", ParamValue::Float(0.5004))]); // tiny growth
        t.sent(&[send("stage", ParamValue::Float(0.6))]); // a real change: a line
        // Everything sent again after a resync, even unchanged: a line.
        let all = [send("stage", ParamValue::Float(0.6)), send("a", ParamValue::Int(1)), send("b", ParamValue::Bool(true))];
        t.sent(&all);
        t.sent(&all);
        let sent: Vec<Value> = dir.lines().into_iter().filter(|l| l["kind"] == "sent").collect();
        assert_eq!(sent.len(), 4, "{sent:?}");
        assert_eq!(sent[0]["values"]["P_stage"], json!(0.5));
        assert!(sent[0]["values"].get("P_appHeartbeat").is_none());
    }

    #[test]
    fn floats_are_rounded() {
        let dir = Dir::new("round");
        let mut t = Trace::new(&dir.0);
        t.set_on(true);
        t.sent(&[send("x", ParamValue::Float(0.35))]);
        t.heard("y", 0.35, "param");
        t.state(json!({ "a": { "b": [0.35_f32] }, "n": 1_793_895_444_463_i64 }));
        let lines = dir.lines();
        assert_eq!(lines[1]["values"]["P_x"], json!(0.35));
        assert_eq!(lines[2]["value"], json!(0.35));
        let read: Value = serde_json::from_str(&fs::read_to_string(dir.0.join("debug/state.json")).unwrap()).unwrap();
        assert_eq!(read["a"]["b"][0], json!(0.35));
        assert_eq!(read["n"], json!(1_793_895_444_463_i64));
    }

    #[test]
    fn received_repeats_are_left_out_until_a_new_avatar() {
        let dir = Dir::new("heard");
        let mut t = Trace::new(&dir.0);
        t.set_on(true);
        t.heard("Hide", 1.0, "held");
        t.heard("Hide", 1.0, "held");
        t.heard("Hide", 1.0, "ignored"); // same value, another outcome
        t.heard("Hide", 1.0004, "ignored"); // tiny change
        t.new_avatar();
        t.heard("Hide", 1.0, "ignored");
        let heard = dir.lines().into_iter().filter(|l| l["kind"] == "received").count();
        assert_eq!(heard, 3);
    }

    #[test]
    fn full_trace_moves_aside() {
        let dir = Dir::new("rotate");
        let mut t = Trace::new(&dir.0);
        t.set_on(true);
        t.file.as_mut().unwrap().1 = MAX_BYTES;
        t.note("x", "new file");
        assert!(dir.0.join("debug/trace.old.jsonl").exists());
        let lines = dir.lines();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0]["text"], "new file");
    }

    #[test]
    fn state_is_replaced() {
        let dir = Dir::new("state");
        let mut t = Trace::new(&dir.0);
        t.set_on(true);
        t.state(json!({ "n": 1 }));
        t.state(json!({ "n": 2 }));
        let read: Value = serde_json::from_str(&fs::read_to_string(dir.0.join("debug/state.json")).unwrap()).unwrap();
        assert_eq!(read["n"], 2);
        assert!(!dir.0.join("debug/state.json.tmp").exists());
    }
}
