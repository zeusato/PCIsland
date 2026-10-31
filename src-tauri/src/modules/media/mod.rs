//! Media hub: merges every source into one `NowPlaying` (docs/modules/media-hub.md).
//! P0 scope: external sources via GSMTC + the web player source.
#![forbid(unsafe_code)]

pub mod commands;
pub mod dto;
pub mod select;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, watch};

use crate::core::artwork::ArtworkCache;
use crate::core::priority;
use crate::ipc::{events, now_ms};
use crate::platform::gsmtc::{self, GsmtcCommand, SessionInfo};
use dto::{MediaCaps, MediaSnapshot, NowPlayingDto, PlaybackStatus, SourceSummary};

/// `media.paused_media_ambient_min` default (docs/modules/activity-hub.md §7).
const PAUSED_WINDOW_MS: u64 = 30 * 60 * 1000;
pub const WEB_SOURCE_ID: &str = "web";

#[derive(Debug, Clone, Copy)]
pub enum MediaCommand {
    PlayPause,
    Next,
    Previous,
}

/// Control callback a non-GSMTC source registers (implementation of the `MediaSource`
/// idea from docs/02-system-design.md §11, reduced for P0).
pub type SourceControl = Arc<dyn Fn(MediaCommand) + Send + Sync>;

enum Input {
    External(Vec<SessionInfo>),
    Web(Option<NowPlayingDto>),
    Command(MediaCommand),
}

struct Entry {
    dto: NowPlayingDto,
    last_playing_at: u64,
}

#[derive(Clone)]
pub struct MediaHandle {
    input: mpsc::UnboundedSender<Input>,
    snapshot: Arc<Mutex<MediaSnapshot>>,
    web_control: Arc<Mutex<Option<SourceControl>>>,
}

impl MediaHandle {
    pub fn snapshot(&self) -> MediaSnapshot {
        self.snapshot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn command(&self, cmd: MediaCommand) {
        let _ = self.input.send(Input::Command(cmd));
    }

    /// Called by the web player whenever its bridge reports a new state.
    pub fn update_web_source(&self, state: Option<NowPlayingDto>) {
        let _ = self.input.send(Input::Web(state));
    }

    pub fn register_web_control(&self, control: SourceControl) {
        *self
            .web_control
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(control);
    }
}

pub fn setup(
    app: &AppHandle,
    artwork: Arc<ArtworkCache>,
    top_priority: watch::Sender<u8>,
) -> MediaHandle {
    let (tx, rx) = mpsc::unbounded_channel();
    let forward = tx.clone();
    let gsmtc_tx = gsmtc::spawn(move |sessions| {
        let _ = forward.send(Input::External(sessions));
    });
    let handle = MediaHandle {
        input: tx,
        snapshot: Arc::new(Mutex::new(MediaSnapshot {
            now_playing: None,
            sources: Vec::new(),
            rev: 0,
        })),
        web_control: Arc::new(Mutex::new(None)),
    };
    tauri::async_runtime::spawn(service(
        app.clone(),
        handle.clone(),
        rx,
        gsmtc_tx,
        artwork,
        top_priority,
    ));
    handle
}

async fn service(
    app: AppHandle,
    handle: MediaHandle,
    mut rx: mpsc::UnboundedReceiver<Input>,
    gsmtc_tx: std::sync::mpsc::Sender<GsmtcCommand>,
    artwork: Arc<ArtworkCache>,
    top_priority: watch::Sender<u8>,
) {
    let own_exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()))
        .unwrap_or_default();
    let mut entries: HashMap<String, Entry> = HashMap::new();
    let mut current: Option<String> = None;

    while let Some(input) = rx.recv().await {
        let now = now_ms();
        match input {
            Input::External(sessions) => {
                entries.retain(|id, _| id == WEB_SOURCE_ID);
                for s in sessions {
                    let id = format!("external:{}", s.aumid);
                    let lower = s.aumid.to_lowercase();
                    // Our own SMTC session (WebView2 / own player) is not an external source.
                    if lower.contains("pcisland")
                        || (!own_exe.is_empty() && lower.contains(&own_exe))
                    {
                        continue;
                    }
                    let dto = external_dto(&id, &s, &artwork);
                    upsert(&mut entries, id, dto, now);
                }
            }
            Input::Web(Some(dto)) => upsert(&mut entries, WEB_SOURCE_ID.to_owned(), dto, now),
            Input::Web(None) => {
                entries.remove(WEB_SOURCE_ID);
            }
            Input::Command(cmd) => {
                let Some(id) = current.clone() else { continue };
                if id == WEB_SOURCE_ID {
                    let control = handle
                        .web_control
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .clone();
                    if let Some(control) = control {
                        control(cmd);
                    }
                } else if let Some(aumid) = id.strip_prefix("external:") {
                    let aumid = aumid.to_owned();
                    let _ = gsmtc_tx.send(match cmd {
                        MediaCommand::PlayPause => GsmtcCommand::TogglePlayPause(aumid),
                        MediaCommand::Next => GsmtcCommand::Next(aumid),
                        MediaCommand::Previous => GsmtcCommand::Previous(aumid),
                    });
                }
                continue;
            }
        }

        let candidates: Vec<select::Candidate<'_>> = entries
            .iter()
            .map(|(id, e)| select::Candidate {
                id,
                status: e.dto.status,
                last_playing_at: e.last_playing_at,
            })
            .collect();
        current =
            select::select_source(&candidates, None, now, PAUSED_WINDOW_MS).map(str::to_owned);
        let now_playing = current
            .as_ref()
            .and_then(|id| entries.get(id))
            .map(|e| e.dto.clone());
        let mut sources: Vec<SourceSummary> = entries
            .iter()
            .map(|(id, e)| SourceSummary {
                source_id: id.clone(),
                app_name: e.dto.app_name.clone(),
                status: e.dto.status,
            })
            .collect();
        sources.sort_by(|a, b| a.source_id.cmp(&b.source_id));

        let snapshot = {
            let mut snap = handle
                .snapshot
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if snap.now_playing == now_playing && snap.sources.len() == sources.len() {
                continue;
            }
            *snap = MediaSnapshot {
                now_playing,
                sources,
                rev: snap.rev + 1,
            };
            snap.clone()
        };
        let level = match snapshot.now_playing.as_ref().map(|n| n.status) {
            Some(PlaybackStatus::Playing) => priority::MEDIA,
            Some(_) => priority::AMBIENT,
            None => priority::IDLE,
        };
        top_priority.send_if_modified(|v| std::mem::replace(v, level) != level);
        for label in ["island", "main"] {
            let _ = app.emit_to(label, events::MEDIA_NOW_PLAYING, &snapshot);
        }
    }
}

