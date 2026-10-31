//! Real audio visualizer (docs/modules/lyrics-visualizer.md §B). Captures only while
//! something plays, the island is visible and the mode is "real" (CPU ≈ 0 otherwise).
#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use tauri::State;
use tauri::ipc::Channel;
use tokio::sync::watch;

use crate::core::events::{AppEvent, EventBus};
use crate::core::settings::Settings;
use crate::modules::media::dto::PlaybackStatus;
use crate::platform::loopback;
use crate::shared::spectrum::Analyzer;

#[derive(Clone, Default)]
pub struct VisualizerHandle {
    channels: Arc<Mutex<Vec<Channel<Vec<u8>>>>>,
}

#[derive(Default, Clone, Copy, PartialEq)]
struct Conditions {
    playing: bool,
    island_hidden: bool,
    locked: bool,
    failed: bool,
}

impl Conditions {
    fn should_run(self, mode: &str) -> bool {
        mode == "real" && self.playing && !self.island_hidden && !self.locked && !self.failed
    }
}

pub fn setup(bus: EventBus, mut settings: watch::Receiver<Settings>) -> VisualizerHandle {
    let handle = VisualizerHandle::default();
    let channels = handle.channels.clone();
    tauri::async_runtime::spawn(async move {
        let mut events = bus.subscribe();
        let mut cond = Conditions::default();
        let mut capture: Option<loopback::Capture> = None;
        let analyzer = Arc::new(Mutex::new(Analyzer::new()));
        let (fail_tx, mut fail_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let mut tick = tokio::time::interval(Duration::from_millis(33));
        loop {
            let fps = settings.borrow().visualizer.fps.clamp(15, 60);
            tokio::select! {
                ev = events.recv() => match ev {
                    Ok(AppEvent::NowPlayingChanged(np)) => cond.playing = np.is_some_and(|n| n.status == PlaybackStatus::Playing),
                    Ok(AppEvent::IslandHidden(hidden)) => cond.island_hidden = hidden,
                    Ok(AppEvent::SessionLocked(locked)) => cond.locked = locked,
                    Ok(AppEvent::AudioDeviceChanged) => { capture = None; cond.failed = false; }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
                    _ => {}
                },
                Some(reason) = fail_rx.recv() => {
                    tracing::warn!(module = "visualizer", %reason, "loopback unavailable; falling back to simulated bars");
                    cond.failed = true;
                }
                _ = settings.changed() => {}
                _ = tick.tick(), if capture.is_some() => {
                    let frame = analyzer.lock().unwrap_or_else(PoisonError::into_inner).frame().to_vec();
                    channels.lock().unwrap_or_else(PoisonError::into_inner).retain(|c| c.send(frame.clone()).is_ok());
                    continue;
                }
            }
            tick = tokio::time::interval(Duration::from_millis(1000 / u64::from(fps)));
            let run = cond.should_run(&settings.borrow().visualizer.mode);
            match (run, capture.is_some()) {
                (true, false) => {
                    let a = analyzer.clone();
                    let fail = fail_tx.clone();
                    capture = Some(loopback::start(
                        move |mono, rate| {
                            a.lock()
                                .unwrap_or_else(PoisonError::into_inner)
                                .push(mono, rate)
                        },
                        move |e| {
                            let _ = fail.send(e);
                        },
                    ));
                }
                (false, true) => {
                    capture = None;
                    // Let the bars fall back to rest.
                    let zero = vec![0u8; crate::shared::spectrum::BANDS];
                    channels
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .retain(|c| c.send(zero.clone()).is_ok());
                }
                _ => {}
            }
        }
    });
    handle
}

/// The island subscribes once; frames arrive only while capture runs.
#[tauri::command]
pub fn visualizer_subscribe(viz: State<'_, VisualizerHandle>, channel: Channel<Vec<u8>>) {
    viz.channels
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(channel);
}

#[cfg(test)]
mod tests {
    use super::Conditions;

    #[test]
    fn runs_only_when_everything_allows_it() {
        let ok = Conditions {
            playing: true,
            ..Default::default()
        };
        assert!(ok.should_run("real"));
        assert!(!ok.should_run("fake"));
        assert!(
            !Conditions {
                island_hidden: true,
                ..ok
            }
            .should_run("real")
        );
        assert!(!Conditions { locked: true, ..ok }.should_run("real"));
        assert!(!Conditions { failed: true, ..ok }.should_run("real"));
        assert!(!Conditions::default().should_run("real"));
    }
}
