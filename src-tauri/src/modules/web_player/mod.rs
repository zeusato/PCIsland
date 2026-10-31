//! Web player: a hidden-able browser window for music sites, controlled from the island
//! (docs/modules/web-player.md). P0 uses the documented fallback: a single webview
//! window (no multiwebview toolbar yet).
#![forbid(unsafe_code)]

pub mod commands;
pub mod dto;
pub mod sites;

use std::sync::{Arc, Mutex, PoisonError};

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
}

impl WebPlayerHandle {
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

pub fn setup(app: &AppHandle, media: MediaHandle) -> WebPlayerHandle {
    let handle = WebPlayerHandle {
        state: Arc::new(Mutex::new(WebPlayerSnapshot {
            lifecycle: Lifecycle::Closed,
            site_id: None,
            url: None,
            rev: 0,
        })),
        media: media.clone(),
    };
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
    handle.media.update_web_source(None);
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
            // Remote artwork is shown directly for now; downloading into ArtworkCache is P2.
            artwork_url: msg
                .artwork
                .filter(|u| u.starts_with("https://"))
                .map(truncate),
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
    handle.media.update_web_source(state);
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
