//! Web player: a hidden-able browser window for music sites, controlled from the island
//! (docs/modules/web-player.md). P0 uses the documented fallback: a single webview
//! window (no multiwebview toolbar yet).
#![forbid(unsafe_code)]

mod artwork;
pub mod commands;
pub mod dto;
mod search;
mod session;
pub mod sites;

use std::sync::{Arc, Mutex, PoisonError};

use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::ipc::{events, now_ms};
use crate::modules::media::dto::{MediaCaps, NowPlayingDto, PlaybackStatus};
use crate::modules::media::{MediaCommand, MediaHandle, WEB_SOURCE_ID};
use dto::{BridgeMessage, BridgeStatus, Lifecycle, WebPlayerSnapshot};

pub const LABEL: &str = "web-player";
const BRIDGE_JS: &str = include_str!("bridge.js");
/// Tauri's default WebView2 flags plus autoplay without a user gesture, so play commands
/// from the island work while the window is hidden.
const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --autoplay-policy=no-user-gesture-required";

#[derive(Clone)]
pub struct WebPlayerHandle {
    search: Arc<search::SearchState>,
    db: crate::core::storage::Db,
    resume_generation: Arc<std::sync::atomic::AtomicU64>,
    pending_restore: Arc<Mutex<Option<dto::WebSession>>>,
    restoring_title: Arc<Mutex<Option<String>>>,
    state: Arc<Mutex<WebPlayerSnapshot>>,
    media: MediaHandle,
    /// Last time (epoch ms) the page reported playing; drives auto-unload.
    last_playing: Arc<std::sync::atomic::AtomicU64>,
    /// When the window last opened, showed or hid; auto-unload never counts from earlier.
    lifecycle_at: Arc<std::sync::atomic::AtomicU64>,
    artwork_cache: Arc<crate::core::artwork::ArtworkCache>,
    pending_artwork: Arc<Mutex<artwork::PendingArtwork>>,
}

impl WebPlayerHandle {
    fn publish(&self, state: Option<NowPlayingDto>) {
        let mut pending = self
            .pending_artwork
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let request = pending.update(state);
        self.media.update_web_source(pending.snapshot());
        drop(pending);
        if let Some((generation, url)) = request {
            let handle = self.clone();
            tauri::async_runtime::spawn(async move {
                match artwork::download(&handle.artwork_cache, &url).await {
                    Ok(local) => {
                        let mut pending = handle
                            .pending_artwork
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner);
                        if let Some(np) = pending.complete(generation, local) {
                            handle.media.update_web_source(Some(np));
                        }
                    }
                    Err(reason) => {
                        tracing::debug!(module = "web_player", %reason, "cover unavailable")
                    }
                }
            });
        }
    }

    /// A resume or search-play is still waiting for its track to start.
    fn busy(&self) -> bool {
        self.pending_restore
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some()
            || self
                .restoring_title
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .is_some()
            || self.search.busy()
    }

    pub fn snapshot(&self) -> WebPlayerSnapshot {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn update(&self, app: &AppHandle, f: impl FnOnce(&mut WebPlayerSnapshot)) {
        let snap = {
            let mut s = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            let before = s.lifecycle;
            f(&mut s);
            if s.lifecycle != before {
                self.lifecycle_at
                    .store(now_ms(), std::sync::atomic::Ordering::Relaxed);
            }
            s.rev += 1;
            s.clone()
        };
        for label in ["island", "main"] {
            let _ = app.emit_to(label, events::WEB_PLAYER_STATE, &snap);
        }
    }
}

pub fn setup(
    app: &AppHandle,
    db: crate::core::storage::Db,
    media: MediaHandle,
    artwork_cache: Arc<crate::core::artwork::ArtworkCache>,
    settings: tokio::sync::watch::Receiver<crate::core::settings::Settings>,
) -> WebPlayerHandle {
    let history = db
        .kv_get(session::KEY)
        .and_then(|value| serde_json::from_str(&value).ok())
        .unwrap_or_default();
    let handle = WebPlayerHandle {
        search: Arc::default(),
        db,
        resume_generation: Arc::default(),
        pending_restore: Arc::default(),
        restoring_title: Arc::default(),
        state: Arc::new(Mutex::new(WebPlayerSnapshot {
            lifecycle: Lifecycle::Closed,
            site_id: None,
            url: None,
            history,
            queue: Vec::new(),
            repeat: None,
            repeat_error: false,
            rev: 0,
        })),
        media: media.clone(),
        last_playing: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        lifecycle_at: Arc::new(std::sync::atomic::AtomicU64::new(now_ms())),
        artwork_cache,
        pending_artwork: Arc::default(),
    };
    spawn_auto_unload(app.clone(), handle.clone(), settings);
    let app_for_control = app.clone();
    media.register_web_control(Arc::new(move |cmd| {
        let action = match cmd {
            MediaCommand::PlayPause => "play_pause",
            MediaCommand::Pause => "pause",
            MediaCommand::Next => "nexttrack",
            MediaCommand::Previous => "previoustrack",
        };
        if let Some(win) = app_for_control.get_webview_window(LABEL) {
            let _ = win.eval(format!(
                "window.__pcisland && window.__pcisland.exec('{action}')"
            ));
        }
    }));
    handle
}

