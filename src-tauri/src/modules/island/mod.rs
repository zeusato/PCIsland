//! Island window: creation, placement, click-through, visibility
//! (docs/modules/island-window.md).
#![forbid(unsafe_code)]

pub mod commands;
pub mod dto;
pub mod policy;

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use tokio::sync::{mpsc, watch};

use crate::core::activity::ActivityHub;
use crate::core::events::{AppEvent, EventBus};
use crate::core::priority;
use crate::core::settings::{IslandSettings, Settings, SettingsHandle};
use crate::ipc::events;
use crate::platform::foreground::{self, ForegroundState};
use crate::platform::window::{self as win, PhysRect, RawHwnd};
use dto::{HitRect, PointerEvent, VisibilityEvent, VisibilityMode};
use policy::{AlertOverFullscreen, PolicyInput, PolicySettings};

pub const LABEL: &str = "island";
/// WebView2 flags shared by the island and main windows (same environment, so they must
/// match). One renderer process for both keeps RAM within budget (docs/02 §13).
pub const APP_BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --renderer-process-limit=1 --disable-background-networking";
/// Resting canvas; interactive panels can grow to 600 logical px plus glow space.
const WINDOW_W: f64 = 480.0;
const WINDOW_H: f64 = 264.0;
/// Cursor poll periods (docs/06-decision-log.md ADR-003).
const POLL_NEAR: Duration = Duration::from_millis(16);
const POLL_FAR: Duration = Duration::from_millis(100);
const NEAR_MARGIN: i32 = 48;
/// Re-check maximized state and z-order even without foreground events.
const RECHECK: Duration = Duration::from_secs(1);

fn canvas_height(rects: &[HitRect]) -> f64 {
    rects
        .iter()
        .take(4)
        .map(|r| r.y + r.height)
        .filter(|height| height.is_finite())
        .fold(WINDOW_H - 16.0, f64::max)
        .clamp(WINDOW_H - 16.0, 600.0)
        + 16.0
}

#[cfg(test)]
mod canvas_tests {
    use super::*;

    #[test]
    fn grows_for_queue_and_returns_to_resting_canvas() {
        let rect = |height| HitRect {
            x: 40.0,
            y: 0.0,
            width: 400.0,
            height,
        };
        assert_eq!(canvas_height(&[rect(450.0)]), 466.0);
        assert_eq!(canvas_height(&[rect(32.0)]), 264.0);
        assert_eq!(canvas_height(&[rect(900.0)]), 616.0);
        assert_eq!(canvas_height(&[rect(f64::NAN)]), 264.0);
    }
}

enum Command {
    ToggleHidden(tokio::sync::oneshot::Sender<VisibilityMode>),
}

struct Shared {
    window: WebviewWindow,
    hwnd: RawHwnd,
    /// Window origin in physical screen px, set whenever we move the window.
    origin: Mutex<(i32, i32)>,
    /// Monitor the island currently sits on.
    monitor: Mutex<PhysRect>,
    /// Hit regions in physical px relative to the window origin.
    hits: Mutex<Vec<PhysRect>>,
    visibility: Mutex<VisibilityEvent>,
    commands: mpsc::UnboundedSender<Command>,
    focus_owner: Mutex<Option<RawHwnd>>,
}

/// Cloneable handle managed as Tauri state.
#[derive(Clone)]
pub struct IslandHandle(Arc<Shared>);

impl IslandHandle {
    pub fn set_hit_regions(&self, rects: &[HitRect]) -> Result<(), tauri::Error> {
        let scale = self.0.window.scale_factor()?;
        let height = canvas_height(rects);
        let current = self.0.window.inner_size()?;
        if (f64::from(current.height) / scale - height).abs() > 1.0 {
            self.0.window.set_size(LogicalSize::new(WINDOW_W, height))?;
        }
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

    pub fn hwnd(&self) -> RawHwnd {
        self.0.hwnd
    }

    pub fn visibility(&self) -> VisibilityEvent {
        lock(&self.0.visibility).clone()
    }

    pub fn scale_factor(&self) -> f64 {
        self.0.window.scale_factor().unwrap_or(1.0)
    }

    /// Island temporarily accepts keyboard focus (note / reminder input).
    pub fn request_focus(&self) {
        let previous = win::take_focus(self.0.hwnd);
        lock(&self.0.focus_owner).get_or_insert(previous);
        let _ = self.0.window.set_focus();
    }

    pub fn release_focus(&self) {
        let previous = lock(&self.0.focus_owner).take();
        win::release_focus(self.0.hwnd, previous);
    }

    pub async fn toggle_hidden(&self) -> Option<VisibilityMode> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.0.commands.send(Command::ToggleHidden(tx)).ok()?;
        rx.await.ok()
    }
}

