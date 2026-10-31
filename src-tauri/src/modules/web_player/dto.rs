use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum Lifecycle {
    Closed,
    Visible,
    Hidden,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct WebPlayerSnapshot {
    pub lifecycle: Lifecycle,
    pub site_id: Option<String>,
    pub url: Option<String>,
    #[ts(type = "number")]
    pub rev: u64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SiteDto {
    pub id: String,
    pub name: String,
    pub home_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BridgeStatus {
    None,
    Playing,
    Paused,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(default)]
pub struct BridgeCaps {
    pub next: bool,
    pub previous: bool,
    pub seek: bool,
}

/// Message sent by `bridge.js`. Untrusted input: validated in `web_player::report`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeMessage {
    pub kind: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub artwork: Option<String>,
    pub status: BridgeStatus,
    pub position_ms: Option<f64>,
    pub duration_ms: Option<f64>,
    #[serde(default = "one")]
    pub rate: f32,
    #[serde(default)]
    pub is_ad: bool,
    #[serde(default)]
    pub caps: BridgeCaps,
}

fn one() -> f32 {
    1.0
}
