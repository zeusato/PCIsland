//! Floating video player (F46, docs/modules/video-player.md, ADR-018).
//! B (main): yt-dlp / page sniffing → `vstream://` proxy → our own player page.
//! A (fallback): the site's official embed in the same window.
#![forbid(unsafe_code)]

pub mod commands;
pub mod dto;
pub mod link;
pub mod progress;
pub mod proxy;
pub mod resolve;
pub mod sniff;
pub mod tools;
pub mod window;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use tauri::{AppHandle, Emitter, WindowEvent};
use tokio::sync::watch;

use crate::core::artwork::ArtworkCache;
use crate::core::settings::Settings;
use crate::core::storage::Db;
use crate::ipc::{IpcError, now_ms};
use crate::modules::media::dto::{MediaCaps, NowPlayingDto, PlaybackStatus};
use crate::modules::media::{MediaCommand, MediaHandle};
use dto::{
    PlaybackSource, SniffedStream, SourceKind, VideoCommand, VideoOpenResult, VideoReport,
    VideoSnapshot, VideoStatus,
};
use link::LinkKind;
use resolve::{Headers, Plan};

pub const SOURCE_ID: &str = "video";
pub const EVENT_SOURCE: &str = "video://source";
pub const EVENT_COMMAND: &str = "video://command";
pub const EVENT_STATE: &str = "video://state";
/// A fresh link at most once per minute (expired signed URLs, docs §5.4).
const REFRESH_EVERY_MS: u64 = 60_000;

#[derive(Default)]
struct State {
    link: Option<LinkKind>,
    /// Parsed yt-dlp output, kept for quality changes without resolving again.
    info: Option<Arc<resolve::Info>>,
    /// Streams a sniffed page offered, by id.
    sniffed: Vec<(String, sniff::Found)>,
    source: Option<PlaybackSource>,
    thumbnail: Option<String>,
    status: Option<VideoStatus>,
    muted: bool,
    last_refresh: u64,
    /// The real stream of a direct/sniffed source, to renew it after expiry.
    stream: Option<(String, bool)>,
    /// Last time the position was saved (`progress`), at most every 5 s.
    last_saved: u64,
    rev: u64,
}

/// Explicit `t=` wins; otherwise where this video was left last time.
fn start_for(db: &Db, link: &LinkKind) -> (u64, Option<u64>) {
    if let Some(s) = link.start_s() {
        return (u64::from(s) * 1000, None);
    }
    match progress::load(db, &progress::key(link)) {
        Some(ms) => (ms, Some(ms)),
        None => (0, None),
    }
}

#[derive(Clone)]
pub struct VideoHandle(Arc<Inner>);

struct Inner {
    app: AppHandle,
    db: Db,
    media: MediaHandle,
    proxy: Arc<proxy::Proxy>,
    artwork: Arc<ArtworkCache>,
    settings: watch::Receiver<Settings>,
    tools_dir: PathBuf,
    sniff: sniff::SniffState,
    state: Mutex<State>,
}

pub fn setup(
    app: &AppHandle,
    db: Db,
    media: MediaHandle,
    proxy: Arc<proxy::Proxy>,
    artwork: Arc<ArtworkCache>,
    settings: watch::Receiver<Settings>,
    local_dir: PathBuf,
) -> VideoHandle {
    let handle = VideoHandle(Arc::new(Inner {
        app: app.clone(),
        db,
        media: media.clone(),
        proxy,
        artwork,
        settings,
        tools_dir: local_dir.join("tools"),
        sniff: sniff::SniffState::default(),
        state: Mutex::default(),
    }));
    let for_control = handle.clone();
    media.register_control(
        SOURCE_ID,
        Arc::new(move |cmd| {
            let action = match cmd {
                MediaCommand::PlayPause => "play_pause",
                MediaCommand::Pause => "pause",
                // No playlist: next/previous jump ±10 s.
                MediaCommand::Next => "forward",
                MediaCommand::Previous => "back",
            };
            for_control.command(action, None);
        }),
    );
    handle
}

