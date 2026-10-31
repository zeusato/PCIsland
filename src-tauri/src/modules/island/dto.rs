use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum VisibilityMode {
    Hidden,
    Minimal,
    Normal,
}

/// Interactive rectangle in logical px, relative to the island window.
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct HitRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VisibilityEvent {
    pub mode: VisibilityMode,
    pub reason: String,
    /// Whether an activity at Live priority or above exists (minimal is skipped then).
    pub has_live_activity: bool,
    #[ts(type = "number")]
    pub rev: u64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PointerEvent {
    pub inside: bool,
    /// Left button pressed outside the hit regions (closes a pinned peek).
    pub click_outside: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct IslandSnapshot {
    pub visibility: VisibilityEvent,
    pub scale_factor: f64,
}