pub fn open(
    app: &AppHandle,
    handle: &WebPlayerHandle,
    site_id: Option<&str>,
    url: Option<&str>,
) -> Result<(), crate::ipc::IpcError> {
    handle
        .resume_generation
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    *handle
        .restoring_title
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = None;
    open_target(app, handle, site_id, url, true, None)
}

fn open_target(
    app: &AppHandle,
    handle: &WebPlayerHandle,
    site_id: Option<&str>,
    url: Option<&str>,
    visible: bool,
    restore: Option<&dto::WebSession>,
) -> Result<(), crate::ipc::IpcError> {
    let site = site_id.and_then(sites::find);
    let target = match (url, site) {
        (Some(u), _) => u.to_owned(),
        (None, Some(s)) => s.home_url.to_owned(),
        (None, None) => {
            return Err(crate::ipc::IpcError::new(
                "web_player.no_target",
                "site or url required",
            ));
        }
    };
    let parsed: Url = target
        .parse()
        .map_err(|_| crate::ipc::IpcError::new("web_player.bad_url", "invalid url"))?;
    if parsed.scheme() != "https" {
        return Err(crate::ipc::IpcError::new(
            "web_player.bad_url",
            "only https is allowed",
        ));
    }

    *handle
        .pending_restore
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = restore.cloned();
    if let Some(win) = app.get_webview_window(LABEL) {
        win.eval(format!(
            "window.__pcisland ? window.__pcisland.navigate({0}) : location.assign({0})",
            serde_json::to_string(parsed.as_str()).unwrap_or_default()
        ))?;
        win.set_title(&format!("PCisland · {}", site.map_or("Web", |s| s.name)))?;
        handle.publish(None);
        if visible {
            win.show()?;
            win.set_focus()?;
        } else {
            win.hide()?;
        }
    } else {
        let data_dir = web_data_dir();
        let restore = handle.pending_restore.clone();
        let repeat_state = handle.state.clone();
        let win = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::External(parsed.clone()))
            .title(format!("PCisland · {}", site.map_or("Web", |s| s.name)))
            .inner_size(1100.0, 760.0)
            .min_inner_size(720.0, 480.0)
            .center()
            .initialization_script(BRIDGE_JS)
            .on_page_load(move |webview, payload| {
                if payload.event() == tauri::webview::PageLoadEvent::Finished {
                    let mode = repeat_state
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .repeat;
                    if let Some(mode) = mode {
                        let _ = webview.eval(format!(
                            "window.__pcisland?.setRepeat({}, true);",
                            serde_json::to_string(&mode).unwrap_or_default()
                        ));
                    }
                    if let Some(saved) = restore
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .take()
                    {
                        let _ = webview.eval(format!(
                            "window.__pcisland?.resume({});",
                            serde_json::to_string(&saved).unwrap_or_default()
                        ));
                    }
                }
            })
            .visible(visible)
            .additional_browser_args(BROWSER_ARGS)
            .data_directory(data_dir)
            // Sign-in with Google/Facebook/Zalo opens a popup and talks back through
            // `window.opener`; Tauri denies popups by default. WebView2's own popup keeps
            // the opener link and gets no Tauri IPC (docs/modules/web-player.md §5).
            .on_new_window(|url, _| match sites::popup_policy(&url) {
                sites::PopupPolicy::InApp => NewWindowResponse::Allow,
                sites::PopupPolicy::External => {
                    // Ads and other links leave the app instead of piling up windows.
                    let _ = std::process::Command::new("explorer")
                        .arg(url.as_str())
                        .spawn();
                    NewWindowResponse::Deny
                }
                sites::PopupPolicy::Deny => NewWindowResponse::Deny,
            })
            .build()?;
        // Closing the window collapses the player instead of stopping the music
        // (docs/modules/web-player.md §9).
        let app_for_close = app.clone();
        let handle_for_close = handle.clone();
        let win_for_close = win.clone();
        win.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = win_for_close.hide();
                handle_for_close.update(&app_for_close, |s| s.lifecycle = Lifecycle::Hidden);
            }
        });
    }
    handle.update(app, |s| {
        s.lifecycle = if visible {
            Lifecycle::Visible
        } else {
            Lifecycle::Hidden
        };
        s.site_id = site.map(|s| s.id.to_owned());
        s.url = Some(parsed.to_string());
        s.queue.clear();
    });
    Ok(())
}

