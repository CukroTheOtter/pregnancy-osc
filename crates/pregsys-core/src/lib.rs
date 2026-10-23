//! Pure logic for Pregnancy OSC: no network, no UI.

pub mod achievements;
pub mod actions;
pub mod chatbox;
pub mod controls;
pub mod driver;
pub mod fertility;
pub mod instance;
pub mod logparse;
pub mod logwatch;
pub mod mapping;
pub mod model;
pub mod movement;
pub mod sps;
pub mod stage;
pub mod storage;
pub mod timeline;

/// Current time in Unix milliseconds.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
