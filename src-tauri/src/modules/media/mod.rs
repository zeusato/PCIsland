//! Media hub: merges every source into one `NowPlaying` (docs/modules/media-hub.md).
//! Sources: external apps via GSMTC, the web player and the local player.
#![forbid(unsafe_code)]

pub mod commands;
pub mod dto;
pub mod select;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, watch};

use crate::core::activity::{Activity, ActivityAction, ActivityHub};
use crate::core::artwork::ArtworkCache;
use crate::core::events::{AppEvent, EventBus, MediaIntent};
use crate::core::priority;
use crate::core::settings::Settings;
use crate::core::supervisor::{Shared, Supervisor, shared};
use crate::ipc::{events, now_ms};
use crate::platform::gsmtc::{self, GsmtcCommand, SessionInfo};
use dto::{MediaCaps, MediaSnapshot, NowPlayingDto, PlaybackStatus, SourceSummary};

/// `activity.paused_media_ambient_min` default (docs/modules/activity-hub.md §7).
const PAUSED_WINDOW_MS: u64 = 30 * 60 * 1000;
pub const WEB_SOURCE_ID: &str = "web";
pub const LOCAL_SOURCE_ID: &str = "local";
const ACTIVITY_ID: &str = "media:now-playing";

#[derive(Debug, Clone, Copy)]
pub enum MediaCommand {
    PlayPause,
    Next,
    Previous,
    Pause,
}

/// Control callback of an in-app source (the `MediaSource` idea of
/// docs/02-system-design.md §11): web player and local player register one.
pub type SourceControl = Arc<dyn Fn(MediaCommand) + Send + Sync>;

enum Input {
    External(Vec<SessionInfo>),
    Source(String, Option<NowPlayingDto>),
    Command(MediaCommand),
    Pin(Option<String>),
}

struct Entry {
    dto: NowPlayingDto,
    last_playing_at: u64,
}

#[derive(Clone)]
pub struct MediaHandle {
    input: mpsc::UnboundedSender<Input>,
    snapshot: Arc<Mutex<MediaSnapshot>>,
    controls: Arc<Mutex<HashMap<String, SourceControl>>>,
    smtc: Arc<Mutex<Option<crate::platform::smtc::AppSmtc>>>,
}

impl MediaHandle {
    pub fn snapshot(&self) -> MediaSnapshot {
        self.snapshot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
    /// Registers PCisland's own SMTC on the island window: media keys then reach the
    /// local and web players. External apps keep their own SMTC sessions.
    pub fn attach_smtc(&self, window: crate::platform::window::RawHwnd) {
        use crate::platform::smtc::{AppSmtc, Button};
        let for_keys = self.clone();
        match AppSmtc::new(window, move |b| {
            for_keys.command(match b {
                Button::PlayPause => MediaCommand::PlayPause,
                Button::Next => MediaCommand::Next,
                Button::Previous => MediaCommand::Previous,
            });
        }) {
            Ok(smtc) => *self.smtc.lock().unwrap_or_else(PoisonError::into_inner) = Some(smtc),
            Err(err) => {
                tracing::warn!(module = "media", error = %err, "app SMTC unavailable; media keys only reach external apps")
            }
        }
    }

    fn update_smtc(&self, np: Option<&NowPlayingDto>) {
        let guard = self.smtc.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(smtc) = guard.as_ref() else { return };
        let own = np.filter(|n| n.source_id == LOCAL_SOURCE_ID || n.source_id == WEB_SOURCE_ID);
        let _ = smtc.update(own.map(|n| {
            (
                n.title.as_str(),
                n.artist.as_deref(),
                n.status == PlaybackStatus::Playing,
            )
        }));
    }

    pub fn command(&self, cmd: MediaCommand) {
        let _ = self.input.send(Input::Command(cmd));
    }
    pub fn pin_source(&self, source_id: Option<String>) {
        let _ = self.input.send(Input::Pin(source_id));
    }
    /// Called by in-app sources whenever their state changes (`None` = gone).
    pub fn update_source(&self, source_id: &str, state: Option<NowPlayingDto>) {
        let _ = self.input.send(Input::Source(source_id.into(), state));
    }
    pub fn update_web_source(&self, state: Option<NowPlayingDto>) {
        self.update_source(WEB_SOURCE_ID, state);
    }
    pub fn register_control(&self, source_id: &str, control: SourceControl) {
        self.controls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(source_id.into(), control);
    }
    pub fn register_web_control(&self, control: SourceControl) {
        self.register_control(WEB_SOURCE_ID, control);
    }
}

pub fn setup(
    app: &AppHandle,
    artwork: Arc<ArtworkCache>,
    hub: ActivityHub,
    settings: watch::Receiver<Settings>,
    bus: EventBus,
    sup: &Supervisor,
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
        controls: Arc::new(Mutex::new(HashMap::new())),
        smtc: Arc::new(Mutex::new(None)),
    };
    let for_actions = handle.clone();
    hub.register_handler(
        "media",
        Arc::new(move |_, action| {
            let cmd = match action {
                "play_pause" => MediaCommand::PlayPause,
                "next" => MediaCommand::Next,
                "previous" => MediaCommand::Previous,
                other => return Err(format!("unknown media action `{other}`")),
            };
            for_actions.command(cmd);
            Ok(())
        }),
    );
    // Focus mode asks media to pause through the EventBus.
    let for_intents = handle.clone();
    let mut intents = bus.subscribe();
    tauri::async_runtime::spawn(async move {
        while let Ok(event) = intents.recv().await {
            if let AppEvent::MediaIntent(MediaIntent::Pause) = event {
                for_intents.command(MediaCommand::Pause);
            }
        }
    });
    let rx = shared(rx);
    let app = app.clone();
    let for_service = handle.clone();
    sup.spawn("media", move || {
        service(
            app.clone(),
            for_service.clone(),
            rx.clone(),
            gsmtc_tx.clone(),
            artwork.clone(),
            hub.clone(),
            settings.clone(),
            bus.clone(),
        )
    });
    handle
}

