//! Reads and controls media sessions of other apps through
//! `GlobalSystemMediaTransportControlsSessionManager` (docs/modules/media-hub.md §5).
//!
//! Runs on its own `gsmtc` thread (WinRT, MTA). GSMTC events only mark the state dirty;
//! after 150 ms of quiet the thread re-reads every session and hands a full snapshot to
//! the caller. This keeps event storms cheap and the logic simple.

use std::collections::HashMap;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::Duration;

use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Storage::Streams::DataReader;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

/// Per session: (track key, thumbnail bytes) so thumbnails are only re-read on track change.
type ThumbnailCache = HashMap<String, (String, Option<std::sync::Arc<Vec<u8>>>)>;

const DEBOUNCE: Duration = Duration::from_millis(150);
const MAX_THUMBNAIL_BYTES: u32 = 2 * 1024 * 1024;
/// 100 ns ticks between 1601-01-01 and 1970-01-01.
const FILETIME_UNIX_OFFSET: i64 = 116_444_736_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
    Changing,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SessionCaps {
    pub play_pause: bool,
    pub next: bool,
    pub previous: bool,
    pub seek: bool,
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub aumid: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub status: PlaybackStatus,
    pub position_ms: Option<u64>,
    pub position_at_ms: u64,
    pub duration_ms: Option<u64>,
    pub rate: f32,
    pub caps: SessionCaps,
    /// Raw thumbnail bytes; only re-read when the track changes.
    pub thumbnail: Option<std::sync::Arc<Vec<u8>>>,
}

#[derive(Debug, Clone)]
pub enum GsmtcCommand {
    TogglePlayPause(String),
    Next(String),
    Previous(String),
}

enum Msg {
    Dirty,
    Command(GsmtcCommand),
}

/// Starts the `gsmtc` thread. `on_snapshot` receives every session (unfiltered) after
/// each change. Returns a sender for control commands.
pub fn spawn<F>(on_snapshot: F) -> Sender<GsmtcCommand>
where
    F: Fn(Vec<SessionInfo>) + Send + 'static,
{
    let (cmd_tx, cmd_rx) = mpsc::channel::<GsmtcCommand>();
    let (tx, rx) = mpsc::channel::<Msg>();
    let forward = tx.clone();
    std::thread::Builder::new()
        .name("gsmtc-cmd".into())
        .spawn(move || {
            while let Ok(cmd) = cmd_rx.recv() {
                if forward.send(Msg::Command(cmd)).is_err() {
                    break;
                }
            }
        })
        .expect("spawn gsmtc-cmd thread");
    std::thread::Builder::new()
        .name("gsmtc".into())
        .spawn(move || {
            if let Err(err) = run(&tx, &rx, &on_snapshot) {
                tracing::warn!(module = "media", error = %err, "GSMTC unavailable; external sources disabled");
            }
        })
        .expect("spawn gsmtc thread");
    cmd_tx
}

struct Watched {
    session: Session,
    tokens: [i64; 3],
}

impl Drop for Watched {
    fn drop(&mut self) {
        let [props, playback, timeline] = self.tokens;
        let _ = self.session.RemoveMediaPropertiesChanged(props);
        let _ = self.session.RemovePlaybackInfoChanged(playback);
        let _ = self.session.RemoveTimelinePropertiesChanged(timeline);
    }
}

fn run<F: Fn(Vec<SessionInfo>)>(
    tx: &Sender<Msg>,
    rx: &mpsc::Receiver<Msg>,
    on_snapshot: &F,
) -> windows::core::Result<()> {
    // SAFETY: initializes COM for this dedicated thread only; never uninitialized
    // because the thread lives as long as the app.
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.ok()?;
    let manager = Manager::RequestAsync()?.join()?;

    let dirty = tx.clone();
    manager.SessionsChanged(&TypedEventHandler::new(move |_, _| {
        let _ = dirty.send(Msg::Dirty);
        Ok(())
    }))?;

    let mut watched: HashMap<String, Watched> = HashMap::new();
    let mut thumbnails = ThumbnailCache::new();
    let mut is_dirty = true;

    loop {
        match rx.recv_timeout(DEBOUNCE) {
            Ok(Msg::Dirty) => is_dirty = true,
            Ok(Msg::Command(cmd)) => {
                if let Err(err) = execute(&manager, &cmd) {
                    tracing::debug!(module = "media", error = %err, ?cmd, "GSMTC command failed");
                }
                is_dirty = true;
            }
            Err(RecvTimeoutError::Timeout) if is_dirty => {
                is_dirty = false;
                let sessions = manager.GetSessions()?;
                let mut infos = Vec::new();
                let mut alive = Vec::new();
                for session in &sessions {
                    let Ok(aumid) = session.SourceAppUserModelId().map(|s| s.to_string()) else {
                        continue;
                    };
                    if !watched.contains_key(&aumid) {
                        if let Ok(w) = watch(&session, tx) {
                            watched.insert(aumid.clone(), w);
                        }
                    }
                    alive.push(aumid.clone());
                    match read_session(&session, &aumid, &mut thumbnails) {
                        Ok(info) => infos.push(info),
                        Err(err) => {
                            tracing::debug!(module = "media", %aumid, error = %err, "read session failed")
                        }
                    }
                }
                watched.retain(|k, _| alive.contains(k));
                thumbnails.retain(|k, _| alive.contains(k));
                on_snapshot(infos);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
}

fn watch(session: &Session, tx: &Sender<Msg>) -> windows::core::Result<Watched> {
    let (t1, t2, t3) = (tx.clone(), tx.clone(), tx.clone());
    let props = session.MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| {
        let _ = t1.send(Msg::Dirty);
        Ok(())
    }))?;
    let playback = session.PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
        let _ = t2.send(Msg::Dirty);
        Ok(())
    }))?;
    let timeline = session.TimelinePropertiesChanged(&TypedEventHandler::new(move |_, _| {
        let _ = t3.send(Msg::Dirty);
        Ok(())
    }))?;
    Ok(Watched {
        session: session.clone(),
        tokens: [props, playback, timeline],
    })
}