/// Creates the island window and starts its services.
pub fn setup(
    app: &AppHandle,
    hub: ActivityHub,
    settings: &SettingsHandle,
    bus: EventBus,
) -> tauri::Result<IslandHandle> {
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("island.html".into()))
        .title("PCisland")
        .additional_browser_args(APP_BROWSER_ARGS)
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
        monitor: Mutex::new(PhysRect::default()),
        hits: Mutex::new(Vec::new()),
        visibility: Mutex::new(VisibilityEvent {
            mode: VisibilityMode::Normal,
            reason: "startup".into(),
            has_live_activity: false,
            rev: 0,
        }),
        commands: cmd_tx,
        focus_owner: Mutex::new(None),
    });
    if let Some(mon) = win::primary_monitor() {
        place_on(&shared, mon)?;
    }
    window.show()?;
    win::reassert_topmost(hwnd);

    let (fg_tx, fg_rx) = mpsc::unbounded_channel();
    foreground::spawn_foreground_hook(fg_tx);
    tauri::async_runtime::spawn(visibility_loop(
        app.clone(),
        shared.clone(),
        fg_rx,
        cmd_rx,
        hub,
        settings.subscribe(),
        bus,
    ));
    tauri::async_runtime::spawn(cursor_poller(
        app.clone(),
        shared.clone(),
        settings.subscribe(),
    ));

    Ok(IslandHandle(shared))
}

/// Centers the window horizontally on `mon`, flush with the top of its work area
/// (docs/modules/island-window.md §5).
fn place_on(shared: &Shared, mon: win::MonitorRects) -> tauri::Result<()> {
    let scale = shared.window.scale_factor()?;
    let width = (WINDOW_W * scale).round() as i32;
    let x = mon.monitor.left + (mon.monitor.width() - width) / 2;
    let y = mon.work_area.top;
    shared.window.set_position(PhysicalPosition::new(x, y))?;
    *lock(&shared.origin) = (x, y);
    *lock(&shared.monitor) = mon.monitor;
    Ok(())
}

/// "cursor" mode: move to the monitor under the cursor after it rests there for 1 s.
/// "primary" mode: return to the primary monitor if we are elsewhere.
fn follow_monitor(
    shared: &Shared,
    mode: &str,
    cx: i32,
    cy: i32,
    since: &mut Option<(PhysRect, std::time::Instant)>,
) {
    let current = *lock(&shared.monitor);
    let target = if mode == "cursor" {
        win::monitor_at(cx, cy)
    } else {
        win::primary_monitor()
    };
    let Some(target) = target else { return };
    if target.monitor == current {
        *since = None;
        return;
    }
    let wait = if mode == "cursor" {
        std::time::Duration::from_secs(1)
    } else {
        std::time::Duration::ZERO
    };
    match since {
        Some((m, t)) if *m == target.monitor && t.elapsed() >= wait => {
            let _ = place_on(shared, target);
            *since = None;
        }
        Some((m, _)) if *m == target.monitor => {}
        _ => *since = Some((target.monitor, std::time::Instant::now())),
    }
}

