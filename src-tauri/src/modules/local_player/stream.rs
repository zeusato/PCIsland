//! Internet radio / direct stream source (F13, docs/modules/local-player.md §5).
//! A network thread fills a buffer; the decoder reads from it, so the audio callback
//! never waits on the network unless the buffer runs dry.

use std::collections::VecDeque;
use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::Duration;

const PREBUFFER: usize = 64 * 1024;
const MAX_BUFFER: usize = 1024 * 1024;
const READ_WAIT: Duration = Duration::from_secs(3);

struct Shared {
    data: Mutex<VecDeque<u8>>,
    ready: Condvar,
    done: AtomicBool,
    closed: AtomicBool,
}

/// `Read + Seek` over an HTTP stream. Seeking is only supported to the current position.
pub struct HttpStream {
    shared: Arc<Shared>,
    pos: u64,
}

impl Drop for HttpStream {
    fn drop(&mut self) {
        self.shared.closed.store(true, Ordering::Relaxed);
        self.shared.ready.notify_all();
    }
}

impl HttpStream {
    /// Connects and prebuffers. Returns the stream and its content type.
    pub fn open(url: &str) -> Result<(Self, Option<String>), String> {
        let client = reqwest::blocking::Client::builder()
            .user_agent(concat!("PCisland/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(8))
            .build()
            .map_err(|e| e.to_string())?;
        let mut resp = client.get(url).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let shared = Arc::new(Shared {
            data: Mutex::new(VecDeque::with_capacity(PREBUFFER * 2)),
            ready: Condvar::new(),
            done: AtomicBool::new(false),
            closed: AtomicBool::new(false),
        });
        let filler = shared.clone();
        std::thread::Builder::new()
            .name("radio-net".into())
            .spawn(move || {
                let mut buf = [0u8; 16 * 1024];
                while !filler.closed.load(Ordering::Relaxed) {
                    match resp.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let mut data =
                                filler.data.lock().unwrap_or_else(PoisonError::into_inner);
                            data.extend(&buf[..n]);
                            // Live streams never end: drop the oldest audio rather than grow forever.
                            while data.len() > MAX_BUFFER {
                                data.pop_front();
                            }
                            filler.ready.notify_all();
                        }
                    }
                }
                filler.done.store(true, Ordering::Relaxed);
                filler.ready.notify_all();
            })
            .map_err(|e| e.to_string())?;
        let stream = Self { shared, pos: 0 };
        // Prebuffer so playback starts smoothly.
        let data = stream
            .shared
            .data
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let (_data, timeout) = stream
            .shared
            .ready
            .wait_timeout_while(data, Duration::from_secs(8), |d| {
                d.len() < PREBUFFER && !stream.shared.done.load(Ordering::Relaxed)
            })
            .unwrap_or_else(PoisonError::into_inner);
        if timeout.timed_out() {
            return Err("stream did not deliver data in time".into());
        }
        drop(_data);
        Ok((stream, content_type))
    }
}

impl Read for HttpStream {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let data = self
            .shared
            .data
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let (mut data, _) = self
            .shared
            .ready
            .wait_timeout_while(data, READ_WAIT, |d| {
                d.is_empty() && !self.shared.done.load(Ordering::Relaxed)
            })
            .unwrap_or_else(PoisonError::into_inner);
        let n = out.len().min(data.len());
        for (dst, src) in out.iter_mut().zip(data.drain(..n)) {
            *dst = src;
        }
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for HttpStream {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        match to {
            SeekFrom::Current(0) => Ok(self.pos),
            SeekFrom::Start(p) if p == self.pos => Ok(self.pos),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "live stream cannot seek",
            )),
        }
    }
}

/// Maps a content type to a decoder hint.
pub fn hint_for(content_type: Option<&str>) -> Option<&'static str> {
    let ct = content_type?.to_ascii_lowercase();
    Some(if ct.contains("mpeg") || ct.contains("mp3") {
        "mp3"
    } else if ct.contains("aac") {
        "aac"
    } else if ct.contains("ogg") {
        "ogg"
    } else if ct.contains("flac") {
        "flac"
    } else {
        return None;
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn content_type_hints() {
        assert_eq!(super::hint_for(Some("audio/mpeg")), Some("mp3"));
        assert_eq!(super::hint_for(Some("audio/aacp")), Some("aac"));
        assert_eq!(super::hint_for(Some("application/ogg")), Some("ogg"));
        assert_eq!(super::hint_for(Some("text/html")), None);
        assert_eq!(super::hint_for(None), None);
    }
}
