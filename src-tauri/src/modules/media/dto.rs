use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
    Changing,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct MediaCaps {
    pub play_pause: bool,
    pub next: bool,
    pub previous: bool,
    pub seek: bool,
}

/// What the island shows for the current source. Position is interpolated by the
/// frontend from `positionMs`, `positionAt` and `rate` (docs/02-system-design.md §8).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct NowPlayingDto {
    pub source_id: String,
    pub app_name: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub artwork_url: Option<String>,
    pub status: PlaybackStatus,
    #[ts(type = "number | null")]
    pub position_ms: Option<u64>,
    #[ts(type = "number")]
    pub position_at: u64,
    #[ts(type = "number | null")]
    pub duration_ms: Option<u64>,
    pub rate: f32,
    pub caps: MediaCaps,
    pub is_ad: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SourceSummary {
    pub source_id: String,
    pub app_name: String,
    pub status: PlaybackStatus,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct MediaSnapshot {
    pub now_playing: Option<NowPlayingDto>,
    pub sources: Vec<SourceSummary>,
    #[ts(type = "number")]
    pub rev: u64,
}