async fn visibility_loop(
    app: AppHandle,
    shared: Arc<Shared>,
    mut foreground_changed: mpsc::UnboundedReceiver<()>,
    mut commands: mpsc::UnboundedReceiver<Command>,
    hub: ActivityHub,
    mut settings: watch::Receiver<Settings>,
    bus: EventBus,
) {
    let mut top = hub.top();
    let mut was_locked = false;
    let mut user_hidden = false;
    let mut last_fg = ForegroundState::default();
    let mut tick = tokio::time::interval(RECHECK);
    loop {
        let mut reason = "recheck";
        let mut visibility_reply = None;
        tokio::select! {
            Some(()) = foreground_changed.recv() => {
                reason = "foreground";
                win::reassert_topmost(shared.hwnd);
            }
            Some(cmd) = commands.recv() => match cmd {
                Command::ToggleHidden(reply) => {
                    user_hidden = !user_hidden;
                    reason = "user";
                    visibility_reply = Some(reply);
                }
            },
            Ok(()) = top.changed() => reason = "activity",
            Ok(()) = settings.changed() => reason = "settings",
            _ = tick.tick() => {}
        }
        let fg = foreground::foreground_state(shared.hwnd);
        let locked = crate::platform::foreground::session_locked();
        if locked != was_locked {
            was_locked = locked;
            bus.send(AppEvent::SessionLocked(locked));
        }
        let info = *top.borrow();
        let previous_mode = lock(&shared.visibility).mode;
        let policy_settings = policy_settings(&settings.borrow().island);
        let mode = policy::decide(
            PolicyInput {
                user_hidden,
                session_locked: locked,
                fullscreen: fg.fullscreen,
                maximized: fg.maximized,
                top_priority: info.priority,
                has_alert: info.has_alert,
            },
            policy_settings,
        );
        let has_live = info.priority >= priority::LIVE;
        if let Some(reply) = visibility_reply {
            let _ = reply.send(mode);
        }
        hub.set_island_hidden(mode == VisibilityMode::Hidden);
        if (mode == VisibilityMode::Hidden) != (previous_mode == VisibilityMode::Hidden) {
            bus.send(AppEvent::IslandHidden(mode == VisibilityMode::Hidden));
        }
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

async fn cursor_poller(app: AppHandle, shared: Arc<Shared>, settings: watch::Receiver<Settings>) {
    let mut inside = false;
    let mut other_monitor_since: Option<(PhysRect, std::time::Instant)> = None;
    let mut last_gaze = (0i8, 0i8);
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
        let (follow, monitor_mode) = {
            let s = settings.borrow();
            (
                s.indicators.eyes_follow_cursor && s.island.idle_display == "eyes",
                s.island.monitor.clone(),
            )
        };
        follow_monitor(&shared, &monitor_mode, cx, cy, &mut other_monitor_since);
        let (ox, oy) = *lock(&shared.origin);
        let (rx, ry) = (cx - ox, cy - oy);
        let scale = shared.window.scale_factor().unwrap_or(1.0);
        if follow {
            // Eyes look toward the cursor within 240 px of the island (docs/04 §11).
            let (ex, ey) = ((WINDOW_W * scale / 2.0) as i32, (15.0 * scale) as i32);
            let reach = (240.0 * scale) as i32;
            let (dx, dy) = (rx - ex, ry - ey);
            let gaze = if dx.abs() <= reach && dy.abs() <= reach {
                (
                    (dx * 100 / reach.max(1)) as i8,
                    (dy * 100 / reach.max(1)) as i8,
                )
            } else {
                (0, 0)
            };
            if gaze != last_gaze {
                last_gaze = gaze;
                let _ = app.emit_to(LABEL, "island://gaze", (gaze.0, gaze.1));
            }
        }
        let bounds = PhysRect {
            left: -NEAR_MARGIN,
            top: -NEAR_MARGIN,
            right: (WINDOW_W * scale) as i32 + NEAR_MARGIN,
            bottom: lock(&shared.hits)
                .iter()
                .map(|r| r.bottom)
                .max()
                .unwrap_or((WINDOW_H * scale) as i32)
                + NEAR_MARGIN,
        };
        let near = if follow { NEAR_MARGIN * 5 } else { NEAR_MARGIN };
        let wide = PhysRect {
            left: bounds.left - near + NEAR_MARGIN,
            top: bounds.top,
            right: bounds.right + near - NEAR_MARGIN,
            bottom: bounds.bottom + near - NEAR_MARGIN,
        };
        period = if wide.contains(rx, ry) {
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
    let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::App(url.into()))
        .title("PCisland")
        .additional_browser_args(APP_BROWSER_ARGS)
        .inner_size(1120.0, 720.0)
        .min_inner_size(960.0, 640.0)
        .decorations(false)
        .background_color(tauri::window::Color(11, 11, 13, 255))
        .visible(false)
        .build()?;
    crate::app::main_window::restore_and_track(&window);
    Ok(())
}

fn policy_settings(s: &IslandSettings) -> PolicySettings {
    PolicySettings {
        hide_in_fullscreen: s.hide_in_fullscreen,
        hide_while_active: s.hide_while_active,
        minimal_when_maximized: s.minimal_when_maximized,
        hide_when_idle: s.idle_display == "hidden",
        alert_over_fullscreen: match s.alert_over_fullscreen.as_str() {
            "never" => AlertOverFullscreen::Never,
            "all" => AlertOverFullscreen::All,
            _ => AlertOverFullscreen::Critical,
        },
    }
}
