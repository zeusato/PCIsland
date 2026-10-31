//! Core services shared by every module (docs/02-system-design.md §3).

pub mod artwork;

/// Activity priority levels (docs/modules/activity-hub.md §4).
/// P0 stand-in: until ActivityHub lands (roadmap P1.2), modules publish the highest
/// priority they currently hold through a `watch` channel created in `lib.rs`.
pub mod priority {
    pub const IDLE: u8 = 0;
    pub const AMBIENT: u8 = 30;
    pub const MEDIA: u8 = 50;
    pub const LIVE: u8 = 60;
    #[allow(dead_code, reason = "used by ActivityHub (roadmap P1.2)")]
    pub const ALERT: u8 = 80;
    pub const CRITICAL: u8 = 100;
}