fn upsert(entries: &mut HashMap<String, Entry>, id: String, dto: NowPlayingDto, now: u64) {
    let previous_at = entries.get(&id).map_or(0, |e| e.last_playing_at);
    let was_playing = entries
        .get(&id)
        .is_some_and(|e| e.dto.status == PlaybackStatus::Playing);
    let last_playing_at = match dto.status {
        PlaybackStatus::Playing if !was_playing => now,
        PlaybackStatus::Playing => previous_at.max(1),
        // Remember when playback stopped so paused sources expire 30 min later.
        _ if was_playing => now,
        _ => previous_at,
    };
    entries.insert(
        id,
        Entry {
            dto,
            last_playing_at,
        },
    );
}

fn external_dto(id: &str, s: &SessionInfo, artwork: &ArtworkCache) -> NowPlayingDto {
    use crate::platform::gsmtc::PlaybackStatus as P;
    NowPlayingDto {
        source_id: id.to_owned(),
        app_name: app_name_from_aumid(&s.aumid),
        title: s.title.clone(),
        artist: s.artist.clone(),
        album: s.album.clone(),
        artwork_url: s.thumbnail.clone().map(|b| artwork.put(b)),
        status: match s.status {
            P::Playing => PlaybackStatus::Playing,
            P::Paused => PlaybackStatus::Paused,
            P::Stopped => PlaybackStatus::Stopped,
            P::Changing => PlaybackStatus::Changing,
        },
        position_ms: s.position_ms,
        position_at: s.position_at_ms,
        duration_ms: s.duration_ms,
        rate: s.rate,
        caps: MediaCaps {
            play_pause: s.caps.play_pause,
            next: s.caps.next,
            previous: s.caps.previous,
            seek: s.caps.seek,
        },
        is_ad: false,
    }
}

/// "Spotify.exe" → "Spotify", "Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic" → "ZuneMusic".
pub fn app_name_from_aumid(aumid: &str) -> String {
    let tail = aumid.rsplit('!').next().unwrap_or(aumid);
    let tail = tail
        .strip_suffix(".exe")
        .or_else(|| tail.strip_suffix(".EXE"))
        .unwrap_or(tail);
    let name = tail.rsplit(['.', '\\']).next().unwrap_or(tail);
    if name.is_empty() {
        aumid.to_owned()
    } else {
        name.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::app_name_from_aumid;

    #[test]
    fn readable_app_names() {
        assert_eq!(app_name_from_aumid("Spotify.exe"), "Spotify");
        assert_eq!(app_name_from_aumid("Chrome"), "Chrome");
        assert_eq!(
            app_name_from_aumid("Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic"),
            "ZuneMusic"
        );
    }
}
