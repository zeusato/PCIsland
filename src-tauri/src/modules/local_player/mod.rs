//! Local file player: library, queue, playback (docs/modules/local-player.md).
#![forbid(unsafe_code)]

pub mod commands;
pub mod engine;
pub mod library;
pub mod queue;
pub mod stream;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot, watch};
use ts_rs::TS;

use crate::core::artwork::ArtworkCache;
use crate::core::audio::AudioEngine;
use crate::core::events::{AppEvent, EventBus, MediaIntent};
use crate::core::settings::Settings;
use crate::core::storage::Db;
use crate::core::supervisor::{Shared, Supervisor, shared};
use crate::ipc::{events, now_ms};
use crate::modules::media::dto::{MediaCaps, NowPlayingDto, PlaybackStatus};
use crate::modules::media::{LOCAL_SOURCE_ID, MediaCommand, MediaHandle};
use engine::{EventSink, MusicCmd, MusicEvent};
use library::Track;
use queue::{Queue, RepeatMode};

const SESSION_KEY: &str = "local_player.session";

/// Carries engine events from the audio thread to the player service.
pub struct MusicEvents {
    tx: mpsc::UnboundedSender<MusicEvent>,
    rx: Mutex<Option<mpsc::UnboundedReceiver<MusicEvent>>>,
}

pub fn event_bridge() -> MusicEvents {
    let (tx, rx) = mpsc::unbounded_channel();
    MusicEvents {
        tx,
        rx: Mutex::new(Some(rx)),
    }
}

impl MusicEvents {
    pub fn sink(&self) -> EventSink {
        let tx = self.tx.clone();
        Arc::new(move |event| {
            let _ = tx.send(event);
        })
    }

