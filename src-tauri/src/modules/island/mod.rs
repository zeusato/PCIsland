//! Island window: creation, placement, click-through, visibility
//! (docs/modules/island-window.md).
#![forbid(unsafe_code)]

pub mod commands;
pub mod dto;
pub mod policy;

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use tokio::sync::{mpsc, watch};

use crate::core::priority;
use crate::ipc::events;
use crate::platform::foreground::{self, ForegroundState};
use crate::platform::window::{self as win, PhysRect, RawHwnd};
use dto::{HitRect, PointerEvent, VisibilityEvent, VisibilityMode};
use policy::{PolicyInput, PolicySettings};

pub const LABEL: &str = "island";
/// Fixed logical size of the island window (docs/04-design-system.md §5).
const WINDOW_W: f64 = 480.0;
const WINDOW_H: f64 = 264.0;
/// Cursor poll periods (docs/06-decision-log.md ADR-003).
const POLL_NEAR: Duration = Duration::from_millis(16);
const POLL_FAR: Duration = Duration::from_millis(100);
const NEAR_MARGIN: i32 = 48;
/// Re-check maximized state and z-order even without foreground events.
const RECHECK: Duration = Duration::from_secs(1);

enum Command {
    ToggleHidden(tokio::sync::oneshot::Sender<VisibilityMode>),
}

struct Shared {
    window: WebviewWindow,
    hwnd: RawHwnd,
    /// Window origin in physical screen px, set whenever we move the window.
    origin: Mutex<(i32, i32)>,
    /// Hit regions in physical px relative to the window origin.
    hits: Mutex<Vec<PhysRect>>,
    visibility: Mutex<VisibilityEvent>,
    commands: mpsc::UnboundedSender<Command>,
}

/// Cloneable handle managed as Tauri state.
#[derive(Clone)]
pub struct IslandHandle(Arc<Shared>);

impl IslandHandle {
    pub fn set_hit_regions(&self, rects: &[HitRect]) -> Result<(), tauri::Error> {
        let scale = self.0.window.scale_factor()?;
        let phys = rects
            .iter()
            .take(4)
            .map(|r| PhysRect {
                left: (r.x * scale).floor() as i32,
                top: (r.y * scale).floor() as i32,
                right: ((r.x + r.width) * scale).ceil() as i32,
                bottom: ((r.y + r.height) * scale).ceil() as i32,
            })
            .collect();
        *lock(&self.0.hits) = phys;
        Ok(())
    }

    pub fn visibility(&self) -> VisibilityEvent {
        lock(&self.0.visibility).clone()
    }

    pub fn scale_factor(&self) -> f64 {
        self.0.window.scale_factor().unwrap_or(1.0)
    }

    pub async fn toggle_hidden(&self) -> Option<VisibilityMode> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.0.commands.send(Command::ToggleHidden(tx)).ok()?;
        rx.await.ok()
    }
}

/// Creates the island window and starts its services. `top_priority` is the P0
/// stand-in for ActivityHub (see `core::priority`).
pub fn setup(app: &AppHandle, top_priority: watch::Receiver<u8>) -> tauri::Result<IslandHandle> {
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("island.html".into()))
        .title("PCisland")
        .inner_size(WINDOW_W, WINDOW_H)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .focused(false)
        .visible(false)
        .build()?;
    let hwnd = RawHwnd::from_hwnd(window.hwnd()?);
    win::make_tool_window(hwnd);
    window.set_ignore_cursor_events(true)?;

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let shared = Arc::new(Shared {
        window: window.clone(),
        hwnd,
        origin: Mutex::new((0, 0)),
        hits: Mutex::new(Vec::new()),
        visibility: Mutex::new(VisibilityEvent {
            mode: VisibilityMode::Normal,
            reason: "startup".into(),
            has_live_activity: false,
            rev: 0,
        }),
        commands: cmd_tx,
    });
    place_on_primary(&shared)?;
    window.show()?;
    win::reassert_topmost(hwnd);

    let (fg_tx, fg_rx) = mpsc::unbounded_channel();
    foreground::spawn_foreground_hook(fg_tx);
    tauri::async_runtime::spawn(visibility_loop(
        app.clone(),
        shared.clone(),
        fg_rx,
        cmd_rx,
        top_priority,
    ));
    tauri::async_runtime::spawn(cursor_poller(app.clone(), shared.clone()));

    Ok(IslandHandle(shared))
}

/// Centers the window horizontally on the primary monitor, flush with the top of
/// its work area (docs/modules/island-window.md §5).
fn place_on_primary(shared: &Shared) -> tauri::Result<()> {
    let Some(mon) = win::primary_monitor() else {
        return Ok(());
    };
    let scale = shared.window.scale_factor()?;
    let width = (WINDOW_W * scale).round() as i32;
    let x = mon.monitor.left + (mon.monitor.width() - width) / 2;
    let y = mon.work_area.top;
    shared.window.set_position(PhysicalPosition::new(x, y))?;
    *lock(&shared.origin) = (x, y);
    Ok(())
}

