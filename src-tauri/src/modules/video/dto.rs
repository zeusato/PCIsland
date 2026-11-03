use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export, rename = "VideoSourceKind")]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    /// hls.js on a (proxied) master playlist.
    Hls,
    /// `<video>` + `<audio>` synced on the audio clock.
    Pair,
    /// One `<video>` with both tracks.
    Single,
    /// Fallback A: the site's official embed in an iframe.
    Embed,
}

/// What the video window plays. Every URL is a `vstream://` URL, except `Embed`.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSource {
    pub kind: SourceKind,
    pub video: String,
    pub audio: Option<String>,
    pub title: String,
    pub uploader: Option<String>,
    #[ts(type = "number | null")]
    pub duration_ms: Option<u64>,
    pub is_live: bool,
    /// Heights for the quality menu (descending); empty = no choice.
    pub qualities: Vec<u32>,
    /// Height playing now (`None` = automatic).
    pub quality: Option<u32>,
    /// Position to start or resume from.
    #[ts(type = "number")]
    pub start_ms: u64,
    /// The link the user pasted, for "open in browser".
    pub page_url: String,
    /// Set when playback continues where the user left this video last time.
    #[ts(type = "number | null")]
    pub resumed_from_ms: Option<u64>,
    #[ts(type = "number")]
    pub rev: u64,
    // Applies to automatic playback only. Manual choices can exceed it.
    pub auto_max_height: u32,
    // Proxied audio-only streams keyed by language tag (empty tag = unnamed).
    pub audio_languages: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SniffedStream {
    pub id: String,
    pub label: String,
    pub hls: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", tag = "result")]
pub enum VideoOpenResult {
    Playing,
    /// yt-dlp is missing; the panel asks before downloading it.
    NeedsTools,
    /// A sniffed page offers several streams.
    Choose {
        streams: Vec<SniffedStream>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum VideoStatus {
    Loading,
    Playing,
    Paused,
    Ended,
    Error,
}

/// Sent by the video window whenever playback changes (and every 5 s while playing).
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VideoReport {
    pub status: VideoStatus,
    pub position_ms: f64,
    pub duration_ms: Option<f64>,
    pub muted: bool,
}

/// `video://state` for the island and main window.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VideoSnapshot {
    pub open: bool,
    pub pinned: bool,
    pub muted: bool,
    pub title: Option<String>,
    pub status: Option<VideoStatus>,
    #[ts(type = "number")]
    pub rev: u64,
}

/// `video://command` to the video window.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VideoCommand {
    pub action: String,
    pub value: Option<f64>,
}

/// Bounded report from the hidden sniffing view.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SniffItem {
    pub url: String,
    pub mime: Option<String>,
}