    pub fn take_receiver(&self) -> Option<mpsc::UnboundedReceiver<MusicEvent>> {
        self.rx
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct LocalPlayerSnapshot {
    pub queue: Vec<Track>,
    pub index: Option<u32>,
    pub playing: bool,
    #[ts(type = "number")]
    pub position_ms: u64,
    #[ts(type = "number")]
    pub position_at: u64,
    #[ts(type = "number | null")]
    pub duration_ms: Option<u64>,
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    #[ts(type = "number")]
    pub rev: u64,
}

pub enum Cmd {
    PlayTracks(Vec<String>, usize),
    PlayFiles(Vec<PathBuf>),
    Enqueue(Vec<String>, bool),
    Control(MediaCommand),
    Play,
    Seek(u64),
    Volume(f32),
    Shuffle(bool),
    Repeat(RepeatMode),
    Stop,
    PlayStation(library::Station),
    Snapshot(oneshot::Sender<LocalPlayerSnapshot>),
}

#[derive(Clone)]
pub struct LocalPlayerHandle {
    tx: mpsc::UnboundedSender<Cmd>,
    pub db: Db,
}

impl LocalPlayerHandle {
    pub fn send(&self, cmd: Cmd) {
        let _ = self.tx.send(cmd);
    }
    pub async fn snapshot(&self) -> LocalPlayerSnapshot {
        let (tx, rx) = oneshot::channel();
        self.send(Cmd::Snapshot(tx));
        rx.await.unwrap_or_default()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Session {
    queue: Queue,
    position_ms: u64,
}

#[allow(clippy::too_many_arguments, reason = "one-off wiring of the service")]
pub fn setup(
    app: &AppHandle,
    db: Db,
    audio: AudioEngine,
    events_in: &MusicEvents,
    media: MediaHandle,
    artwork: Arc<ArtworkCache>,
    settings: watch::Receiver<Settings>,
    bus: EventBus,
    sup: &Supervisor,
) -> LocalPlayerHandle {
    let (tx, rx) = mpsc::unbounded_channel();
    let handle = LocalPlayerHandle {
        tx: tx.clone(),
        db: db.clone(),
    };
    let control_tx = tx.clone();
    media.register_control(
        LOCAL_SOURCE_ID,
        Arc::new(move |cmd| {
            let _ = control_tx.send(Cmd::Control(cmd));
        }),
    );
    // Focus mode can ask for a playlist through the EventBus.
    let for_intents = handle.clone();
    let mut intents = bus.subscribe();
    tauri::async_runtime::spawn(async move {
        while let Ok(event) = intents.recv().await {
            match event {
                AppEvent::MediaIntent(MediaIntent::PlayLocalPlaylist(id)) => {
                    match library::playlist_tracks(&for_intents.db, &id) {
                        Ok(ids) if !ids.is_empty() => for_intents.send(Cmd::PlayTracks(ids, 0)),
                        _ => {
                            tracing::warn!(module = "local_player", playlist = %id, "focus playlist missing or empty")
                        }
                    }
                }
                // Downloaded focus music loops for the whole phase (focus-sounds §3).
                AppEvent::MediaIntent(MediaIntent::PlayFiles { paths, shuffle }) => {
                    for_intents.send(Cmd::Shuffle(shuffle));
                    for_intents.send(Cmd::Repeat(RepeatMode::All));
                    for_intents.send(Cmd::PlayFiles(paths));
                }
                _ => {}
            }
        }
    });
    if let Some(engine_rx) = events_in.take_receiver() {
        let (rx, engine_rx) = (shared(rx), shared(engine_rx));
        let app = app.clone();
        sup.spawn("local_player", move || {
            let ctx = Ctx {
                db: db.clone(),
                audio: audio.clone(),
                media: media.clone(),
                artwork: artwork.clone(),
                settings: settings.clone(),
            };
            service(app.clone(), rx.clone(), engine_rx.clone(), ctx)
        });
    }
    handle
}

struct Ctx {
    db: Db,
    audio: AudioEngine,
    media: MediaHandle,
    artwork: Arc<ArtworkCache>,
    settings: watch::Receiver<Settings>,
}

struct State {
    queue: Queue,
    tracks: Vec<Track>,
    playing: bool,
    position_ms: u64,
    position_at: u64,
    duration_ms: Option<u64>,
    artwork_url: Option<String>,
    /// Set while an internet radio station plays instead of the queue.
    station: Option<library::Station>,
    rev: u64,
}

async fn service(app: AppHandle, rx: Shared<Cmd>, engine_rx: Shared<MusicEvent>, ctx: Ctx) {
    let volume = ctx.settings.borrow().local_player.volume;
    ctx.audio.music(MusicCmd::SetVolume(volume));
    let session: Session = ctx
        .db
        .kv_get(SESSION_KEY)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut s = State {
        tracks: session
            .queue
            .items
            .iter()
            .filter_map(|id| library::get(&ctx.db, id))
            .collect(),
        queue: session.queue,
        playing: false,
        position_ms: session.position_ms,
        position_at: now_ms(),
        duration_ms: None,
        artwork_url: None,
        station: None,
        rev: 0,
    };
    // Restore the last track paused (docs/modules/local-player.md §3, "resume_on_start").
    if ctx.settings.borrow().local_player.resume_on_start != "off" {
        let autoplay = ctx.settings.borrow().local_player.resume_on_start == "play";
        load_current(&ctx, &mut s, session.position_ms, autoplay);
    }
    let mut save_tick = tokio::time::interval(std::time::Duration::from_secs(30));
    loop {
        tokio::select! {
            Some(cmd) = async { rx.lock().await.recv().await } => match cmd {
                Cmd::PlayTracks(ids, start) => {
                    s.queue.set(ids, start);
                    refresh_tracks(&ctx, &mut s);
                    load_current(&ctx, &mut s, 0, true);
                }
                Cmd::PlayFiles(paths) => {
                    let ids: Vec<String> = paths.iter().filter(|p| library::is_audio(p)).filter_map(|p| library::track_for_file(&ctx.db, p).ok()).map(|t| t.id).collect();
                    if !ids.is_empty() {
                        s.queue.set(ids, 0);
                        refresh_tracks(&ctx, &mut s);
                        load_current(&ctx, &mut s, 0, true);
                    }
                }
                Cmd::Enqueue(ids, next) => {
                    let was_empty = s.queue.current.is_none();
                    s.queue.enqueue(ids, next);
                    refresh_tracks(&ctx, &mut s);
                    if was_empty {
                        load_current(&ctx, &mut s, 0, true);
                    }
                }
                Cmd::Control(MediaCommand::PlayPause) => {
                    ctx.audio.music(if s.playing { MusicCmd::Pause } else { MusicCmd::Play });
                }
                Cmd::Control(MediaCommand::Pause) => ctx.audio.music(MusicCmd::Pause),
                Cmd::Play => ctx.audio.music(MusicCmd::Play),
                Cmd::PlayStation(station) => {
                    ctx.audio.music(MusicCmd::LoadUrl { url: station.url.clone(), autoplay: true });
                    s.station = Some(station);
                    s.duration_ms = None;
                    s.position_ms = 0;
                    s.position_at = now_ms();
                    s.artwork_url = None;
                }
                Cmd::Control(MediaCommand::Next | MediaCommand::Previous) if s.station.is_some() => {}
                Cmd::Control(MediaCommand::Next) => {
                    if s.queue.next(false).is_some() {
                        load_current(&ctx, &mut s, 0, true);
                    }
                }
                Cmd::Control(MediaCommand::Previous) => {
                    let pos = position(&s);
                    if pos > 3_000 {
                        ctx.audio.music(MusicCmd::Seek(0));
                    } else if s.queue.previous().is_some() {
                        load_current(&ctx, &mut s, 0, true);
                    }
                }
                Cmd::Seek(ms) => ctx.audio.music(MusicCmd::Seek(ms)),
                Cmd::Volume(v) => {
                    ctx.audio.music(MusicCmd::SetVolume(v));
                }
                Cmd::Shuffle(on) => s.queue.set_shuffle(on),
                Cmd::Repeat(mode) => s.queue.repeat = mode,
                Cmd::Stop => {
                    ctx.audio.music(MusicCmd::Stop);
                    s.playing = false;
                    s.queue = Queue::default();
                    s.tracks.clear();
                }
                Cmd::Snapshot(reply) => {
                    let _ = reply.send(snapshot(&ctx, &s));
                    continue;
                }
            },
            Some(event) = async { engine_rx.lock().await.recv().await } => match event {
                MusicEvent::Loaded { duration_ms, playing, position_ms, .. } => {
                    s.duration_ms = duration_ms;
                    s.playing = playing;
                    s.position_ms = position_ms;
                    s.position_at = now_ms();
                }
                MusicEvent::State { playing, position_ms } => {
                    s.playing = playing;
                    s.position_ms = position_ms;
                    s.position_at = now_ms();
                }
                MusicEvent::Ended if s.station.is_some() => {
                    s.playing = false;
                }
                MusicEvent::Ended => {
                    if s.queue.next(true).is_some() {
                        load_current(&ctx, &mut s, 0, true);
                    } else {
                        s.playing = false;
                        s.position_ms = 0;
                    }
                }
                MusicEvent::Failed { path, message } => {
                    tracing::warn!(module = "local_player", path = %path.display(), %message, "cannot play file");
                    if s.queue.next(false).is_some() {
                        load_current(&ctx, &mut s, 0, true);
                    }
                }
            },
            _ = save_tick.tick() => {
                save_session(&ctx, &s);
                continue;
            }
            else => return,
        }
        s.rev += 1;
        publish(&app, &ctx, &s);
        save_session(&ctx, &s);
    }
}

fn refresh_tracks(ctx: &Ctx, s: &mut State) {
    s.tracks = s
        .queue
        .items
        .iter()
        .filter_map(|id| library::get(&ctx.db, id))
        .collect();
}

fn load_current(ctx: &Ctx, s: &mut State, start_ms: u64, autoplay: bool) {
    s.station = None;
    let Some(track) = s
        .queue
        .current_item()
        .and_then(|id| library::get(&ctx.db, id))
    else {
        return;
    };
    s.duration_ms = track.duration_ms;
    s.position_ms = start_ms;
    s.position_at = now_ms();
    s.artwork_url = library::read_artwork(std::path::Path::new(&track.path))
        .map(|bytes| ctx.artwork.put(Arc::new(bytes)));
    ctx.audio.music(MusicCmd::Load {
        path: PathBuf::from(&track.path),
        start_ms,
        autoplay,
    });
}

fn position(s: &State) -> u64 {
    if s.playing {
        s.position_ms + now_ms().saturating_sub(s.position_at)
    } else {
        s.position_ms
    }
}

fn snapshot(ctx: &Ctx, s: &State) -> LocalPlayerSnapshot {
    LocalPlayerSnapshot {
        queue: s.tracks.clone(),
        index: s.queue.current.map(|i| i as u32),
        playing: s.playing,
        position_ms: s.position_ms,
        position_at: s.position_at,
        duration_ms: s.duration_ms,
        volume: ctx.settings.borrow().local_player.volume,
        shuffle: s.queue.shuffle,
        repeat: s.queue.repeat,
        rev: s.rev,
    }
}

fn publish(app: &AppHandle, ctx: &Ctx, s: &State) {
    let current = s
        .queue
        .current_item()
        .and_then(|id| s.tracks.iter().find(|t| t.id == id));
    let station = s.station.as_ref().map(|st| NowPlayingDto {
        source_id: LOCAL_SOURCE_ID.into(),
        app_name: "Radio".into(),
        title: st.name.clone(),
        artist: None,
        album: None,
        artwork_url: None,
        status: if s.playing {
            PlaybackStatus::Playing
        } else {
            PlaybackStatus::Paused
        },
        position_ms: None,
        position_at: s.position_at,
        duration_ms: None,
        rate: 1.0,
        caps: MediaCaps {
            play_pause: true,
            next: false,
            previous: false,
            seek: false,
        },
        is_ad: false,
    });
    let np = station.or_else(|| {
        current.map(|t| NowPlayingDto {
            source_id: LOCAL_SOURCE_ID.into(),
            app_name: "PCisland".into(),
            title: t.title.clone(),
            artist: t.artist.clone(),
            album: t.album.clone(),
            artwork_url: s.artwork_url.clone(),
            status: if s.playing {
                PlaybackStatus::Playing
            } else {
                PlaybackStatus::Paused
            },
            position_ms: Some(s.position_ms),
            position_at: s.position_at,
            duration_ms: s.duration_ms,
            rate: 1.0,
            caps: MediaCaps {
                play_pause: true,
                next: true,
                previous: true,
                seek: true,
            },
            is_ad: false,
        })
    });
    ctx.media.update_source(LOCAL_SOURCE_ID, np);
    let snap = snapshot(ctx, s);
    for label in ["island", "main"] {
        let _ = app.emit_to(label, events::LOCAL_PLAYER_STATE, &snap);
    }
}

fn save_session(ctx: &Ctx, s: &State) {
    let session = Session {
        queue: s.queue.clone(),
        position_ms: position(s),
    };
    if let Ok(json) = serde_json::to_string(&session) {
        let _ = ctx.db.kv_set(SESSION_KEY, &json);
    }
}