async fn visibility_loop(
    app: AppHandle,
    shared: Arc<Shared>,
    mut foreground_changed: mpsc::UnboundedReceiver<()>,
    mut commands: mpsc::UnboundedReceiver<Command>,
    mut top_priority: watch::Receiver<u8>,
) {
    let settings = PolicySettings::default();
    let mut user_hidden = false;
    let mut last_fg = ForegroundState::default();
    let mut tick = tokio::time::interval(RECHECK);
    loop {
        let mut reason = "recheck";
        tokio::select! {
            Some(()) = foreground_changed.recv() => {
                reason = "foreground";
                win::reassert_topmost(shared.hwnd);
            }
            Some(cmd) = commands.recv() => match cmd {
                Command::ToggleHidden(reply) => {
                    user_hidden = !user_hidden;
                    reason = "user";
                    let _ = reply.send(if user_hidden { VisibilityMode::Hidden } else { VisibilityMode::Normal });
                }
            },
            Ok(()) = top_priority.changed() => reason = "activity",
            _ = tick.tick() => {}
        }
        let fg = foreground::foreground_state(shared.hwnd);
        let top = *top_priority.borrow();
        let mode = policy::decide(
            PolicyInput {
                user_hidden,
                fullscreen: fg.fullscreen,
                maximized: fg.maximized,
                top_priority: top,
                ..Default::default()
            },
            settings,
        );
        let has_live = top >= priority::LIVE;
        let previous = lock(&shared.visibility).clone();
        if mode == previous.mode && has_live == previous.has_live_activity && fg == last_fg {
            continue;
        }
        last_fg = fg;
        let event = VisibilityEvent {
            mode,
            reason: reason.into(),
            has_live_activity: has_live,
            rev: previous.rev + 1,
        };
        *lock(&shared.visibility) = event.clone();
        apply_visibility(&app, &shared, previous.mode, &event);
    }
}

fn apply_visibility(
    app: &AppHandle,
    shared: &Shared,
    previous: VisibilityMode,
    event: &VisibilityEvent,
) {
    tracing::info!(module = "island", mode = ?event.mode, reason = %event.reason, "visibility changed");
    let hidden = event.mode == VisibilityMode::Hidden;
    if !hidden && previous == VisibilityMode::Hidden {
        let _ = shared.window.show();
        win::reassert_topmost(shared.hwnd);
    }
    let _ = app.emit_to(LABEL, events::ISLAND_VISIBILITY, event);
    if hidden {
        // Hide the native window so it never interferes with fullscreen apps.
        let _ = shared.window.hide();
    }
}

async fn cursor_poller(app: AppHandle, shared: Arc<Shared>) {
    let mut inside = false;
    let mut was_pressed = false;
    let mut period = POLL_FAR;
    loop {
        tokio::time::sleep(period).await;
        if lock(&shared.visibility).mode == VisibilityMode::Hidden {
            period = POLL_FAR;
            continue;
        }
        let Some((cx, cy)) = win::cursor_pos() else {
            continue;
        };
        let (ox, oy) = *lock(&shared.origin);
        let (rx, ry) = (cx - ox, cy - oy);
        let scale = shared.window.scale_factor().unwrap_or(1.0);
        let bounds = PhysRect {
            left: -NEAR_MARGIN,
            top: -NEAR_MARGIN,
            right: (WINDOW_W * scale) as i32 + NEAR_MARGIN,
            bottom: (WINDOW_H * scale) as i32 + NEAR_MARGIN,
        };
        period = if bounds.contains(rx, ry) {
            POLL_NEAR
        } else {
            POLL_FAR
        };
        let now_inside = lock(&shared.hits).iter().any(|r| r.contains(rx, ry));
        let pressed = win::left_button_down();
        if !now_inside && pressed && !was_pressed {
            let _ = app.emit_to(
                LABEL,
                events::ISLAND_POINTER,
                PointerEvent {
                    inside: false,
                    click_outside: true,
                },
            );
        }
        was_pressed = pressed;
        if now_inside != inside {
            inside = now_inside;
            let _ = shared.window.set_ignore_cursor_events(!inside);
            let _ = app.emit_to(
                LABEL,
                events::ISLAND_POINTER,
                PointerEvent {
                    inside,
                    click_outside: false,
                },
            );
        }
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Opens (or focuses) the full window — docs/modules/full-window.md §5.
pub fn open_main(app: &AppHandle, route: Option<&str>) -> tauri::Result<()> {
    if let Some(main) = app.get_webview_window("main") {
        main.unminimize()?;
        main.show()?;
        main.set_focus()?;
        return Ok(());
    }
    let url = format!(
        "main.html{}",
        route.map(|r| format!("#{r}")).unwrap_or_default()
    );
    WebviewWindowBuilder::new(app, "main", WebviewUrl::App(url.into()))
        .title("PCisland")
        .inner_size(1120.0, 720.0)
        .min_inner_size(960.0, 640.0)
        .decorations(false)
        .center()
        .build()?;
    Ok(())
}