pub fn resume(
    app: &AppHandle,
    handle: &WebPlayerHandle,
    site_id: &str,
) -> Result<(), crate::ipc::IpcError> {
    let snapshot = handle.snapshot();
    let Some(saved) = snapshot
        .history
        .into_iter()
        .find(|item| item.site_id == site_id)
    else {
        return open(app, handle, Some(site_id), None);
    };
    let page = session::site_url(&saved.page_url, site_id)
        .ok_or_else(|| crate::ipc::IpcError::new("web_player.bad_url", "invalid saved URL"))?;
    let generation = handle
        .resume_generation
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        + 1;
    let has_current_track = handle
        .pending_artwork
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .snapshot()
        .is_some_and(|track| track.title == saved.title);
    if snapshot.site_id.as_deref() == Some(site_id)
        && has_current_track
        && app.get_webview_window(LABEL).is_some()
    {
        let win = app
            .get_webview_window(LABEL)
            .ok_or_else(|| crate::ipc::IpcError::new("web_player.closed", "closed"))?;
        win.eval("window.__pcisland?.exec('play')")?;
        if snapshot.lifecycle != Lifecycle::Visible {
            set_visible(app, handle, false)?;
        }
    } else {
        *handle
            .restoring_title
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(saved.title.clone());
        open_target(app, handle, Some(site_id), Some(&page), false, Some(&saved))?;
    }
    watch_resume(app, handle, generation);
    Ok(())
}

fn watch_resume(app: &AppHandle, handle: &WebPlayerHandle, generation: u64) {
    let app = app.clone();
    let handle = handle.clone();
    let started = now_ms();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        if handle
            .resume_generation
            .load(std::sync::atomic::Ordering::Relaxed)
            == generation
            && (handle
                .last_playing
                .load(std::sync::atomic::Ordering::Relaxed)
                < started
                || handle
                    .restoring_title
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .is_some())
        {
            *handle
                .restoring_title
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = None;
            *handle
                .pending_restore
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = None;
            let _ = set_visible(&app, &handle, true);
        }
    });
}

pub fn select_queue(
    app: &AppHandle,
    handle: &WebPlayerHandle,
    url: &str,
) -> Result<(), crate::ipc::IpcError> {
    if !handle.snapshot().queue.iter().any(|item| item.url == url) {
        return Err(crate::ipc::IpcError::new(
            "web_player.queue",
            "track is no longer in queue",
        ));
    }
    if let Some(win) = app.get_webview_window(LABEL) {
        win.eval(format!(
            "window.__pcisland?.select({})",
            serde_json::to_string(url).unwrap_or_default()
        ))?;
    }
    Ok(())
}

pub fn set_visible(app: &AppHandle, handle: &WebPlayerHandle, visible: bool) -> tauri::Result<()> {
    let Some(win) = app.get_webview_window(LABEL) else {
        return Ok(());
    };
    if visible {
        win.show()?;
        win.set_focus()?;
    } else {
        win.hide()?;
    }
    handle.update(app, |s| {
        s.lifecycle = if visible {
            Lifecycle::Visible
        } else {
            Lifecycle::Hidden
        }
    });
    Ok(())
}

pub fn close(app: &AppHandle, handle: &WebPlayerHandle) -> tauri::Result<()> {
    handle
        .resume_generation
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    *handle
        .pending_restore
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = None;
    *handle
        .restoring_title
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = None;
    if let Some(win) = app.get_webview_window(LABEL) {
        win.destroy()?;
    }
    handle.publish(None);
    handle.update(app, |s| {
        s.lifecycle = Lifecycle::Closed;
        s.url = None;
        s.queue.clear();
    });
    Ok(())
}