fn embed_url(link: &LinkKind, start_ms: u64) -> Option<String> {
    let start_s = (start_ms >= 1000).then_some(start_ms / 1000);
    match link {
        LinkKind::Youtube { id, .. } => Some(format!(
            "https://www.youtube-nocookie.com/embed/{id}?autoplay=1&playsinline=1&rel=0{}",
            start_s.map(|s| format!("&start={s}")).unwrap_or_default()
        )),
        LinkKind::Facebook { url } => {
            let mut u = tauri::Url::parse("https://www.facebook.com/plugins/video.php").ok()?;
            u.query_pairs_mut()
                .append_pair("href", url)
                .append_pair("show_text", "false")
                .append_pair("autoplay", "true");
            Some(u.to_string())
        }
        _ => None,
    }
}

impl VideoHandle {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.0.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn snapshot(&self) -> VideoSnapshot {
        let s = self.lock();
        VideoSnapshot {
            open: window::get(&self.0.app).is_some(),
            pinned: window::pinned(&self.0.db),
            muted: s.muted,
            title: s.source.as_ref().map(|x| x.title.clone()),
            status: s.status,
            rev: s.rev,
        }
    }

    fn publish(&self) {
        let snap = self.snapshot();
        for label in ["island", "main", window::LABEL] {
            let _ = self.0.app.emit_to(label, EVENT_STATE, &snap);
        }
    }

    pub fn current(&self) -> Option<PlaybackSource> {
        self.lock().source.clone()
    }

    pub fn tools_status(&self) -> tools::ToolsStatus {
        tools::status(&self.0.db, &self.0.tools_dir)
    }

    pub async fn install_tools(&self) -> Result<tools::ToolsStatus, IpcError> {
        tools::install(&self.0.db, &self.0.tools_dir)
            .await
            .map_err(|e| IpcError::new("video.tools_install", e))
    }

    /// Sends an island/media-key command to the player page.
    pub fn command(&self, action: &str, value: Option<f64>) {
        if let Some(win) = window::get(&self.0.app) {
            let _ = win.emit_to(
                window::LABEL,
                EVENT_COMMAND,
                VideoCommand {
                    action: action.into(),
                    value,
                },
            );
        }
    }

    fn max_height(&self) -> u32 {
        self.0.settings.borrow().video.max_height
    }

    fn default_width(&self) -> u32 {
        self.0.settings.borrow().video.default_width
    }

    /// yt-dlp with one forced tool update on failure (YouTube changes often).
    async fn resolve_with_tools(&self, url: &str) -> Result<resolve::Info, IpcError> {
        let attempt = |dir: PathBuf, url: String| async move {
            tauri::async_runtime::spawn_blocking(move || tools::dump_json(&dir, &url))
                .await
                .map_err(|e| e.to_string())?
        };
        let json = match attempt(self.0.tools_dir.clone(), url.to_owned()).await {
            Ok(json) => json,
            Err(e) if e == "tools_missing" => return Err(IpcError::new("video.tools_missing", e)),
            Err(first) => {
                tracing::warn!(module = "video", error = %first, "yt-dlp failed; trying an update");
                if !tools::update_if_due(&self.0.db, &self.0.tools_dir, true).await {
                    return Err(IpcError::new("video.resolve_failed", first));
                }
                attempt(self.0.tools_dir.clone(), url.to_owned())
                    .await
                    .map_err(|e| IpcError::new("video.resolve_failed", e))?
            }
        };
        serde_json::from_str(&json)
            .map_err(|e| IpcError::new("video.resolve_failed", e.to_string()))
    }