#[allow(
    clippy::too_many_arguments,
    reason = "one-off wiring of the service task"
)]
async fn service(
    app: AppHandle,
    handle: MediaHandle,
    rx: Shared<Input>,
    gsmtc_tx: std::sync::mpsc::Sender<GsmtcCommand>,
    artwork: Arc<ArtworkCache>,
    hub: ActivityHub,
    settings: watch::Receiver<Settings>,
    bus: EventBus,
) {
    let own_exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()))
        .unwrap_or_default();
    let mut entries: HashMap<String, Entry> = HashMap::new();
    let mut current: Option<String> = None;
    let mut pinned: Option<String> = None;

    while let Some(input) = rx.lock().await.recv().await {
        let now = now_ms();
        match input {
            Input::External(sessions) => {
                entries.retain(|id, _| !id.starts_with("external:"));
                if !settings.borrow().media.external_enabled {
                    // Fall through so the snapshot drops external sources.
                } else {
                    for s in sessions {
                        let lower = s.aumid.to_lowercase();
                        // Our own SMTC session (WebView2 / own player) is not an external source.
                        if lower.contains("pcisland")
                            || (!own_exe.is_empty() && lower.contains(&own_exe))
                        {
                            continue;
                        }
                        let id = format!("external:{}", s.aumid);
                        let dto = external_dto(&id, &s, &artwork);
                        upsert(&mut entries, id, dto, now);
                    }
                }
            }
            Input::Source(id, Some(dto)) => upsert(&mut entries, id, dto, now),
            Input::Source(id, None) => {
                entries.remove(&id);
            }
            Input::Pin(id) => pinned = id,
            Input::Command(cmd) => {
                if let Some(id) = current.clone() {
                    dispatch(&handle, &gsmtc_tx, &id, cmd);
                }
                continue;
            }
        }

        // Our web player also shows up in GSMTC as a generic WebView2 session; while the
        // bridge reports it as `web`, that duplicate is hidden.
        let web_active = entries.contains_key(WEB_SOURCE_ID);
        let visible = |id: &str| !(web_active && is_webview2_session(id));
        let candidates: Vec<select::Candidate<'_>> = entries
            .iter()
            .filter(|(id, _)| visible(id))
            .map(|(id, e)| select::Candidate {
                id,
                status: e.dto.status,
                last_playing_at: e.last_playing_at,
            })
            .collect();
        current = select::select_source(&candidates, pinned.as_deref(), now, PAUSED_WINDOW_MS)
            .map(str::to_owned);
        let now_playing = current
            .as_ref()
            .and_then(|id| entries.get(id))
            .map(|e| e.dto.clone());
        let mut sources: Vec<SourceSummary> = entries
            .iter()
            .filter(|(id, _)| visible(id))
            .map(|(id, e)| SourceSummary {
                source_id: id.clone(),
                app_name: e.dto.app_name.clone(),
                status: e.dto.status,
            })
            .collect();
        sources.sort_by(|a, b| a.source_id.cmp(&b.source_id));

        let (snapshot, previous) = {
            let mut snap = handle
                .snapshot
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if snap.now_playing == now_playing && snap.sources.len() == sources.len() {
                continue;
            }
            let previous = snap.now_playing.clone();
            *snap = MediaSnapshot {
                now_playing,
                sources,
                rev: snap.rev + 1,
            };
            (snap.clone(), previous)
        };
        bus.send(AppEvent::NowPlayingChanged(snapshot.now_playing.clone()));
        handle.update_smtc(snapshot.now_playing.as_ref());
        publish_activity(
            &hub,
            &snapshot,
            previous.as_ref(),
            settings.borrow().media.alert_on_track_change,
        );
        for label in ["island", "main"] {
            let _ = app.emit_to(label, events::MEDIA_NOW_PLAYING, &snapshot);
        }
    }
}

