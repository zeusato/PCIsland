//! Remembers the full window's position, size and maximized state across launches
//! (docs/modules/full-window.md §5).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent};

use crate::core::storage::Db;

const KEY: &str = "main_window.geometry";
const SAVE_DELAY: Duration = Duration::from_millis(600);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

/// Accepts a saved geometry only if its title bar still lands on a connected monitor.
pub fn visible_on(g: &Geometry, monitors: &[(i32, i32, u32, u32)]) -> bool {
    let (tx, ty) = (g.x + (g.width as i32).min(200) / 2, g.y + 16);
    g.width >= 200
        && g.height >= 200
        && monitors.iter().any(|&(mx, my, mw, mh)| {
            tx >= mx && tx < mx + mw as i32 && ty >= my && ty < my + mh as i32
        })
}

/// Applies the saved geometry to a hidden, freshly built window, then starts tracking it.
pub fn restore_and_track(window: &WebviewWindow) {
    restore_with_key(window, KEY);
}

pub fn restore_with_key(window: &WebviewWindow, key: &'static str) {
    let Some(db) = window.try_state::<Db>().map(|d| d.inner().clone()) else {
        return;
    };
    let saved: Option<Geometry> = db.kv_get(key).and_then(|v| serde_json::from_str(&v).ok());
    let monitors: Vec<_> = window
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
    match saved.filter(|g| visible_on(g, &monitors)) {
        Some(g) => {
            let _ = window.set_size(PhysicalSize::new(g.width, g.height));
            let _ = window.set_position(PhysicalPosition::new(g.x, g.y));
            if g.maximized {
                let _ = window.maximize();
            }
        }
        None => {
            let _ = window.center();
        }
    }
    let _ = window.show();
    let _ = window.set_focus();

    // Debounced: a drag emits a Moved event per frame.
    let generation = Arc::new(AtomicU64::new(0));
    let w = window.clone();
    window.on_window_event(move |event| {
        if !matches!(event, WindowEvent::Moved(_) | WindowEvent::Resized(_)) {
            return;
        }
        let my = generation.fetch_add(1, Ordering::SeqCst) + 1;
        let (generation, w, db) = (generation.clone(), w.clone(), db.clone());
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(SAVE_DELAY).await;
            if generation.load(Ordering::SeqCst) != my || w.is_minimized().unwrap_or(true) {
                return;
            }
            let maximized = w.is_maximized().unwrap_or(false);
            // Keep the last normal bounds while maximized, so un-maximizing restores them.
            let previous: Option<Geometry> =
                db.kv_get(key).and_then(|v| serde_json::from_str(&v).ok());
            let g = match (maximized, previous) {
                (true, Some(p)) => Geometry {
                    maximized: true,
                    ..p
                },
                _ => {
                    let (Ok(pos), Ok(size)) = (w.outer_position(), w.inner_size()) else {
                        return;
                    };
                    Geometry {
                        x: pos.x,
                        y: pos.y,
                        width: size.width,
                        height: size.height,
                        maximized,
                    }
                }
            };
            if let Ok(json) = serde_json::to_string(&g) {
                let _ = db.kv_set(key, &json);
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::{Geometry, visible_on};

    const G: Geometry = Geometry {
        x: 100,
        y: 100,
        width: 1120,
        height: 720,
        maximized: false,
    };

    #[test]
    fn accepts_geometry_on_a_monitor() {
        assert!(visible_on(&G, &[(0, 0, 1920, 1080)]));
    }

    #[test]
    fn rejects_geometry_on_a_disconnected_monitor() {
        let off = Geometry { x: 2500, ..G };
        assert!(!visible_on(&off, &[(0, 0, 1920, 1080)]));
        assert!(visible_on(
            &off,
            &[(0, 0, 1920, 1080), (1920, 0, 2560, 1440)]
        ));
    }

    #[test]
    fn rejects_tiny_windows() {
        assert!(!visible_on(
            &Geometry { width: 50, ..G },
            &[(0, 0, 1920, 1080)]
        ));
    }
}