    /// Turns a plan into proxied URLs. Clears previous proxy sessions first.
    fn source_from_plan(
        &self,
        resolved: &resolve::Resolved,
        page_url: &str,
        start_ms: u64,
        quality: Option<u32>,
    ) -> PlaybackSource {
        self.0.proxy.clear();
        let reg = |s: &resolve::Stream| self.0.proxy.register(&s.url, &s.headers);
        let mut audio_languages = std::collections::BTreeMap::new();
        let (kind, video, audio, quality) = match &resolved.plan {
            Plan::Hls { manifest } => (SourceKind::Hls, reg(manifest), None, quality),
            Plan::Pair {
                video,
                audio,
                languages,
                height,
            } => {
                let default = reg(audio);
                for (language, track) in languages {
                    let url = if track == audio {
                        default.clone()
                    } else {
                        reg(track)
                    };
                    audio_languages.insert(language.clone(), url);
                }
                (
                    SourceKind::Pair,
                    reg(video),
                    Some(default),
                    quality.map(|_| *height),
                )
            }
            Plan::Single { stream } => (SourceKind::Single, reg(stream), None, None),
        };
        PlaybackSource {
            kind,
            video,
            audio,
            audio_languages,
            auto_max_height: self.max_height(),
            title: if resolved.title.is_empty() {
                page_url.to_owned()
            } else {
                resolved.title.clone()
            },
            uploader: resolved.uploader.clone(),
            duration_ms: resolved.duration_s.map(|d| (d * 1000.0) as u64),
            is_live: resolved.is_live,
            qualities: resolved.qualities.clone(),
            quality,
            start_ms,
            page_url: page_url.to_owned(),
            resumed_from_ms: None,
            rev: 0,
        }
    }

    fn direct_source(
        &self,
        url: &str,
        hls: bool,
        referer: Option<&str>,
        title: &str,
        start_ms: u64,
    ) -> PlaybackSource {
        self.0.proxy.clear();
        self.lock().stream = Some((url.to_owned(), hls));
        let mut headers: Headers = vec![(
            "User-Agent".into(),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/156.0.0.0 Safari/537.36".into(),
        )];
        if let Some(r) = referer {
            headers.push(("Referer".into(), r.into()));
        }
        PlaybackSource {
            kind: if hls {
                SourceKind::Hls
            } else {
                SourceKind::Single
            },
            video: self.0.proxy.register(url, &headers),
            audio: None,
            audio_languages: Default::default(),
            auto_max_height: self.max_height(),
            title: title.to_owned(),
            uploader: None,
            duration_ms: None,
            is_live: false,
            qualities: Vec::new(),
            quality: None,
            start_ms,
            page_url: referer.unwrap_or(url).to_owned(),
            resumed_from_ms: None,
            rev: 0,
        }
    }

    /// Shows `source` in the (single) window and tells everyone.
    async fn play(
        &self,
        mut source: PlaybackSource,
        thumbnail: Option<String>,
    ) -> Result<(), IpcError> {
        {
            let mut s = self.lock();
            s.rev += 1;
            source.rev = s.rev;
            s.source = Some(source.clone());
            s.status = Some(VideoStatus::Loading);
            s.thumbnail = None;
        }
        let fresh = window::get(&self.0.app).is_none();
        let win = window::open(&self.0.app, &self.0.db, self.default_width())?;
        if fresh {
            let me = self.clone();
            win.on_window_event(move |event| {
                if matches!(event, WindowEvent::Destroyed) {
                    me.closed();
                }
            });
        }
        let _ = win.emit_to(window::LABEL, EVENT_SOURCE, &source);
        self.publish();
        if let Some(url) = thumbnail {
            let me = self.clone();
            let rev = source.rev;
            tauri::async_runtime::spawn(async move {
                if let Ok(local) = crate::core::artwork::download_remote(&me.0.artwork, &url).await
                {
                    let mut s = me.lock();
                    if s.rev == rev {
                        s.thumbnail = Some(local);
                    }
                }
            });
        }
        Ok(())
    }