/// Validates a bridge message (docs/modules/web-player.md §5) and forwards it to media.
pub fn report(
    app: &AppHandle,
    handle: &WebPlayerHandle,
    page_url: &Url,
    msg: BridgeMessage,
) -> Result<(), crate::ipc::IpcError> {
    let host = page_url.host_str().unwrap_or_default();
    if page_url.scheme() != "https" || !sites::host_allowed(host) {
        return Err(crate::ipc::IpcError::new(
            "web_player.origin",
            "origin not allowed",
        ));
    }
    let site = sites::for_host(host);
    if let Some(site) = site {
        let queue = session::queue(msg.queue.clone(), site.id);
        let page = msg
            .page_url
            .as_deref()
            .and_then(|url| session::site_url(url, site.id))
            .unwrap_or_else(|| page_url.to_string());
        let prior = handle.snapshot();
        let title = truncate(msg.title.clone().unwrap_or_default());
        let previous = prior.history.iter().find(|s| s.site_id == site.id);
        let track_url = msg
            .track_url
            .as_deref()
            .and_then(|url| session::site_url(url, site.id));
        *handle
            .search
            .playing_url
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = if msg.status == BridgeStatus::Playing {
            track_url.clone()
        } else {
            None
        };
        let position = msg
            .position_ms
            .filter(|v| v.is_finite())
            .unwrap_or_else(|| {
                previous
                    .filter(|s| s.title == title)
                    .map_or(0.0, |s| s.position_ms as f64)
            })
            .clamp(0.0, 604_800_000.0) as u64;
        let restoring = {
            let mut expected = handle
                .restoring_title
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if msg.status == BridgeStatus::Playing && expected.as_deref() == Some(&title) {
                *expected = None;
            }
            expected.is_some()
        };
        // Ads are not "the track the user listened to last" (resume rule, web-player §3).
        let save = !restoring
            && !msg.is_ad
            && msg.status != BridgeStatus::None
            && !title.is_empty()
            && prior
                .history
                .iter()
                .find(|s| s.site_id == site.id)
                .is_none_or(|s| {
                    s.page_url != page
                        || s.track_url != track_url
                        || s.title != title
                        || s.queue != queue
                        || s.position_ms.abs_diff(position) >= 5000
                        || (msg.status == BridgeStatus::Paused && s.position_ms != position)
                });
        if save
            || prior.queue != queue
            || prior.url.as_deref() != Some(&page)
            || (msg.repeat.is_some() && prior.repeat != msg.repeat)
            || prior.repeat_error != msg.repeat_error
        {
            handle.update(app, |state| {
                state.site_id = Some(site.id.into());
                state.url = Some(page.clone());
                state.queue = queue.clone();
                if msg.repeat.is_some() {
                    state.repeat = msg.repeat;
                }
                state.repeat_error = msg.repeat_error;
                if save {
                    state.history.retain(|s| s.site_id != site.id);
                    state.history.push(dto::WebSession {
                        site_id: site.id.into(),
                        page_url: page,
                        title,
                        track_url,
                        position_ms: position,
                        queue,
                    });
                }
            });
            if save {
                let _ = handle.db.kv_set(
                    session::KEY,
                    &serde_json::to_string(&handle.snapshot().history).unwrap_or_default(),
                );
            }
        }
    }
    let state = match msg.status {
        BridgeStatus::None => None,
        status => Some(NowPlayingDto {
            source_id: WEB_SOURCE_ID.to_owned(),
            app_name: site.map_or_else(|| host.to_owned(), |s| s.name.to_owned()),
            title: truncate(msg.title.unwrap_or_default()),
            artist: msg.artist.map(truncate),
            album: msg.album.map(truncate),
            artwork_url: msg.artwork.as_deref().and_then(artwork::remote_url),
            status: if status == BridgeStatus::Playing {
                PlaybackStatus::Playing
            } else {
                PlaybackStatus::Paused
            },
            position_ms: msg.position_ms.map(|v| v.max(0.0) as u64),
            position_at: now_ms(),
            duration_ms: msg
                .duration_ms
                .filter(|d| d.is_finite() && *d > 0.0)
                .map(|v| v as u64),
            rate: msg.rate.clamp(0.0, 16.0),
            caps: MediaCaps {
                play_pause: true,
                next: msg.caps.next,
                previous: msg.caps.previous,
                seek: msg.caps.seek,
            },
            is_ad: msg.is_ad,
        }),
    };
    if state
        .as_ref()
        .is_some_and(|s| s.status == PlaybackStatus::Playing)
    {
        handle
            .last_playing
            .store(now_ms(), std::sync::atomic::Ordering::Relaxed);
    }
    handle.publish(state);
    Ok(())
}

fn truncate(s: String) -> String {
    s.chars().take(512).collect()
}

