//! Finds media streams on an arbitrary page in a hidden, muted view (docs/modules/
//! video-player.md §5.3). One sniff at a time; the page can only report bounded URLs.

use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use tauri::{AppHandle, Manager, Url, Webview, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::mpsc;

use super::dto::SniffItem;
use crate::ipc::IpcError;

pub const LABEL: &str = "video-sniff";
const TIMEOUT: Duration = Duration::from_secs(25);
const ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
--mute-audio --autoplay-policy=no-user-gesture-required";

pub struct Found {
    pub url: String,
    pub hls: bool,
}

pub struct Report {
    pub items: Vec<SniffItem>,
    pub blobs: u32,
    pub done: bool,
}

#[derive(Default)]
pub struct SniffState {
    pending: Mutex<Option<(String, mpsc::UnboundedSender<Report>)>>,
}

/// Same rule as the add-on's `classify`: HLS playlists and media files; DASH, segments
/// and anything else are dropped.
pub fn classify(url: &str, mime: Option<&str>) -> Option<bool> {
    let parsed = Url::parse(url).ok()?;
    if !matches!(parsed.scheme(), "https" | "http") || url.len() > 4096 {
        return None;
    }
    let path = parsed.path().to_ascii_lowercase();
    let ty = mime
        .unwrap_or("")
        .to_ascii_lowercase()
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_owned();
    if path.ends_with(".m3u8") || ty.contains("mpegurl") {
        return Some(true);
    }
    if path.ends_with(".mpd") || ty == "application/dash+xml" {
        return None;
    }
    if [".ts", ".m4s", ".cmfv", ".cmfa"]
        .iter()
        .any(|e| path.ends_with(e))
        || ty == "video/mp2t"
    {
        return None;
    }
    let file_ext = [
        ".mp4", ".webm", ".mov", ".m4v", ".mkv", ".mp3", ".m4a", ".ogg", ".oga", ".wav", ".aac",
        ".flac",
    ]
    .iter()
    .any(|e| path.ends_with(e));
    (file_ext || ty.starts_with("video/") || ty.starts_with("audio/")).then_some(false)
}

fn data_dir(app: &AppHandle) -> std::path::PathBuf {
    app.try_state::<crate::core::paths::Paths>()
        .map(|p| p.local_dir.clone())
        .unwrap_or_else(std::env::temp_dir)
        .join("WebView2")
        .join("video-sniff")
}

/// Loads `page` hidden and collects streams. Returns them plus the `blob:` count.
pub async fn run(
    app: &AppHandle,
    state: &SniffState,
    page: &str,
) -> Result<(Vec<Found>, u32), IpcError> {
    let target: Url = page
        .parse()
        .map_err(|_| IpcError::new("video.bad_url", "invalid url"))?;
    let id = crate::core::id::new_id();
    let (tx, mut rx) = mpsc::unbounded_channel();
    {
        let mut pending = state.pending.lock().unwrap_or_else(PoisonError::into_inner);
        if pending.is_some() {
            return Err(IpcError::new(
                "video.sniff_busy",
                "already looking for streams",
            ));
        }
        *pending = Some((id.clone(), tx));
    }
    let script = include_str!("sniff.js").replace(
        "__PCISLAND_REQUEST__",
        &serde_json::to_string(&id).unwrap_or_default(),
    );
    let built = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::External(target))
        .title("PCisland video sniffer")
        .visible(false)
        .skip_taskbar(true)
        .inner_size(1280.0, 720.0)
        .data_directory(data_dir(app))
        .additional_browser_args(ARGS)
        .initialization_script(script)
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .build();
    let mut last = Report {
        items: Vec::new(),
        blobs: 0,
        done: false,
    };
    let result = match built {
        Ok(window) => {
            let deadline = tokio::time::Instant::now() + TIMEOUT;
            while let Ok(Some(report)) = tokio::time::timeout_at(deadline, rx.recv()).await {
                let done = report.done;
                last = report;
                if done {
                    break;
                }
            }
            let _ = window.destroy();
            Ok(())
        }
        Err(e) => Err(IpcError::from(e)),
    };
    *state.pending.lock().unwrap_or_else(PoisonError::into_inner) = None;
    result?;
    let mut seen = std::collections::HashSet::new();
    let found = last
        .items
        .into_iter()
        .filter_map(|i| {
            let hls = classify(&i.url, i.mime.as_deref())?;
            seen.insert(i.url.clone())
                .then_some(Found { url: i.url, hls })
        })
        .take(30)
        .collect();
    Ok((found, last.blobs))
}

pub fn report(
    state: &SniffState,
    webview: &Webview,
    request_id: &str,
    items: Vec<SniffItem>,
    blobs: u32,
    done: bool,
) -> Result<(), IpcError> {
    let pending = state.pending.lock().unwrap_or_else(PoisonError::into_inner);
    let Some((id, tx)) = pending.as_ref() else {
        return Ok(());
    };
    if id != request_id || webview.label() != LABEL {
        return Err(IpcError::new("video.sniff_sender", "unexpected sender"));
    }
    let _ = tx.send(Report {
        items: items.into_iter().take(40).collect(),
        blobs: blobs.min(10_000),
        done,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::classify;

    #[test]
    fn matches_the_addon_classifier() {
        assert_eq!(
            classify("https://c.example/v/master.m3u8", None),
            Some(true)
        );
        assert_eq!(
            classify("https://c.example/v", Some("application/vnd.apple.mpegurl")),
            Some(true)
        );
        assert_eq!(classify("https://c.example/v.mp4?x=1", None), Some(false));
        assert_eq!(
            classify("https://c.example/play", Some("video/mp4; codecs=avc1")),
            Some(false)
        );
        assert_eq!(classify("https://c.example/seg/001.ts", None), None);
        assert_eq!(classify("https://c.example/a.mpd", None), None);
        assert_eq!(classify("https://c.example/page", Some("text/html")), None);
        assert_eq!(classify("ftp://c.example/a.mp4", None), None);
    }
}
