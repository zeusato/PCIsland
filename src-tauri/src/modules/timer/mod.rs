//! Timers, stopwatch, Pomodoro and Focus mode (docs/modules/timer.md).
#![forbid(unsafe_code)]

pub mod commands;
pub mod logic;

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot, watch};
use ts_rs::TS;

use crate::core::activity::{Activity, ActivityAction, ActivityHub};
use crate::core::audio::{AudioEngine, SoundId};
use crate::core::events::{AppEvent, EventBus, MediaIntent};
use crate::core::id::new_id;
use crate::core::priority;
use crate::core::settings::Settings;
use crate::core::storage::Db;
use crate::core::supervisor::{Shared, Supervisor, shared};
use crate::ipc::{events, now_ms};
use logic::{Countdown, CountdownState, Phase, Pomodoro, PomodoroConfig, PomodoroRun, Stopwatch};

const STATE_KEY: &str = "timer.state";
/// Safety-net wake-up while something runs (docs/modules/timer.md §5).
const FALLBACK_CHECK: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", default)]
pub struct TimerSnapshot {
    pub timers: Vec<Countdown>,
    pub stopwatch: Stopwatch,
    pub pomodoro: Option<Pomodoro>,
    pub focus_manual: bool,
    pub focus_active: bool,
    #[ts(type = "number")]
    pub rev: u64,
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PomodoroStats {
    pub completed_work: u32,
    #[ts(type = "number")]
    pub focused_ms: u64,
}

pub enum Cmd {
    Start {
        duration_ms: u64,
        label: Option<String>,
        reply: oneshot::Sender<Countdown>,
    },
    Pause(String),
    Resume(String),
    Cancel(String),
    Add(String, i64),
    Dismiss(String),
    Repeat(String),
    StopwatchStart,
    StopwatchPause,
    StopwatchReset,
    StopwatchLap,
    PomodoroStart,
    PomodoroPause,
    PomodoroResume,
    PomodoroSkip,
    PomodoroStop,
    PomodoroNext,
    Focus(bool),
    Snapshot(oneshot::Sender<TimerSnapshot>),
}

#[derive(Clone)]
pub struct TimerHandle {
    tx: mpsc::UnboundedSender<Cmd>,
    db: Db,
}

impl TimerHandle {
    pub fn send(&self, cmd: Cmd) {
        let _ = self.tx.send(cmd);
    }
    pub async fn start(&self, duration_ms: u64, label: Option<String>) -> Option<Countdown> {
        let (reply, rx) = oneshot::channel();
        self.send(Cmd::Start {
            duration_ms,
            label,
            reply,
        });
        rx.await.ok()
    }
    pub async fn snapshot(&self) -> TimerSnapshot {
        let (reply, rx) = oneshot::channel();
        self.send(Cmd::Snapshot(reply));
        rx.await.unwrap_or_default()
    }
    /// Pomodoro stats since `since` (epoch ms).
    pub fn stats(&self, since: u64) -> PomodoroStats {
        self.db
            .call(|c| {
                c.query_row(
                    "SELECT COUNT(*), COALESCE(SUM(ended_at - started_at), 0) FROM timer_pomodoro_sessions
                     WHERE phase = 'work' AND completed = 1 AND started_at >= ?1",
                    [since as i64],
                    |r| Ok(PomodoroStats { completed_work: r.get::<_, i64>(0)? as u32, focused_ms: r.get::<_, i64>(1)? as u64 }),
                )
            })
            .unwrap_or_default()
    }
}

pub fn setup(
    app: &AppHandle,
    db: Db,
    hub: ActivityHub,
    audio: AudioEngine,
    settings: watch::Receiver<Settings>,
    bus: EventBus,
    sup: &Supervisor,
) -> TimerHandle {
    let (tx, rx) = mpsc::unbounded_channel();
    let handle = TimerHandle { tx, db: db.clone() };
    let for_actions = handle.clone();
    hub.register_handler(
        "timer",
        Arc::new(move |activity, action| {
            let target = activity.strip_prefix("timer:").unwrap_or_default();
            let id = target.strip_prefix("ring-").unwrap_or(target).to_owned();
            let cmd = match (target, action) {
                ("pomodoro" | "pomodoro-end", "pause") => Cmd::PomodoroPause,
                ("pomodoro" | "pomodoro-end", "resume") => Cmd::PomodoroResume,
                ("pomodoro" | "pomodoro-end", "skip") => Cmd::PomodoroSkip,
                ("pomodoro" | "pomodoro-end", "stop") => Cmd::PomodoroStop,
                ("pomodoro-end", "start_next") => Cmd::PomodoroNext,
                ("stopwatch", "pause") => Cmd::StopwatchPause,
                ("stopwatch", "resume") => Cmd::StopwatchStart,
                ("stopwatch", "lap") => Cmd::StopwatchLap,
                ("stopwatch", "reset") => Cmd::StopwatchReset,
                (_, "pause") => Cmd::Pause(id),
                (_, "resume") => Cmd::Resume(id),
                (_, "add_minute") => Cmd::Add(id, 60_000),
                (_, "cancel") => Cmd::Cancel(id),
                (_, "stop") => Cmd::Dismiss(id),
                (_, "repeat") => Cmd::Repeat(id),
                _ => return Err(format!("unknown timer action `{action}`")),
            };
            for_actions.send(cmd);
            Ok(())
        }),
    );
    let rx = shared(rx);
    let app = app.clone();
    // Restartable: state is reloaded from the database on every (re)start.
    sup.spawn("timer", move || {
        let initial: TimerSnapshot = db
            .kv_get(STATE_KEY)
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let ctx = Ctx {
            db: db.clone(),
            hub: hub.clone(),
            audio: audio.clone(),
            settings: settings.clone(),
            bus: bus.clone(),
        };
        service(app.clone(), rx.clone(), ctx, initial)
    });
    handle
}

struct Ctx {
    db: Db,
    hub: ActivityHub,
    audio: AudioEngine,
    settings: watch::Receiver<Settings>,
    bus: EventBus,
}

impl Ctx {
    fn pomodoro_config(&self) -> PomodoroConfig {
        let p = &self.settings.borrow().pomodoro;
        PomodoroConfig {
            work_ms: u64::from(p.work_min) * 60_000,
            short_ms: u64::from(p.short_break_min) * 60_000,
            long_ms: u64::from(p.long_break_min) * 60_000,
            long_every: p.long_every,
            auto_start_breaks: p.auto_start_breaks,
            auto_start_work: p.auto_start_work,
        }
    }
    fn ring(&self) {
        let s = self.settings.borrow();
        if let Some(sound) = SoundId::named(&s.app.sound_timer) {
            self.audio.ring(
                sound,
                s.app.sound_volume,
                Duration::from_secs(u64::from(s.timer.ring_max_sec)),
            );
        }
    }
}

async fn service(app: AppHandle, rx: Shared<Cmd>, ctx: Ctx, mut s: TimerSnapshot) {
    // Anything that ended while the app was closed rings now.
    let mut dirty = true;
    let mut events = ctx.bus.subscribe();
    let mut focus_was = !s.focus_active;
    loop {
        let now = now_ms();
        for t in &mut s.timers {
            if t.check(now) {
                tracing::info!(module = "timer", id = %t.id, "timer finished");
                ctx.ring();
                dirty = true;
            }
        }
        let cfg = ctx.pomodoro_config();
        if let Some(p) = s.pomodoro.as_mut() {
            if let Some(end) = p.check(&cfg, now) {
                record_phase(&ctx.db, &end);
                let (sound, vol) = {
                    let a = &ctx.settings.borrow().app;
                    (SoundId::named(&a.sound_pomodoro), a.sound_volume)
                };
                if let Some(sound) = sound {
                    ctx.audio.play(sound, vol);
                }
                if end.auto_started {
                    ctx.hub.alert(phase_alert(&end).transient(5_000));
                }
                if end.next != Phase::Work && ctx.settings.borrow().pomodoro.pause_media_on_break {
                    ctx.bus.send(AppEvent::MediaIntent(MediaIntent::Pause));
                }
                dirty = true;
            }
        }
        if dirty {
            s.focus_active = s.focus_manual
                || (ctx.settings.borrow().pomodoro.focus_with_pomodoro
                    && s.pomodoro.as_ref().is_some_and(Pomodoro::in_work));
            ctx.hub.set_focus(s.focus_active);
            if focus_was != s.focus_active {
                focus_was = s.focus_active;
                ctx.bus.send(AppEvent::FocusModeChanged(s.focus_active));
            }
            s.rev += 1;
            publish(&app, &ctx, &s, now);
            if let Ok(json) = serde_json::to_string(&s) {
                let _ = ctx.db.kv_set(STATE_KEY, &json);
            }
            dirty = false;
        }

        let next_deadline = s
            .timers
            .iter()
            .filter_map(Countdown::deadline)
            .chain(s.pomodoro.as_ref().and_then(Pomodoro::deadline))
            .min();
        let sleep = next_deadline.map_or(Duration::from_secs(3600), |d| {
            Duration::from_millis(d.saturating_sub(now_ms())).min(FALLBACK_CHECK)
        });
        let cmd = tokio::select! {
            cmd = async { rx.lock().await.recv().await } => match cmd { Some(c) => c, None => return },
            // Wake-ups (sleep/resume, clock change) re-check every deadline.
            Ok(AppEvent::Resumed) = events.recv() => continue,
            () = tokio::time::sleep(sleep) => continue,
        };
        let now = now_ms();
        dirty = true;
        match cmd {
            Cmd::Start {
                duration_ms,
                label,
                reply,
            } => {
                let t = Countdown::start(new_id(), duration_ms.max(1_000), label, now);
                let _ = reply.send(t.clone());
                s.timers.push(t);
            }
            Cmd::Pause(id) => with_timer(&mut s, &id, |t| t.pause(now)),
            Cmd::Resume(id) => with_timer(&mut s, &id, |t| t.resume(now)),
            Cmd::Add(id, delta) => {
                with_timer(&mut s, &id, |t| t.add(delta, now));
                ctx.audio.stop_alert();
            }
            Cmd::Cancel(id) | Cmd::Dismiss(id) => {
                s.timers.retain(|t| t.id != id);
                ctx.hub.remove(&format!("timer:ring-{id}"));
                ctx.audio.stop_alert();
            }
            Cmd::Repeat(id) => {
                with_timer(&mut s, &id, |t| {
                    *t = Countdown::start(t.id.clone(), t.duration_ms, t.label.clone(), now)
                });
                ctx.hub.remove(&format!("timer:ring-{id}"));
                ctx.audio.stop_alert();
            }
            Cmd::StopwatchStart => s.stopwatch.start(now),
            Cmd::StopwatchPause => s.stopwatch.pause(now),
            Cmd::StopwatchReset => s.stopwatch = Stopwatch::default(),
            Cmd::StopwatchLap => s.stopwatch.lap(now),
            Cmd::PomodoroStart => {
                s.pomodoro = Some(Pomodoro::start(&ctx.pomodoro_config(), now));
                let playlist = ctx.settings.borrow().pomodoro.focus_playlist_id.clone();
                if !playlist.is_empty() {
                    ctx.bus
                        .send(AppEvent::MediaIntent(MediaIntent::PlayLocalPlaylist(
                            playlist,
                        )));
                }
            }
            Cmd::PomodoroPause => s.pomodoro.iter_mut().for_each(|p| p.pause(now)),
            Cmd::PomodoroResume => s.pomodoro.iter_mut().for_each(|p| p.resume(now)),
            Cmd::PomodoroNext => s.pomodoro.iter_mut().for_each(|p| p.start_waiting(now)),
            Cmd::PomodoroSkip => {
                let cfg = ctx.pomodoro_config();
                if let Some(p) = s.pomodoro.as_mut() {
                    let end = p.advance(&cfg, now, false);
                    record_phase(&ctx.db, &end);
                    p.start_waiting(now);
                }
            }
            Cmd::PomodoroStop => s.pomodoro = None,
            Cmd::Focus(on) => s.focus_manual = on,
            Cmd::Snapshot(reply) => {
                let _ = reply.send(s.clone());
                dirty = false;
            }
        }
    }
}

fn with_timer(s: &mut TimerSnapshot, id: &str, f: impl FnOnce(&mut Countdown)) {
    if let Some(t) = s.timers.iter_mut().find(|t| t.id == id) {
        f(t);
    }
}

fn record_phase(db: &Db, end: &logic::PhaseEnd) {
    let phase = match end.finished {
        Phase::Work => "work",
        Phase::ShortBreak => "short_break",
        Phase::LongBreak => "long_break",
    };
    let _ = db.call(|c| {
        c.execute(
            "INSERT INTO timer_pomodoro_sessions (id, phase, planned_ms, started_at, ended_at, completed) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![new_id(), phase, end.planned_ms as i64, end.started_at as i64, end.ended_at as i64, end.completed],
        )
    });
}

fn phase_alert(end: &logic::PhaseEnd) -> Activity {
    Activity::new(
        "timer",
        "timer:pomodoro-end",
        "pomodoro.phase_end",
        priority::ALERT,
        crate::core::activity::payload(&PhaseEndPayload {
            finished: end.finished,
            next: end.next,
            auto_started: end.auto_started,
        }),
    )
}

fn publish(app: &AppHandle, ctx: &Ctx, s: &TimerSnapshot, now: u64) {
    let hub = &ctx.hub;
    // Countdowns: one activity for the soonest running/paused timer, one critical per ringing timer.
    let mut active: Vec<&Countdown> = s
        .timers
        .iter()
        .filter(|t| !matches!(t.state, CountdownState::Ringing { .. }))
        .collect();
    active.sort_by_key(|t| t.remaining(now));
    match active.first() {
        Some(t) => hub.upsert(
            Activity::new(
                "timer",
                "timer:countdowns",
                "timer.running",
                priority::LIVE,
                crate::core::activity::payload(&CountdownsPayload {
                    timer: (*t).clone(),
                    count: active.len() as u32,
                    all: active.iter().map(|c| (*c).clone()).collect(),
                }),
            )
            .with_actions(vec![
                ActivityAction::new(
                    if matches!(t.state, CountdownState::Paused { .. }) {
                        "resume"
                    } else {
                        "pause"
                    },
                    "pause",
                    "timer.action.pauseResume",
                )
                .primary(),
                ActivityAction::new("add_minute", "plus", "timer.action.addMinute"),
                ActivityAction::new("cancel", "x", "timer.action.cancel"),
            ]),
        ),
        None => hub.remove("timer:countdowns"),
    }
    for t in &s.timers {
        let id = format!("timer:ring-{}", t.id);
        if let CountdownState::Ringing { .. } = t.state {
            hub.upsert(
                Activity::new(
                    "timer",
                    id,
                    "timer.finished",
                    priority::CRITICAL,
                    serde_json::to_value(t).unwrap_or_default(),
                )
                .with_actions(vec![
                    ActivityAction::new("stop", "check", "timer.action.stop").primary(),
                    ActivityAction::new("add_minute", "plus", "timer.action.addMinute"),
                    ActivityAction::new("repeat", "rotate-ccw", "timer.action.repeat"),
                ]),
            );
        }
    }
    if s.stopwatch.is_idle() {
        hub.remove("timer:stopwatch");
    } else {
        let running = s.stopwatch.started_at.is_some();
        hub.upsert(
            Activity::new(
                "timer",
                "timer:stopwatch",
                "stopwatch.running",
                priority::LIVE,
                serde_json::to_value(&s.stopwatch).unwrap_or_default(),
            )
            .with_actions(vec![
                ActivityAction::new(
                    if running { "pause" } else { "resume" },
                    "pause",
                    "timer.action.pauseResume",
                )
                .primary(),
                ActivityAction::new("lap", "flag", "timer.action.lap"),
                ActivityAction::new("reset", "rotate-ccw", "timer.action.reset"),
            ]),
        );
    }
    match &s.pomodoro {
        Some(p) if p.run == PomodoroRun::Waiting => {
            hub.remove("timer:pomodoro");
            hub.upsert(
                Activity::new(
                    "timer",
                    "timer:pomodoro-end",
                    "pomodoro.phase_end",
                    priority::CRITICAL,
                    serde_json::to_value(p).unwrap_or_default(),
                )
                .with_actions(vec![
                    ActivityAction::new("start_next", "play", "timer.action.startNext").primary(),
                    ActivityAction::new("stop", "square", "timer.action.stop"),
                ]),
            );
        }
        Some(p) => {
            hub.remove("timer:pomodoro-end");
            let paused = matches!(p.run, PomodoroRun::Paused { .. });
            hub.upsert(
                Activity::new(
                    "timer",
                    "timer:pomodoro",
                    "pomodoro.running",
                    priority::LIVE,
                    serde_json::to_value(p).unwrap_or_default(),
                )
                .with_actions(vec![
                    ActivityAction::new(
                        if paused { "resume" } else { "pause" },
                        "pause",
                        "timer.action.pauseResume",
                    )
                    .primary(),
                    ActivityAction::new("skip", "skip-forward", "timer.action.skip"),
                    ActivityAction::new("stop", "square", "timer.action.stop"),
                ]),
            );
        }
        None => {
            hub.remove("timer:pomodoro");
            hub.remove("timer:pomodoro-end");
        }
    }
    for label in ["island", "main"] {
        let _ = app.emit_to(label, events::TIMER_STATE, s);
    }
}

#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PhaseEndPayload {
    pub finished: Phase,
    pub next: Phase,
    pub auto_started: bool,
}

#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CountdownsPayload {
    /// The timer ending soonest.
    pub timer: Countdown,
    pub count: u32,
    pub all: Vec<Countdown>,
}
