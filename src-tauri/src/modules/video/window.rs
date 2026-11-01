//! The single video window (docs/modules/video-player.md §5.6): own WebView2 folder with
//! flags that keep playback alive when covered or minimized (spike S2), pin, reset size,
//! remembered bounds.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};

use crate::core::storage::Db;

pub const LABEL: &str = "video";
const GEOMETRY: &str = "video_window.geometry";
const PINNED: &str = "video_window.pinned";
const SAVE_DELAY: Duration = Duration::from_millis(600);
/// App defaults plus: never throttle/suspend media when covered or minimized (S2), and
/// autoplay so island commands work without a click in the window.
const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
--disable-backgrounding-occluded-windows --disable-renderer-backgrounding \
--disable-background-media-suspend --autoplay-policy=no-user-gesture-required";
const MIN_W: f64 = 240.0;
const MIN_H: f64 = 135.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Default size: `width` logical px at 16:9.
pub fn default_size(width: u32, scale: f64) -> (u32, u32) {
    let w = (f64::from(width) * scale).round();
    (w as u32, (w * 9.0 / 16.0).round() as u32)
}

/// Bottom-right of the work area with a margin.
pub fn default_position(work: (i32, i32, i32, i32), size: (u32, u32), scale: f64) -> (i32, i32) {
    let margin = (24.0 * scale) as i32;
    let (left, top, right, bottom) = work;
    (
        (right - size.0 as i32 - margin).max(left),
        (bottom - size.1 as i32 - margin).max(top),
    )
}

/// A saved bound is kept only if its centre is still on a connected monitor.
pub fn on_screen(b: &Bounds, monitors: &[(i32, i32, u32, u32)]) -> bool {
    let (cx, cy) = (b.x + b.width as i32 / 2, b.y + b.height as i32 / 2);
    b.width >= 160
        && b.height >= 90
        && monitors.iter().any(|&(mx, my, mw, mh)| {
            cx >= mx && cx < mx + mw as i32 && cy >= my && cy < my + mh as i32
        })
}

fn data_dir(app: &AppHandle) -> std::path::PathBuf {
    app.try_state::<crate::core::paths::Paths>()
        .map(|p| p.local_dir.clone())
        .unwrap_or_else(std::env::temp_dir)
        .join("WebView2")
        .join("video")
}

pub fn get(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

pub fn pinned(db: &Db) -> bool {
    db.kv_get(PINNED).is_some_and(|v| v == "true")
}

/// Opens (or reuses) the window. Must be called from an async command (03 §3).
pub fn open(app: &AppHandle, db: &Db, default_width: u32) -> tauri::Result<WebviewWindow> {
    if let Some(win) = get(app) {
        win.show()?;
        return Ok(win);
    }
    let win = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("video.html".into()))
        .title("PCisland Video")
        .decorations(false)
        .resizable(true)
        .min_inner_size(MIN_W, MIN_H)
        .inner_size(
            f64::from(default_width),
            f64::from(default_width) * 9.0 / 16.0,
        )
        .skip_taskbar(true)
        .always_on_top(pinned(db))
        .shadow(true)
        .focused(false)
        .visible(false)
        .background_color(tauri::window::Color(0, 0, 0, 255))
        .data_directory(data_dir(app))
        .additional_browser_args(BROWSER_ARGS)
        .build()?;
    place(&win, db, default_width, false);
    win.show()?;
    track(&win, db.clone());
    Ok(win)
}

