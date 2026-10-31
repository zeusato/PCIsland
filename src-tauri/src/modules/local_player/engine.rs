//! Music playback half of the audio thread (docs/modules/local-player.md §5).
//! Lives inside `core::audio`'s thread; talks back through `EventSink`.

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use rodio::{Decoder, Player as Sink, Source};

#[derive(Debug, Clone)]
pub enum MusicCmd {
    Load {
        path: PathBuf,
        start_ms: u64,
        autoplay: bool,
    },
    Play,
    Pause,
    Seek(u64),
    SetVolume(f32),
    Stop,
}

#[derive(Debug, Clone)]
pub enum MusicEvent {
    Loaded {
        duration_ms: Option<u64>,
        playing: bool,
        position_ms: u64,
    },
    State {
        playing: bool,
        position_ms: u64,
    },
    Ended,
    Failed {
        path: PathBuf,
        message: String,
    },
}

pub type EventSink = Arc<dyn Fn(MusicEvent) + Send + Sync>;

pub struct MusicPlayer {
    sink: Sink,
    events: EventSink,
    loaded: bool,
    ended_reported: bool,
}

impl MusicPlayer {
    pub fn new(mixer: &rodio::mixer::Mixer, events: EventSink) -> Self {
        let sink = Sink::connect_new(mixer);
        sink.pause();
        Self {
            sink,
            events,
            loaded: false,
            ended_reported: false,
        }
    }

    pub fn handle(&mut self, cmd: MusicCmd) {
        match cmd {
            MusicCmd::Load {
                path,
                start_ms,
                autoplay,
            } => self.load(path, start_ms, autoplay),
            MusicCmd::Play if self.loaded => {
                self.sink.play();
                self.report_state();
            }
            MusicCmd::Pause => {
                self.sink.pause();
                self.report_state();
            }
            MusicCmd::Seek(ms) if self.loaded => {
                if let Err(err) = self.sink.try_seek(Duration::from_millis(ms)) {
                    tracing::debug!(module = "local_player", error = ?err, "seek failed");
                }
                self.report_state();
            }
            MusicCmd::SetVolume(v) => self.sink.set_volume(perceptual(v)),
            MusicCmd::Stop => {
                self.sink.stop();
                self.loaded = false;
            }
            _ => {}
        }
    }

    fn load(&mut self, path: PathBuf, start_ms: u64, autoplay: bool) {
        self.sink.stop();
        self.loaded = false;
        let decoded = File::open(&path)
            .map_err(|e| e.to_string())
            .and_then(|f| Decoder::new(BufReader::new(f)).map_err(|e| e.to_string()));
        match decoded {
            Ok(source) => {
                let duration_ms = source.total_duration().map(|d| d.as_millis() as u64);
                self.sink.append(source);
                if start_ms > 0 {
                    let _ = self.sink.try_seek(Duration::from_millis(start_ms));
                }
                if autoplay {
                    self.sink.play();
                } else {
                    self.sink.pause();
                }
                self.loaded = true;
                self.ended_reported = false;
                (self.events)(MusicEvent::Loaded {
                    duration_ms,
                    playing: autoplay,
                    position_ms: self.position_ms(),
                });
            }
            Err(message) => (self.events)(MusicEvent::Failed { path, message }),
        }
    }

    /// Called every loop iteration of the audio thread (≤ 200 ms).
    pub fn tick(&mut self) {
        if self.loaded && !self.ended_reported && self.sink.empty() {
            self.ended_reported = true;
            self.loaded = false;
            (self.events)(MusicEvent::Ended);
        }
    }

    fn position_ms(&self) -> u64 {
        self.sink.get_pos().as_millis() as u64
    }

    fn report_state(&self) {
        (self.events)(MusicEvent::State {
            playing: !self.sink.is_paused(),
            position_ms: self.position_ms(),
        });
    }
}

/// Volume slider value → amplitude; a power curve feels linear to the ear.
fn perceptual(v: f32) -> f32 {
    v.clamp(0.0, 1.0).powf(2.0)
}
