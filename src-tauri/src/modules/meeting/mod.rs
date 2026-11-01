//! Meeting capture is independent of every window and media player.
#![forbid(unsafe_code)]
pub mod codec;
pub mod playback;
mod transcript;
use crate::{
    core::{id::new_id, storage::Db},
    ipc::{IpcError, IpcResult, now_ms},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError, mpsc},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, State};
use ts_rs::TS;

#[derive(Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Highlight {
    pub id: String,
    #[ts(type = "number")]
    pub start_ms: u64,
    #[ts(type = "number | null")]
    pub end_ms: Option<u64>,
    pub file: String,
    pub status: String,
    pub transcript: String,
    pub edited_text: Option<String>,
    pub transcript_status: String,
    pub attempts: u32,
    #[ts(type = "number")]
    pub retry_at: u64,
    pub error: String,
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Meeting {
    pub id: String,
    pub title: String,
    #[ts(type = "number")]
    pub started_at: u64,
    #[ts(type = "number")]
    pub duration_ms: u64,
    pub status: String,
    pub folder: String,
    pub full_file: String,
    pub notes: String,
    #[ts(type = "number")]
    pub note_revision: u64,
    #[serde(default)]
    pub mic_device: Option<String>,
    #[serde(default)]
    pub system_device: Option<String>,
    pub mic: bool,
    pub system: bool,
    pub transcribe: bool,
    pub highlights: Vec<Highlight>,
    pub error: String,
}
#[derive(Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSnapshot {
    pub meetings: Vec<Meeting>,
    pub active: Option<String>,
    pub mic_level: f32,
    pub system_level: f32,
    pub key_configured: bool,
}
enum Command {
    Stop,
    Mark,
}
struct Inner {
    meetings: Vec<Meeting>,
    active: Option<(String, mpsc::SyncSender<Command>)>,
    mic_level: f32,
    system_level: f32,
    last_mark: Option<Instant>,
}
#[derive(Clone)]
pub struct MeetingHandle {
    inner: Arc<Mutex<Inner>>,
    db: Db,
    app: AppHandle,
    root: PathBuf,
    hub: crate::core::activity::ActivityHub,
    pub(super) wake: Arc<tokio::sync::Notify>,
}
pub(super) fn error(e: impl std::fmt::Display) -> IpcError {
    IpcError::new("meeting.failed", e.to_string())
}
impl MeetingHandle {
    fn snapshot(&self) -> MeetingSnapshot {
        let s = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        MeetingSnapshot {
            meetings: s.meetings.clone(),
            active: s.active.as_ref().map(|a| a.0.clone()),
            mic_level: s.mic_level,
            system_level: s.system_level,
            key_configured: !transcript::KEY.is_empty(),
        }
    }
    fn publish(&self) {
        let snapshot = self.snapshot();
        if let Some(m) = snapshot
            .meetings
            .iter()
            .find(|m| Some(&m.id) == snapshot.active.as_ref())
        {
            let mut activity = crate::core::activity::Activity::new(
                "meeting",
                "meeting:recording",
                "meeting.recording",
                70,
                serde_json::json!({"title":m.title}),
            )
            .secondary_only();
            activity.started_at = m.started_at;
            self.hub.upsert(activity);
        } else {
            self.hub.remove("meeting:recording");
        }
        for label in ["main", "island"] {
            let _ = self.app.emit_to(label, "meeting://state", &snapshot);
        }
    }
    pub(super) fn update(&self, id: &str, change: impl FnOnce(&mut Meeting)) -> Result<(), String> {
        let mut s = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let meeting = s
            .meetings
            .iter_mut()
            .find(|m| m.id == id)
            .ok_or("Meeting missing")?;
        let mut next = meeting.clone();
        change(&mut next);
        self.db
            .kv_set(
                &format!("meeting.session.{id}"),
                &serde_json::to_string(&next).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        *meeting = next;
        drop(s);
        self.publish();
        Ok(())
    }
    fn get(&self, id: &str) -> Result<Meeting, String> {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .meetings
            .iter()
            .find(|m| m.id == id)
            .cloned()
            .ok_or("Meeting missing".into())
    }
    pub fn mark(&self) -> Result<(), String> {
        let mut s = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        if s.last_mark
            .is_some_and(|t| t.elapsed() < Duration::from_millis(500))
        {
            return Ok(());
        }
        let tx = &s.active.as_ref().ok_or("No active meeting")?.1;
        tx.try_send(Command::Mark).map_err(|e| e.to_string())?;
        s.last_mark = Some(Instant::now());
        Ok(())
    }
    fn export(&self, id: String, clip: Option<String>) {
        let handle = self.clone();
        std::thread::spawn(move || {
            let Ok(m) = handle.get(&id) else { return };
            let journal = PathBuf::from(&m.folder).join("audio.packets");
            let (start, end, dest) = if let Some(ref cid) = clip {
                let Some(c) = m.highlights.iter().find(|c| &c.id == cid) else {
                    return;
                };
                (
                    c.start_ms,
                    c.end_ms.unwrap_or(m.duration_ms),
                    PathBuf::from(&c.file),
                )
            } else {
                (
                    0,
                    m.duration_ms,
                    PathBuf::from(&m.folder).join("meeting.opus"),
                )
            };
            let result = codec::export(&journal, &dest, start, end);
            let _ = handle.update(&id, |m| {
                if let Some(ref cid) = clip {
                    if let Some(c) = m.highlights.iter_mut().find(|c| &c.id == cid) {
                        c.status = if result.is_ok() { "ready" } else { "failed" }.into();
                        c.error = result.as_ref().err().cloned().unwrap_or_default();
                        if result.is_ok() && m.transcribe {
                            c.transcript_status = "queued".into();
                        }
                    }
                } else {
                    m.status = if result.is_ok() { "complete" } else { "failed" }.into();
                    if result.is_ok() {
                        m.full_file = dest.to_string_lossy().into_owned();
                    } else {
                        m.error = result.as_ref().err().cloned().unwrap_or_default();
                    }
                }
            });
            handle.wake.notify_one();
            if handle.get(&id).is_ok_and(|m| {
                m.status == "complete" && m.highlights.iter().all(|c| c.status == "ready")
            }) {
                let _ = std::fs::remove_file(journal);
            }
        });
    }
}
pub fn setup(
    app: &AppHandle,
    db: Db,
    root: PathBuf,
    hub: crate::core::activity::ActivityHub,
) -> MeetingHandle {
    let meetings: Vec<Meeting> = db
        .call(|c| {
            let mut stmt = c.prepare("SELECT value FROM kv WHERE key LIKE 'meeting.session.%'")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            Ok(rows
                .filter_map(Result::ok)
                .filter_map(|s| serde_json::from_str(&s).ok())
                .collect())
        })
        .unwrap_or_default();
    let h = MeetingHandle {
        inner: Arc::new(Mutex::new(Inner {
            meetings,
            active: None,
            mic_level: 0.0,
            system_level: 0.0,
            last_mark: None,
        })),
        db,
        app: app.clone(),
        root,
        hub,
        wake: Arc::default(),
    };
    let recovery = h.clone();
    std::thread::spawn(move || {
        for m in recovery.snapshot().meetings {
            if matches!(m.status.as_str(), "recording" | "finalizing")
                || (m.status == "interrupted" && m.full_file.is_empty())
            {
                let journal = PathBuf::from(&m.folder).join("audio.packets");
                let duration = codec::recover_duration(&journal).unwrap_or(0);
                let _ = recovery.update(&m.id, |m| {
                    m.duration_ms = duration;
                    m.status = "interrupted".into();
                    m.error = "Recording interrupted; audio recovered from the last durable packet"
                        .into();
                    for c in &mut m.highlights {
                        if c.end_ms.is_none() {
                            c.end_ms = Some(duration);
                            c.status = "exporting".into();
                        }
                    }
                });
                if duration > 0 {
                    recovery.export(m.id.clone(), None);
                }
            }
            if let Ok(current) = recovery.get(&m.id) {
                for c in current.highlights {
                    if c.status == "exporting" {
                        recovery.export(m.id.clone(), Some(c.id));
                    }
                }
            }
        }
    });
    transcript::start(h.clone());
    h
}

#[tauri::command]
pub fn meeting_state(h: State<'_, MeetingHandle>) -> MeetingSnapshot {
    h.snapshot()
}
#[tauri::command]
pub async fn meeting_start(
    h: State<'_, MeetingHandle>,
    title: String,
    mic: bool,
    system: bool,
    transcribe: bool,
    mic_device: Option<String>,
    system_device: Option<String>,
) -> IpcResult<String> {
    if !mic && !system {
        return Err(error("Choose a recording source"));
    }
    let id = new_id();
    let folder = h.root.join(&id);
    std::fs::create_dir_all(&folder).map_err(error)?;
    let (tx, rx) = mpsc::sync_channel(16);
    {
        let mut s = h.inner.lock().unwrap_or_else(PoisonError::into_inner);
        if s.active.is_some() {
            return Err(error("A meeting is already recording"));
        }
        s.active = Some((id.clone(), tx));
        s.last_mark = None;
    }
    let m = Meeting {
        id: id.clone(),
        title: if title.trim().is_empty() {
            "Meeting".into()
        } else {
            title.chars().take(160).collect()
        },
        started_at: now_ms(),
        duration_ms: 0,
        status: "recording".into(),
        folder: folder.to_string_lossy().into_owned(),
        full_file: String::new(),
        notes: String::new(),
        note_revision: 0,
        mic_device,
        system_device,
        mic,
        system,
        transcribe,
        highlights: Vec::new(),
        error: String::new(),
    };
    h.inner
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .meetings
        .insert(0, m);
    if let Err(e) = h.update(&id, |_| {}) {
        h.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .active = None;
        return Err(error(e));
    }
    let handle = h.inner().clone();
    let worker_id = id.clone();
    std::thread::spawn(move || {
        let result = record(&handle, &worker_id, rx);
        if let Err(e) = result {
            let duration = handle
                .get(&worker_id)
                .ok()
                .and_then(|m| {
                    codec::recover_duration(&PathBuf::from(m.folder).join("audio.packets")).ok()
                })
                .unwrap_or(0);
            let _ = handle.update(&worker_id, |m| {
                m.status = "interrupted".into();
                m.error = e;
                m.duration_ms = duration;
                for c in &mut m.highlights {
                    if c.end_ms.is_none() {
                        c.end_ms = Some(duration);
                        c.status = "exporting".into();
                    }
                }
            });
            if duration > 0 {
                handle.export(worker_id.clone(), None);
                if let Ok(m) = handle.get(&worker_id) {
                    for c in m.highlights {
                        if c.status == "exporting" {
                            handle.export(worker_id.clone(), Some(c.id));
                        }
                    }
                }
            }
        }
        {
            let mut s = handle.inner.lock().unwrap_or_else(PoisonError::into_inner);
            s.active = None;
            s.mic_level = 0.0;
            s.system_level = 0.0;
        }
        handle.publish();
    });
    Ok(id)
}

fn record(h: &MeetingHandle, id: &str, rx: mpsc::Receiver<Command>) -> Result<(), String> {
    let m = h.get(id)?;
    let journal = PathBuf::from(&m.folder).join("audio.packets");
    let mut recorder = codec::Recorder::new(&journal)?;
    let (samples_tx, samples_rx) = mpsc::sync_channel::<(bool, Vec<f32>)>(128);
    let (fail_tx, fail_rx) = mpsc::channel();
    let mut captures = Vec::new();
    for (system, enabled) in [(false, m.mic), (true, m.system)] {
        if !enabled {
            continue;
        }
        let (tx, fail, error_tx) = (samples_tx.clone(), fail_tx.clone(), fail_tx.clone());
        captures.push(crate::platform::meeting_audio::start(
            system,
            if system {
                m.system_device.clone()
            } else {
                m.mic_device.clone()
            },
            move |samples| {
                if tx.try_send((system, samples.to_vec())).is_err() {
                    let _ = fail.send(
                        "Audio queue overflow; recording stopped to avoid a silent gap".into(),
                    );
                }
            },
            move |e| {
                let _ = error_tx.send(e);
            },
        )?);
    }
    let mut mic = VecDeque::new();
    let mut system = VecDeque::new();
    let mut next = Instant::now() + Duration::from_millis(60);
    let mut frame = [0.0; codec::FRAME];
    let mut pending = Vec::<(String, u64)>::new();
    loop {
        if let Ok(e) = fail_rx.try_recv() {
            recorder.flush()?;
            return Err(e);
        }
        for (s, chunk) in samples_rx.try_iter() {
            let q = if s { &mut system } else { &mut mic };
            q.extend(chunk);
            if q.len() > 96000 {
                return Err("Audio input drift exceeded buffer limit".into());
            }
        }
        let mut stop = false;
        for command in rx.try_iter() {
            match command {
                Command::Stop => stop = true,
                Command::Mark => {
                    let at = recorder.frames * 20;
                    let mut export = None;
                    h.update(id, |m| {
                        if let Some(c) = m.highlights.last_mut().filter(|c| c.end_ms.is_none()) {
                            c.end_ms = Some(at.max(c.start_ms + 20));
                            c.status = "exporting".into();
                            export = Some((c.id.clone(), c.end_ms.unwrap_or(at)));
                        } else {
                            let cid = new_id();
                            m.highlights.push(Highlight {
                                id: cid.clone(),
                                start_ms: at.saturating_sub(15000),
                                end_ms: None,
                                file: PathBuf::from(&m.folder)
                                    .join(format!("highlight-{cid}.opus"))
                                    .to_string_lossy()
                                    .into_owned(),
                                status: "marking".into(),
                                transcript: String::new(),
                                edited_text: None,
                                transcript_status: "off".into(),
                                attempts: 0,
                                retry_at: 0,
                                error: String::new(),
                            });
                        }
                    })?;
                    if let Some(e) = export {
                        pending.push(e);
                    }
                }
            }
        }
        if stop {
            let end = recorder.frames * 20;
            recorder.push(&[0.0; codec::FRAME])?;
            recorder.flush()?;
            h.update(id, |m| {
                m.duration_ms = end;
                m.status = "finalizing".into();
                if let Some(c) = m.highlights.last_mut().filter(|c| c.end_ms.is_none()) {
                    c.end_ms = Some(end);
                    c.status = "exporting".into();
                    pending.push((c.id.clone(), end));
                }
            })?;
            for (cid, _) in pending.drain(..) {
                h.export(id.to_owned(), Some(cid));
            }
            if end > 0 {
                h.export(id.to_owned(), None);
            } else {
                h.update(id, |m| {
                    m.status = "complete".into();
                })?;
            }
            break;
        }
        let now = Instant::now();
        if now < next {
            std::thread::sleep((next - now).min(Duration::from_millis(5)));
            continue;
        }
        if now.duration_since(next) > Duration::from_secs(2) {
            recorder.flush()?;
            return Err(
                "Capture interrupted by sleep or a stalled system; resume as a new meeting".into(),
            );
        }
        next += Duration::from_millis(20);
        let (mut ml, mut sl) = (0.0_f32, 0.0_f32);
        for output in &mut frame {
            let a = mic.pop_front().unwrap_or(0.0);
            let b = system.pop_front().unwrap_or(0.0);
            ml = ml.max(a.abs());
            sl = sl.max(b.abs());
            *output = ((a + b) * if m.mic && m.system { 0.5 } else { 1.0 }).clamp(-1.0, 1.0);
        }
        recorder.push(&frame)?;
        if recorder.frames % 25 == 0 {
            let mut s = h.inner.lock().unwrap_or_else(PoisonError::into_inner);
            s.mic_level = ml;
            s.system_level = sl;
            drop(s);
            h.publish();
        }
        if recorder.frames % 250 == 0 {
            recorder.flush()?;
            h.update(id, |m| m.duration_ms = recorder.frames * 20)?;
        }
        let ready: Vec<_> = pending
            .iter()
            .filter(|(_, end)| recorder.frames * 20 > *end + 20)
            .cloned()
            .collect();
        if !ready.is_empty() {
            recorder.flush()?;
            for (cid, _) in &ready {
                h.export(id.to_owned(), Some(cid.clone()));
            }
            pending.retain(|p| !ready.contains(p));
        }
    }
    drop(captures);
    Ok(())
}
#[tauri::command]
pub fn meeting_stop(h: State<'_, MeetingHandle>) -> IpcResult<()> {
    let s = h.inner.lock().unwrap_or_else(PoisonError::into_inner);
    s.active
        .as_ref()
        .ok_or_else(|| error("No active meeting"))?
        .1
        .try_send(Command::Stop)
        .map_err(error)
}
#[tauri::command]
pub fn meeting_mark(h: State<'_, MeetingHandle>) -> IpcResult<()> {
    h.mark().map_err(error)
}
#[tauri::command]
pub fn meeting_notes(
    h: State<'_, MeetingHandle>,
    id: String,
    text: String,
    revision: u64,
) -> IpcResult<()> {
    if text.len() > 200_000 {
        return Err(error("Notes too long"));
    }
    let mut conflict = false;
    h.update(&id, |m| {
        if m.note_revision != revision {
            conflict = true;
        } else {
            m.notes = text;
            m.note_revision += 1;
        }
    })
    .map_err(error)?;
    if conflict {
        Err(error("Meeting notes changed in another window; draft kept"))
    } else {
        Ok(())
    }
}
#[tauri::command]
pub fn meeting_retry(h: State<'_, MeetingHandle>, id: String, clip_id: String) -> IpcResult<()> {
    h.update(&id, |m| {
        if let Some(c) = m
            .highlights
            .iter_mut()
            .find(|c| c.id == clip_id && c.status == "ready" && c.transcript_status != "running")
        {
            c.transcript_status = "queued".into();
            c.attempts = 0;
            c.retry_at = 0;
            c.error.clear();
        }
    })
    .map_err(error)?;
    h.wake.notify_one();
    Ok(())
}
#[tauri::command]
pub fn meeting_transcript_edit(
    h: State<'_, MeetingHandle>,
    id: String,
    clip_id: String,
    text: String,
) -> IpcResult<()> {
    if text.len() > 100_000 {
        return Err(error("Transcript too long"));
    }
    h.update(&id, |m| {
        if let Some(c) = m.highlights.iter_mut().find(|c| c.id == clip_id) {
            c.edited_text = Some(text);
        }
    })
    .map_err(error)
}
#[tauri::command]
pub async fn meeting_export(
    h: State<'_, MeetingHandle>,
    id: String,
    destination: String,
) -> IpcResult<()> {
    let m = h.get(&id).map_err(error)?;
    if !matches!(m.status.as_str(), "complete" | "failed" | "interrupted") {
        return Err(error("Stop recording and wait for export first"));
    }
    let folder = PathBuf::from(&m.folder);
    std::fs::write(
        folder.join("notes.md"),
        format!("# {}\n\n{}", m.title, m.notes),
    )
    .map_err(error)?;
    std::fs::write(
        folder.join("meeting.json"),
        serde_json::to_vec_pretty(&m).map_err(error)?,
    )
    .map_err(error)?;
    tauri::async_runtime::spawn_blocking(move || {
        crate::shared::archive::zip_files(
            &[folder],
            std::path::Path::new(&destination),
            &std::sync::atomic::AtomicBool::new(false),
            |_| {},
        )
    })
    .await
    .map_err(error)?
    .map_err(error)
}

#[tauri::command]
pub async fn meeting_devices() -> IpcResult<Vec<crate::platform::meeting_audio::MeetingDevice>> {
    tauri::async_runtime::spawn_blocking(crate::platform::meeting_audio::devices)
        .await
        .map_err(error)?
        .map_err(error)
}
