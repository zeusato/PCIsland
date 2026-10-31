//! Local file player: library, queue, playback (docs/modules/local-player.md).
#![forbid(unsafe_code)]

pub mod commands;
pub mod engine;
pub mod library;
pub mod queue;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot, watch};
use ts_rs::TS;

use crate::core::artwork::ArtworkCache;
use crate::core::audio::AudioEngine;
use crate::core::settings::Settings;
use crate::core::storage::Db;
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

pub fn setup(
    app: &AppHandle,
    db: Db,
    audio: AudioEngine,
    events_in: &MusicEvents,
    media: MediaHandle,
    artwork: Arc<ArtworkCache>,
    settings: watch::Receiver<Settings>,
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
    if let Some(engine_rx) = events_in.take_receiver() {
        tauri::async_runtime::spawn(service(
            app.clone(),
            rx,
            engine_rx,
            Ctx {
                db,
                audio,
                media,
                artwork,
                settings,
            },
        ));
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
    rev: u64,
}

async fn service(
    app: AppHandle,
    mut rx: mpsc::UnboundedReceiver<Cmd>,
    mut engine_rx: mpsc::UnboundedReceiver<MusicEvent>,
    ctx: Ctx,
) {
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
            Some(cmd) = rx.recv() => match cmd {
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
            Some(event) = engine_rx.recv() => match event {
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
    let np = current.map(|t| NowPlayingDto {
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
