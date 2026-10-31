//! IPC boundary: error type shared by every command and event name constants.

use serde::Serialize;
use ts_rs::TS;

/// Error returned by every command. `code` is stable (`module.reason`) and mapped to
/// `errors.<code>` by the frontend; `message` is English and meant for logs.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct IpcError {
    pub code: String,
    pub message: String,
}

impl IpcError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }
}

impl From<tauri::Error> for IpcError {
    fn from(err: tauri::Error) -> Self {
        Self::new("app.tauri", err.to_string())
    }
}

pub type IpcResult<T> = Result<T, IpcError>;

pub mod events {
    pub const ISLAND_STATE: &str = "island://state";
    pub const SETTINGS_CHANGED: &str = "settings://changed";
    pub const TIMER_STATE: &str = "timer://state";
    pub const REMINDERS_STATE: &str = "reminders://state";
    pub const NOTES_STATE: &str = "notes://state";
    pub const SHELF_STATE: &str = "shelf://state";
    pub const LOCAL_PLAYER_STATE: &str = "local_player://state";
    pub const LYRICS_CURRENT: &str = "lyrics://current";
    pub const ISLAND_VISIBILITY: &str = "island://visibility";
    pub const ISLAND_POINTER: &str = "island://pointer";
    pub const MEDIA_NOW_PLAYING: &str = "media://now-playing";
    pub const WEB_PLAYER_STATE: &str = "web_player://state";
}

/// Milliseconds since the Unix epoch, used for every `*At` field (docs/02-system-design.md §8).
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}