fn dispatch(
    handle: &MediaHandle,
    gsmtc_tx: &std::sync::mpsc::Sender<GsmtcCommand>,
    id: &str,
    cmd: MediaCommand,
) {
    if let Some(aumid) = id.strip_prefix("external:") {
        let aumid = aumid.to_owned();
        let _ = gsmtc_tx.send(match cmd {
            MediaCommand::PlayPause | MediaCommand::Pause => GsmtcCommand::TogglePlayPause(aumid),
            MediaCommand::Next => GsmtcCommand::Next(aumid),
            MediaCommand::Previous => GsmtcCommand::Previous(aumid),
        });
        return;
    }
    let control = handle
        .controls
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(id)
        .cloned();
    if let Some(control) = control {
        control(cmd);
    }
}

fn publish_activity(
    hub: &ActivityHub,
    snap: &MediaSnapshot,
    previous: Option<&NowPlayingDto>,
    alert_on_change: bool,
) {
    let Some(np) = &snap.now_playing else {
        hub.remove(ACTIVITY_ID);
        return;
    };
    let level = if np.status == PlaybackStatus::Playing {
        priority::MEDIA
    } else {
        priority::AMBIENT
    };
    let payload = serde_json::to_value(np).unwrap_or_default();
    hub.upsert(
        Activity::new(
            "media",
            ACTIVITY_ID,
            "media.now_playing",
            level,
            payload.clone(),
        )
        .with_actions(vec![
            ActivityAction::new("previous", "skip-back", "media.peek.previous"),
            ActivityAction::new("play_pause", "play", "media.peek.playPause").primary(),
            ActivityAction::new("next", "skip-forward", "media.peek.next"),
        ]),
    );
    let changed = previous.is_some_and(|p| p.title != np.title || p.source_id != np.source_id);
    if alert_on_change && changed && np.status == PlaybackStatus::Playing && !np.is_ad {
        hub.alert(
            Activity::new(
                "media",
                "media:track-changed",
                "media.track_changed",
                priority::ALERT,
                payload,
            )
            .transient(2_500),
        );
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

/// GSMTC id of a session hosted by any WebView2 process (ours included).
fn is_webview2_session(id: &str) -> bool {
    id.strip_prefix("external:")
        .is_some_and(|aumid| aumid.to_lowercase().contains("msedgewebview2"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn detects_webview2_sessions() {
        assert!(super::is_webview2_session("external:msedgewebview2.exe"));
        assert!(!super::is_webview2_session("external:Spotify.exe"));
        assert!(!super::is_webview2_session("web"));
    }

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
