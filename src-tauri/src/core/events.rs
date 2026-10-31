//! EventBus: cross-module events only (docs/02-system-design.md §3).
//! Module-internal events never go here.

use std::time::{Duration, Instant, SystemTime};

use tokio::sync::broadcast;

use crate::modules::media::dto::NowPlayingDto;

#[derive(Debug, Clone)]
pub enum MediaIntent {
    /// Pause whatever is playing (Focus mode break).
    Pause,
    /// Play a local playlist from the start (Focus mode start).
    PlayLocalPlaylist(String),
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    /// The machine woke up or the wall clock jumped (sleep, NTP, manual change).
    Resumed,
    SessionLocked(bool),
    /// Island window hidden (fullscreen app, user hidden, locked).
    IslandHidden(bool),
    NowPlayingChanged(Option<NowPlayingDto>),
    #[allow(
        dead_code,
        reason = "consumed by plugins and the mic/cam indicator (P4)"
    )]
    FocusModeChanged(bool),
    MediaIntent(MediaIntent),
    /// Default audio output device changed.
    AudioDeviceChanged,
}

#[derive(Clone)]
pub struct EventBus(broadcast::Sender<AppEvent>);

impl EventBus {
    pub fn new() -> Self {
        Self(broadcast::channel(64).0)
    }
    pub fn send(&self, event: AppEvent) {
        let _ = self.0.send(event);
    }
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.0.subscribe()
    }
}

/// Emits `Resumed` when wall-clock time and monotonic time drift apart by more than
/// 10 s between two 5 s ticks — i.e. after sleep/hibernate or a clock change.
pub fn spawn_resume_detector(bus: EventBus) {
    tauri::async_runtime::spawn(async move {
        let mut wall = SystemTime::now();
        let mut mono = Instant::now();
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let (w, m) = (SystemTime::now(), Instant::now());
            let wall_delta = w.duration_since(wall).unwrap_or_default().as_secs_f64();
            let mono_delta = m.duration_since(mono).as_secs_f64();
            if (wall_delta - mono_delta).abs() > 10.0 {
                tracing::info!(
                    wall_delta,
                    mono_delta,
                    "clock jump detected, broadcasting Resumed"
                );
                bus.send(AppEvent::Resumed);
            }
            (wall, mono) = (w, m);
        }
    });
}
