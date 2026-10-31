//! Core services shared by every module (docs/02-system-design.md §3).

pub mod activity;
pub mod artwork;
pub mod audio;
pub mod id;
pub mod paths;
pub mod settings;
pub mod storage;

/// Activity priority levels (docs/modules/activity-hub.md §4).
pub mod priority {
    pub const IDLE: u8 = 0;
    pub const AMBIENT: u8 = 30;
    pub const MEDIA: u8 = 50;
    pub const LIVE: u8 = 60;
    pub const ALERT: u8 = 80;
    pub const CRITICAL: u8 = 100;
}