fn web_data_dir() -> std::path::PathBuf {
    let base =
        std::env::var_os("LOCALAPPDATA").map_or_else(std::env::temp_dir, std::path::PathBuf::from);
    base.join("PCisland").join("WebView2").join("web-player")
}

/// Destroys a hidden, paused player after `web_player.unload_after_paused_min` to give
/// its RAM back (docs/modules/web-player.md §3, step 6). 0 disables it.
fn spawn_auto_unload(
    app: AppHandle,
    handle: WebPlayerHandle,
    settings: tokio::sync::watch::Receiver<crate::core::settings::Settings>,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            let minutes = u64::from(settings.borrow().web_player.unload_after_paused_min);
            let hidden = handle.snapshot().lifecycle == Lifecycle::Hidden;
            let last_playing = handle
                .last_playing
                .load(std::sync::atomic::Ordering::Relaxed);
            let lifecycle_at = handle
                .lifecycle_at
                .load(std::sync::atomic::Ordering::Relaxed);
            if should_unload(
                minutes,
                hidden,
                handle.busy(),
                last_playing,
                lifecycle_at,
                now_ms(),
            ) {
                tracing::info!(module = "web_player", "unloading idle web player");
                let _ = close(&app, &handle);
            }
        }
    });
}

/// Idle = no playback AND no open/show/hide for the whole period. A player that was just
/// hidden, or is still restoring/searching, is never unloaded.
fn should_unload(
    minutes: u64,
    hidden: bool,
    busy: bool,
    last_playing: u64,
    lifecycle_at: u64,
    now: u64,
) -> bool {
    minutes > 0
        && hidden
        && !busy
        && now.saturating_sub(last_playing.max(lifecycle_at)) > minutes * 60_000
}

/// Back / forward / reload / home inside the player page.
pub fn navigate(
    app: &AppHandle,
    handle: &WebPlayerHandle,
    action: &str,
) -> Result<(), crate::ipc::IpcError> {
    let Some(win) = app.get_webview_window(LABEL) else {
        return Err(crate::ipc::IpcError::new(
            "web_player.closed",
            "player is not open",
        ));
    };
    match action {
        "back" => win.eval("history.back()")?,
        "forward" => win.eval("history.forward()")?,
        "reload" => win.eval("location.reload()")?,
        "home" => {
            let site = handle.snapshot().site_id.and_then(|id| sites::find(&id));
            if let Some(site) = site {
                win.navigate(
                    site.home_url
                        .parse()
                        .map_err(|_| crate::ipc::IpcError::new("web_player.bad_url", "home"))?,
                )?;
            }
        }
        other => return Err(crate::ipc::IpcError::new("web_player.bad_action", other)),
    }
    Ok(())
}

/// Signs out of every site: closes the player and deletes its WebView2 profile.
pub fn clear_data(app: &AppHandle, handle: &WebPlayerHandle) -> Result<(), crate::ipc::IpcError> {
    close(app, handle)?;
    let dir = web_data_dir();
    // WebView2 releases its files shortly after the window is destroyed.
    for _ in 0..10 {
        if !dir.exists() || std::fs::remove_dir_all(&dir).is_ok() {
            let _ = handle.db.kv_set(session::KEY, "[]");
            handle.update(app, |s| {
                s.history.clear();
                s.queue.clear();
            });
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    Err(crate::ipc::IpcError::new(
        "web_player.clear_failed",
        "profile is still in use",
    ))
}

#[cfg(test)]
mod unload_tests {
    use super::should_unload;

    const MIN: u64 = 60_000;

    #[test]
    fn a_freshly_hidden_player_is_kept_even_if_it_never_played() {
        // Opened, signed in, hidden one minute ago, never played (last_playing = 0).
        assert!(!should_unload(30, true, false, 0, 100 * MIN, 101 * MIN));
    }

    #[test]
    fn unloads_only_after_the_whole_period_hidden_and_paused() {
        assert!(should_unload(30, true, false, 10 * MIN, 20 * MIN, 51 * MIN));
        assert!(!should_unload(
            30,
            true,
            false,
            40 * MIN,
            20 * MIN,
            51 * MIN
        ));
    }

    #[test]
    fn never_unloads_while_visible_restoring_or_disabled() {
        assert!(!should_unload(30, false, false, 0, 0, 999 * MIN));
        assert!(!should_unload(30, true, true, 0, 0, 999 * MIN));
        assert!(!should_unload(0, true, false, 0, 0, 999 * MIN));
    }
}