fn read_session(
    session: &Session,
    aumid: &str,
    thumbnails: &mut ThumbnailCache,
) -> windows::core::Result<SessionInfo> {
    let props = session.TryGetMediaPropertiesAsync()?.join()?;
    let title = props.Title()?.to_string();
    let artist = non_empty(props.Artist()?.to_string());
    let album = non_empty(props.AlbumTitle()?.to_string());

    let playback = session.GetPlaybackInfo()?;
    let status = match playback.PlaybackStatus()? {
        Status::Playing => PlaybackStatus::Playing,
        Status::Paused => PlaybackStatus::Paused,
        Status::Changing | Status::Opened => PlaybackStatus::Changing,
        _ => PlaybackStatus::Stopped,
    };
    let rate = playback
        .PlaybackRate()
        .and_then(|r| r.Value())
        .map_or(1.0, |r| r as f32);
    let controls = playback.Controls()?;
    let caps = SessionCaps {
        play_pause: controls.IsPlayPauseToggleEnabled()?,
        next: controls.IsNextEnabled()?,
        previous: controls.IsPreviousEnabled()?,
        seek: controls.IsPlaybackPositionEnabled()?,
    };

    let timeline = session.GetTimelineProperties()?;
    let end = timeline.EndTime()?.Duration - timeline.StartTime()?.Duration;
    let duration_ms = (end > 0).then(|| ticks_to_ms(end));
    let position = ticks_to_ms(timeline.Position()?.Duration);
    let position_ms = duration_ms.map(|_| position);
    let position_at_ms = filetime_to_unix_ms(timeline.LastUpdatedTime()?.UniversalTime);

    let key = format!("{title}\u{1f}{}", artist.as_deref().unwrap_or_default());
    let thumbnail = match thumbnails.get(aumid) {
        Some((cached_key, bytes)) if *cached_key == key && bytes.is_some() => bytes.clone(),
        _ => {
            let bytes = read_thumbnail(&props)
                .ok()
                .flatten()
                .map(std::sync::Arc::new);
            thumbnails.insert(aumid.to_owned(), (key, bytes.clone()));
            bytes
        }
    };

    Ok(SessionInfo {
        aumid: aumid.to_owned(),
        title,
        artist,
        album,
        status,
        position_ms,
        position_at_ms,
        duration_ms,
        rate,
        caps,
        thumbnail,
    })
}

fn read_thumbnail(
    props: &windows::Media::Control::GlobalSystemMediaTransportControlsSessionMediaProperties,
) -> windows::core::Result<Option<Vec<u8>>> {
    let Ok(reference) = props.Thumbnail() else {
        return Ok(None);
    };
    let stream = reference.OpenReadAsync()?.join()?;
    let size = u32::try_from(stream.Size()?).unwrap_or(u32::MAX);
    if size == 0 || size > MAX_THUMBNAIL_BYTES {
        return Ok(None);
    }
    let reader = DataReader::CreateDataReader(&stream)?;
    reader.LoadAsync(size)?.join()?;
    let mut buf = vec![0u8; size as usize];
    reader.ReadBytes(&mut buf)?;
    Ok(Some(buf))
}

fn execute(manager: &Manager, cmd: &GsmtcCommand) -> windows::core::Result<()> {
    let target = match cmd {
        GsmtcCommand::TogglePlayPause(a) | GsmtcCommand::Next(a) | GsmtcCommand::Previous(a) => a,
    };
    for session in &manager.GetSessions()? {
        if session.SourceAppUserModelId()? != *target {
            continue;
        }
        let op = match cmd {
            GsmtcCommand::TogglePlayPause(_) => session.TryTogglePlayPauseAsync()?,
            GsmtcCommand::Next(_) => session.TrySkipNextAsync()?,
            GsmtcCommand::Previous(_) => session.TrySkipPreviousAsync()?,
        };
        op.join()?;
        break;
    }
    Ok(())
}

fn non_empty(s: String) -> Option<String> {
    (!s.trim().is_empty()).then_some(s)
}

fn ticks_to_ms(ticks: i64) -> u64 {
    u64::try_from(ticks / 10_000).unwrap_or(0)
}

fn filetime_to_unix_ms(ticks: i64) -> u64 {
    u64::try_from((ticks - FILETIME_UNIX_OFFSET) / 10_000).unwrap_or(0)
}