/// Saved bounds, or the default size at the bottom-right of the monitor under the cursor.
fn place(win: &WebviewWindow, db: &Db, default_width: u32, force_default: bool) {
    let monitors: Vec<_> = win
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| {
            (
                m.position().x,
                m.position().y,
                m.size().width,
                m.size().height,
            )
        })
        .collect();
    let saved: Option<Bounds> = (!force_default)
        .then(|| {
            db.kv_get(GEOMETRY)
                .and_then(|v| serde_json::from_str(&v).ok())
        })
        .flatten();
    if let Some(b) = saved.filter(|b| on_screen(b, &monitors)) {
        let _ = win.set_size(PhysicalSize::new(b.width, b.height));
        let _ = win.set_position(PhysicalPosition::new(b.x, b.y));
        return;
    }
    let (cx, cy) = crate::platform::window::cursor_pos().unwrap_or((0, 0));
    let Some(mon) = crate::platform::window::monitor_at(cx, cy) else {
        let _ = win.center();
        return;
    };
    let scale = win
        .available_monitors()
        .unwrap_or_default()
        .into_iter()
        .find(|m| {
            let p = m.position();
            let s = m.size();
            cx >= p.x && cx < p.x + s.width as i32 && cy >= p.y && cy < p.y + s.height as i32
        })
        .map_or(1.0, |m| m.scale_factor());
    let size = default_size(default_width, scale);
    let w = mon.work_area;
    let (x, y) = default_position((w.left, w.top, w.right, w.bottom), size, scale);
    let _ = win.set_size(PhysicalSize::new(size.0, size.1));
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

/// "Về mặc định": default size, keeping the top-left corner inside the screen.
pub fn reset(win: &WebviewWindow, default_width: u32) {
    let scale = win.scale_factor().unwrap_or(1.0);
    let size = default_size(default_width, scale);
    let pos = win.outer_position().unwrap_or_default();
    let mut x = pos.x;
    let mut y = pos.y;
    if let Some(mon) = crate::platform::window::monitor_at(x, y) {
        let w = mon.work_area;
        x = x.clamp(w.left, (w.right - size.0 as i32).max(w.left));
        y = y.clamp(w.top, (w.bottom - size.1 as i32).max(w.top));
    }
    let _ = win.set_size(PhysicalSize::new(size.0, size.1));
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

pub fn set_pinned(win: &WebviewWindow, db: &Db, pinned: bool) {
    let _ = win.set_always_on_top(pinned);
    let _ = db.kv_set(PINNED, if pinned { "true" } else { "false" });
}

/// Debounced save of the window bounds after a move/resize.
fn track(win: &WebviewWindow, db: Db) {
    let generation = Arc::new(AtomicU64::new(0));
    let w = win.clone();
    win.on_window_event(move |event| {
        if !matches!(event, WindowEvent::Moved(_) | WindowEvent::Resized(_)) {
            return;
        }
        let my = generation.fetch_add(1, Ordering::SeqCst) + 1;
        let (generation, w, db) = (generation.clone(), w.clone(), db.clone());
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(SAVE_DELAY).await;
            if generation.load(Ordering::SeqCst) != my
                || w.is_minimized().unwrap_or(true)
                || w.is_fullscreen().unwrap_or(false)
            {
                return;
            }
            let (Ok(pos), Ok(size)) = (w.outer_position(), w.inner_size()) else {
                return;
            };
            let b = Bounds {
                x: pos.x,
                y: pos.y,
                width: size.width,
                height: size.height,
            };
            if let Ok(json) = serde_json::to_string(&b) {
                let _ = db.kv_set(GEOMETRY, &json);
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_320_wide_16_by_9_at_any_dpi() {
        assert_eq!(default_size(320, 1.0), (320, 180));
        assert_eq!(default_size(320, 1.5), (480, 270));
    }

    #[test]
    fn default_position_is_bottom_right_inside_the_work_area() {
        assert_eq!(
            default_position((0, 0, 1920, 1040), (320, 180), 1.0),
            (1576, 836)
        );
        assert_eq!(
            default_position((1920, 0, 2000, 100), (320, 180), 1.0),
            (1920, 0)
        );
    }

    #[test]
    fn saved_bounds_must_be_on_a_monitor() {
        let b = Bounds {
            x: 1500,
            y: 800,
            width: 320,
            height: 180,
        };
        assert!(on_screen(&b, &[(0, 0, 1920, 1080)]));
        assert!(!on_screen(&Bounds { x: 2500, ..b }, &[(0, 0, 1920, 1080)]));
        assert!(!on_screen(
            &Bounds { width: 50, ..b },
            &[(0, 0, 1920, 1080)]
        ));
    }
}