    pub async fn open(&self, raw: &str, embed: bool) -> Result<VideoOpenResult, IpcError> {
        let link = link::classify(raw)
            .ok_or_else(|| IpcError::new("video.unsupported_link", "unsupported link"))?;
        let use_embed = embed || self.0.settings.borrow().video.engine == "embed";
        {
            let mut s = self.lock();
            s.link = Some(link.clone());
            s.info = None;
            s.sniffed.clear();
            s.stream = None;
        }
        let (start_ms, resumed_from_ms) = start_for(&self.0.db, &link);
        match &link {
            LinkKind::Youtube { .. } | LinkKind::Facebook { .. } if use_embed => {
                let url = embed_url(&link, start_ms)
                    .ok_or_else(|| IpcError::new("video.unsupported_link", "no embed"))?;
                self.0.proxy.clear();
                let source = PlaybackSource {
                    kind: SourceKind::Embed,
                    video: url,
                    audio: None,
                    audio_languages: Default::default(),
                    auto_max_height: self.max_height(),
                    title: link.source_url(),
                    uploader: None,
                    duration_ms: None,
                    is_live: false,
                    qualities: Vec::new(),
                    quality: None,
                    start_ms,
                    page_url: link.source_url(),
                    resumed_from_ms,
                    rev: 0,
                };
                self.play(source, None).await?;
            }
            LinkKind::Youtube { .. } | LinkKind::Facebook { .. } => {
                if !self.tools_status().installed {
                    return Ok(VideoOpenResult::NeedsTools);
                }
                let page = link.source_url();
                let info = Arc::new(self.resolve_with_tools(&page).await?);
                let resolved = resolve::plan(&info, self.max_height(), None).ok_or_else(|| {
                    IpcError::new(
                        "video.no_stream",
                        "no playable stream (DRM or unsupported codecs)",
                    )
                })?;
                let mut source = self.source_from_plan(&resolved, &page, start_ms, None);
                source.resumed_from_ms = resumed_from_ms;
                self.lock().info = Some(info);
                self.play(source, resolved.thumbnail.clone()).await?;
                if self.0.settings.borrow().video.auto_update_tools {
                    let (db, dir) = (self.0.db.clone(), self.0.tools_dir.clone());
                    tauri::async_runtime::spawn(async move {
                        tools::update_if_due(&db, &dir, false).await;
                    });
                }
            }
            LinkKind::Direct { url, hls } => {
                let title = url
                    .rsplit('/')
                    .next()
                    .unwrap_or(url)
                    .split('?')
                    .next()
                    .unwrap_or(url)
                    .to_owned();
                let mut source = self.direct_source(url, *hls, None, &title, start_ms);
                source.resumed_from_ms = resumed_from_ms;
                self.play(source, None).await?;
            }
            LinkKind::Page { url } => {
                let (found, blobs) = sniff::run(&self.0.app, &self.0.sniff, url).await?;
                if found.is_empty() {
                    return Err(IpcError::new(
                        if blobs > 0 {
                            "video.sniff_blob_only"
                        } else {
                            "video.sniff_none"
                        },
                        "no stream found on the page",
                    ));
                }
                let streams: Vec<SniffedStream> = found
                    .iter()
                    .enumerate()
                    .map(|(i, f)| SniffedStream {
                        id: i.to_string(),
                        label: f
                            .url
                            .rsplit('/')
                            .next()
                            .unwrap_or(&f.url)
                            .split('?')
                            .next()
                            .unwrap_or("")
                            .chars()
                            .take(80)
                            .collect(),
                        hls: f.hls,
                    })
                    .collect();
                let single = found.len() == 1;
                self.lock().sniffed = found
                    .into_iter()
                    .enumerate()
                    .map(|(i, f)| (i.to_string(), f))
                    .collect();
                if single {
                    self.open_stream("0").await?;
                } else {
                    return Ok(VideoOpenResult::Choose { streams });
                }
            }
        }
        Ok(VideoOpenResult::Playing)
    }

    /// Plays one of the streams a sniffed page offered.
    pub async fn open_stream(&self, id: &str) -> Result<(), IpcError> {
        let (url, hls, page) = {
            let s = self.lock();
            let f = s
                .sniffed
                .iter()
                .find(|(i, _)| i == id)
                .map(|(_, f)| (f.url.clone(), f.hls))
                .ok_or_else(|| IpcError::new("video.stream_gone", "stream no longer offered"))?;
            (f.0, f.1, s.link.clone())
        };
        let (start_ms, resumed_from_ms) = page
            .as_ref()
            .map_or((0, None), |l| start_for(&self.0.db, l));
        let page = page.as_ref().map(LinkKind::source_url);
        let title = page.clone().unwrap_or_else(|| url.clone());
        let mut source = self.direct_source(&url, hls, page.as_deref(), &title, start_ms);
        source.resumed_from_ms = resumed_from_ms;
        self.play(source, None).await
    }

