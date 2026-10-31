//! Audio engine on its own thread: rodio's output stream is not `Send`
//! (docs/02-system-design.md §9). Alert sounds are synthesized, so no assets ship.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use rodio::Source;
use rodio::source::SineWave;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum SoundId {
    /// Reminders.
    Chime,
    /// Timer finished.
    Bell,
    /// Pomodoro phase change.
    SoftTick,
    /// Item dropped on the shelf.
    Pop,
}

/// Names accepted by the per-type sound settings; `"none"` means silent.
pub const SOUND_NAMES: [&str; 5] = ["chime", "bell", "softTick", "pop", "none"];

impl SoundId {
    /// Parses a settings value; `None` for "none" or anything unknown.
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "chime" => Some(Self::Chime),
            "bell" => Some(Self::Bell),
            "softTick" => Some(Self::SoftTick),
            "pop" => Some(Self::Pop),
            _ => None,
        }
    }
}

enum Cmd {
    Play {
        sound: SoundId,
        volume: f32,
        repeat_until: Option<Instant>,
    },
    StopAlert,
    /// Default output device changed: rebuild the stream and resume.
    Reopen,
    Music(crate::modules::local_player::engine::MusicCmd),
}

#[derive(Clone)]
pub struct AudioEngine {
    tx: Sender<Cmd>,
}

impl AudioEngine {
    pub fn start(music_events: crate::modules::local_player::engine::EventSink) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("audio".into())
            .spawn(move || run(&rx, &music_events))
            .expect("spawn audio thread");
        Self { tx }
    }

    pub fn play(&self, sound: SoundId, volume: f32) {
        let _ = self.tx.send(Cmd::Play {
            sound,
            volume,
            repeat_until: None,
        });
    }

    /// Repeats `sound` until `duration` passes or `stop_alert` is called.
    pub fn ring(&self, sound: SoundId, volume: f32, duration: Duration) {
        let _ = self.tx.send(Cmd::Play {
            sound,
            volume,
            repeat_until: Some(Instant::now() + duration),
        });
    }

    pub fn reopen(&self) {
        let _ = self.tx.send(Cmd::Reopen);
    }

    pub fn stop_alert(&self) {
        let _ = self.tx.send(Cmd::StopAlert);
    }

    pub fn music(&self, cmd: crate::modules::local_player::engine::MusicCmd) {
        let _ = self.tx.send(Cmd::Music(cmd));
    }
}

fn run(rx: &mpsc::Receiver<Cmd>, music_events: &crate::modules::local_player::engine::EventSink) {
    let mut stream = match rodio::DeviceSinkBuilder::open_default_sink() {
        Ok(s) => s,
        Err(err) => {
            tracing::warn!(module = "audio", error = %err, "no audio output; sounds disabled");
            // Drain commands so senders never block.
            while rx.recv().is_ok() {}
            return;
        }
    };
    stream.log_on_drop(false);
    let mut alert = rodio::Player::connect_new(stream.mixer());
    let mut music = crate::modules::local_player::engine::MusicPlayer::new(
        stream.mixer(),
        music_events.clone(),
    );
    let mut ring: Option<(SoundId, f32, Instant)> = None;

    loop {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(Cmd::Play {
                sound,
                volume,
                repeat_until,
            }) => {
                alert.stop();
                append(&alert, sound, volume);
                ring = repeat_until.map(|until| (sound, volume, until));
            }
            Ok(Cmd::StopAlert) => {
                alert.stop();
                ring = None;
            }
            Ok(Cmd::Music(cmd)) => music.handle(cmd),
            Ok(Cmd::Reopen) => match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(mut next) => {
                    next.log_on_drop(false);
                    alert = rodio::Player::connect_new(next.mixer());
                    music.reattach(next.mixer());
                    // Dropping the old stream after the swap releases the old device.
                    drop(std::mem::replace(&mut stream, next));
                    tracing::info!(
                        module = "audio",
                        "reopened output on the new default device"
                    );
                }
                Err(err) => {
                    tracing::warn!(module = "audio", error = %err, "cannot reopen audio output")
                }
            },
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        if let Some((sound, volume, until)) = ring {
            if Instant::now() >= until {
                ring = None;
            } else if alert.empty() {
                std::thread::sleep(Duration::from_millis(600));
                append(&alert, sound, volume);
            }
        }
        music.tick();
    }
}

/// Short, soft synthesized motifs (docs/04-design-system.md §8).
fn append(sink: &rodio::Player, sound: SoundId, volume: f32) {
    let notes: &[(f32, u64)] = match sound {
        SoundId::Chime => &[(1046.5, 140), (1318.5, 140), (1568.0, 260)],
        SoundId::Bell => &[(880.0, 180), (660.0, 180), (880.0, 180), (660.0, 260)],
        SoundId::SoftTick => &[(1174.7, 120), (1568.0, 200)],
        SoundId::Pop => &[(740.0, 70)],
    };
    for &(freq, ms) in notes {
        let note = SineWave::new(freq)
            .take_duration(Duration::from_millis(ms))
            .fade_in(Duration::from_millis(8))
            .amplify(0.25 * volume.clamp(0.0, 1.0));
        sink.append(note);
    }
}
