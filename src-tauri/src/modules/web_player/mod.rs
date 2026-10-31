//! Web player: a hidden-able browser window for music sites, controlled from the island
//! (docs/modules/web-player.md). P0 uses the documented fallback: a single webview
//! window (no multiwebview toolbar yet).
#![forbid(unsafe_code)]

mod artwork;
pub mod commands;
pub mod dto;
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
    state: Arc<Mutex<WebPlayerSnapshot>>,
    media: MediaHandle,
    /// Last time (epoch ms) the page reported playing; drives auto-unload.
    last_playing: Arc<std::sync::atomic::AtomicU64>,
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

    pub fn snapshot(&self) -> WebPlayerSnapshot {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn update(&self, app: &AppHandle, f: impl FnOnce(&mut WebPlayerSnapshot)) {
        let snap = {
            let mut s = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            f(&mut s);
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
    media: MediaHandle,
    artwork_cache: Arc<crate::core::artwork::ArtworkCache>,
    settings: tokio::sync::watch::Receiver<crate::core::settings::Settings>,
) -> WebPlayerHandle {
    let handle = WebPlayerHandle {
        state: Arc::new(Mutex::new(WebPlayerSnapshot {
            lifecycle: Lifecycle::Closed,
            site_id: None,
            url: None,
            rev: 0,
        })),
        media: media.clone(),
        last_playing: Arc::new(std::sync::atomic::AtomicU64::new(0)),
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

    if let Some(win) = app.get_webview_window(LABEL) {
        win.navigate(parsed.clone())?;
        handle.publish(None);
        win.show()?;
        win.set_focus()?;
    } else {
        let data_dir = web_data_dir();
        let win = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::External(parsed.clone()))
            .title(format!("PCisland · {}", site.map_or("Web", |s| s.name)))
            .inner_size(1100.0, 760.0)
            .min_inner_size(720.0, 480.0)
            .center()
            .initialization_script(BRIDGE_JS)
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
        s.lifecycle = Lifecycle::Visible;
        s.site_id = site.map(|s| s.id.to_owned());
        s.url = Some(parsed.to_string());
    });
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
    if let Some(win) = app.get_webview_window(LABEL) {
        win.destroy()?;
    }
    handle.publish(None);
    handle.update(app, |s| {
        s.lifecycle = Lifecycle::Closed;
        s.url = None;
    });
    Ok(())
}

/// Validates a bridge message (docs/modules/web-player.md §5) and forwards it to media.
pub fn report(
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
            let last = handle
                .last_playing
                .load(std::sync::atomic::Ordering::Relaxed);
            if minutes > 0 && hidden && now_ms().saturating_sub(last) > minutes * 60_000 {
                tracing::info!(module = "web_player", "unloading idle web player");
                let _ = close(&app, &handle);
            }
        }
    });
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
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    Err(crate::ipc::IpcError::new(
        "web_player.clear_failed",
        "profile is still in use",
    ))
}