    /// Resolves again (expired URL or a quality change) and resumes at `position_ms`.
    pub async fn reload(
        &self,
        position_ms: u64,
        quality: Option<Option<u32>>,
    ) -> Result<PlaybackSource, IpcError> {
        let (link, info, current) = {
            let mut s = self.lock();
            let now = now_ms();
            if quality.is_none() {
                if now.saturating_sub(s.last_refresh) < REFRESH_EVERY_MS {
                    return Err(IpcError::new(
                        "video.refresh_throttled",
                        "refreshed less than a minute ago",
                    ));
                }
                s.last_refresh = now;
            }
            (s.link.clone(), s.info.clone(), s.source.clone())
        };
        let link = link.ok_or_else(|| IpcError::new("video.closed", "nothing is playing"))?;
        let current = current.ok_or_else(|| IpcError::new("video.closed", "nothing is playing"))?;
        let page = link.source_url();
        let mut source = match (&link, quality) {
            (LinkKind::Youtube { .. } | LinkKind::Facebook { .. }, Some(want)) => {
                // Quality change: reuse the parsed formats; `None` = automatic (HLS).
                let info =
                    info.ok_or_else(|| IpcError::new("video.closed", "nothing is playing"))?;
                let resolved = resolve::plan(&info, self.max_height(), want)
                    .ok_or_else(|| IpcError::new("video.no_stream", "quality unavailable"))?;
                self.source_from_plan(&resolved, &page, position_ms, want)
            }
            (LinkKind::Youtube { .. } | LinkKind::Facebook { .. }, None) => {
                let info = Arc::new(self.resolve_with_tools(&page).await?);
                let resolved = resolve::plan(&info, self.max_height(), current.quality)
                    .ok_or_else(|| IpcError::new("video.no_stream", "no playable stream"))?;
                self.lock().info = Some(info);
                self.source_from_plan(&resolved, &page, position_ms, current.quality)
            }
            (LinkKind::Page { url }, _) => {
                // Sniff the original page again; keep the same stream when it is offered.
                let previous = self.lock().stream.clone();
                let (found, _) = sniff::run(&self.0.app, &self.0.sniff, url).await?;
                let path = |u: &str| tauri::Url::parse(u).map(|x| x.path().to_owned()).ok();
                let pick = found
                    .iter()
                    .find(|f| {
                        previous
                            .as_ref()
                            .is_some_and(|(p, _)| path(p) == path(&f.url))
                    })
                    .or_else(|| found.first())
                    .ok_or_else(|| {
                        IpcError::new("video.sniff_none", "the page no longer offers a stream")
                    })?;
                self.direct_source(&pick.url, pick.hls, Some(url), &current.title, position_ms)
            }
            (LinkKind::Direct { url, hls }, _) => {
                // A direct link cannot be renewed; register it again (transient failures).
                self.direct_source(url, *hls, None, &current.title, position_ms)
            }
        };
        let mut s = self.lock();
        s.rev += 1;
        source.rev = s.rev;
        s.source = Some(source.clone());
        Ok(source)
    }

    /// State reported by the player page → media hub source `video`.
    pub fn report(&self, r: VideoReport) {
        let np = {
            let mut s = self.lock();
            let changed = s.status != Some(r.status) || s.muted != r.muted;
            s.status = Some(r.status);
            s.muted = r.muted;
            let now = now_ms();
            let live = s.source.as_ref().is_some_and(|x| x.is_live);
            if let (Some(link), false) = (s.link.clone(), live) {
                if changed || now.saturating_sub(s.last_saved) >= 5_000 {
                    s.last_saved = now;
                    progress::save(
                        &self.0.db,
                        &progress::key(&link),
                        r.position_ms.max(0.0) as u64,
                        r.duration_ms
                            .filter(|d| d.is_finite() && *d > 0.0)
                            .map(|d| d as u64),
                        r.status == VideoStatus::Ended,
                    );
                }
            }
            let np = s.source.as_ref().map(|src| NowPlayingDto {
                source_id: SOURCE_ID.into(),
                app_name: "Video".into(),
                title: src.title.clone(),
                artist: src.uploader.clone(),
                album: None,
                artwork_url: s.thumbnail.clone(),
                status: match r.status {
                    VideoStatus::Playing => PlaybackStatus::Playing,
                    VideoStatus::Loading => PlaybackStatus::Changing,
                    _ => PlaybackStatus::Paused,
                },
                position_ms: Some(r.position_ms.max(0.0) as u64),
                position_at: now_ms(),
                duration_ms: r
                    .duration_ms
                    .filter(|d| d.is_finite() && *d > 0.0)
                    .map(|d| d as u64)
                    .or(src.duration_ms),
                rate: 1.0,
                caps: MediaCaps {
                    play_pause: true,
                    next: !src.is_live,
                    previous: !src.is_live,
                    seek: !src.is_live,
                },
                is_ad: false,
            });
            (np, changed)
        };
        self.0.media.update_source(SOURCE_ID, np.0);
        if np.1 {
            self.publish();
        }
    }

    pub fn window_action(&self, action: &str) -> Result<(), IpcError> {
        let win = window::get(&self.0.app)
            .ok_or_else(|| IpcError::new("video.closed", "no video window"))?;
        match action {
            "pin" => window::set_pinned(&win, &self.0.db, true),
            "unpin" => window::set_pinned(&win, &self.0.db, false),
            "reset" => window::reset(&win, self.default_width()),
            "show" => {
                win.show()?;
                win.unminimize()?;
                win.set_focus()?;
            }
            "close" => win.destroy()?,
            other => return Err(IpcError::new("video.bad_action", other)),
        }
        self.publish();
        Ok(())
    }

    /// Opens the link the user pasted in the default browser.
    pub fn open_in_browser(&self) -> Result<(), IpcError> {
        let page = self
            .lock()
            .source
            .as_ref()
            .map(|s| s.page_url.clone())
            .ok_or_else(|| IpcError::new("video.closed", "nothing is playing"))?;
        let url =
            tauri::Url::parse(&page).map_err(|_| IpcError::new("video.bad_url", "invalid url"))?;
        if !matches!(url.scheme(), "https" | "http") {
            return Err(IpcError::new("video.bad_url", "only web links"));
        }
        std::process::Command::new("explorer")
            .arg(url.as_str())
            .spawn()
            .map_err(|e| IpcError::new("video.open_failed", e.to_string()))?;
        Ok(())
    }

    fn closed(&self) {
        self.0.proxy.clear();
        {
            let mut s = self.lock();
            *s = State {
                rev: s.rev + 1,
                ..State::default()
            };
        }
        self.0.media.update_source(SOURCE_ID, None);
        self.publish();
    }

    pub fn sniff_state(&self) -> &sniff::SniffState {
        &self.0.sniff
    }
}

/// Reads a copied link for the panel; only links the player accepts are returned.
pub fn clipboard_link() -> Option<String> {
    let text = crate::platform::clipboard::read_text(2048)?;
    let text = text.trim();
    link::classify(text).map(|_| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_urls_for_the_fallback() {
        let yt = embed_url(
            &LinkKind::Youtube {
                id: "aqz-KE-bpKQ".into(),
                start_s: Some(30),
            },
            30_000,
        )
        .unwrap();
        assert_eq!(
            yt,
            "https://www.youtube-nocookie.com/embed/aqz-KE-bpKQ?autoplay=1&playsinline=1&rel=0&start=30"
        );
        let fb = embed_url(
            &LinkKind::Facebook {
                url: "https://www.facebook.com/x/videos/1/".into(),
            },
            0,
        )
        .unwrap();
        assert!(fb.starts_with("https://www.facebook.com/plugins/video.php?href=https%3A%2F%2Fwww.facebook.com%2Fx%2Fvideos%2F1%2F"));
        assert!(
            embed_url(
                &LinkKind::Page {
                    url: "https://e.com".into()
                },
                0
            )
            .is_none()
        );
    }
}
