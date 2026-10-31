//! Foreground window watching and fullscreen / maximized detection
//! (docs/modules/island-window.md §5).

use std::sync::OnceLock;

use tokio::sync::mpsc::UnboundedSender;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONULL, MONITOR_DEFAULTTOPRIMARY, MONITORINFO,
    MonitorFromWindow,
};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::Shell::{
    QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, EVENT_SYSTEM_FOREGROUND, GetClassNameW, GetForegroundWindow, GetMessageW,
    GetWindowRect, GetWindowThreadProcessId, IsZoomed, MSG, WINEVENT_OUTOFCONTEXT,
};

use super::window::{PhysRect, RawHwnd};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ForegroundState {
    /// A fullscreen app (video, game, slideshow) is in front on the island's monitor.
    pub fullscreen: bool,
    /// A maximized window is in front on the island's monitor.
    pub maximized: bool,
}

/// Window classes that never count as fullscreen or maximized.
const SHELL_CLASSES: &[&str] = &[
    "Progman",
    "WorkerW",
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
];

/// Inspects the current foreground window relative to the monitor that holds `island`.
pub fn foreground_state(island: RawHwnd) -> ForegroundState {
    let island = island.hwnd();
    // SAFETY: every call below only reads window/monitor information; handles come
    // straight from the OS and out-structs are correctly sized locals.
    unsafe {
        let fg = GetForegroundWindow();
        if fg.is_invalid() || fg == island || is_own_process(fg) || is_shell_window(fg) {
            return ForegroundState::default();
        }
        let island_mon = MonitorFromWindow(island, MONITOR_DEFAULTTOPRIMARY);
        let fg_mon = MonitorFromWindow(fg, MONITOR_DEFAULTTONULL);
        if fg_mon != island_mon {
            return ForegroundState::default();
        }
        let maximized = IsZoomed(fg).as_bool();
        let covers_monitor = match (window_rect(fg), monitor_rect(fg_mon)) {
            (Some(win), Some(mon)) => win.covers(&mon),
            _ => false,
        };
        let quns_busy = matches!(
            SHQueryUserNotificationState(),
            Ok(s) if s == QUNS_BUSY || s == QUNS_RUNNING_D3D_FULL_SCREEN || s == QUNS_PRESENTATION_MODE
        );
        ForegroundState {
            fullscreen: covers_monitor || quns_busy,
            maximized,
        }
    }
}

unsafe fn is_own_process(hwnd: HWND) -> bool {
    let mut pid = 0u32;
    // SAFETY: `pid` is a valid out-pointer.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    pid == std::process::id()
}

unsafe fn is_shell_window(hwnd: HWND) -> bool {
    let mut buf = [0u16; 64];
    // SAFETY: `buf` is a valid writable buffer of the given length.
    let len = unsafe { GetClassNameW(hwnd, &mut buf) };
    let len = usize::try_from(len).unwrap_or(0);
    let class = String::from_utf16_lossy(&buf[..len]);
    SHELL_CLASSES.contains(&class.as_str())
}

unsafe fn window_rect(hwnd: HWND) -> Option<PhysRect> {
    let mut r = RECT::default();
    // SAFETY: `r` is a valid out-pointer.
    unsafe { GetWindowRect(hwnd, &mut r) }
        .ok()
        .map(|()| r.into())
}

unsafe fn monitor_rect(hmon: HMONITOR) -> Option<PhysRect> {
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: `info` is a correctly sized out-struct.
    unsafe { GetMonitorInfoW(hmon, &mut info) }
        .as_bool()
        .then(|| info.rcMonitor.into())
}

static FOREGROUND_TX: OnceLock<UnboundedSender<()>> = OnceLock::new();

/// Starts the `win-events` thread: a message loop hosting `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`.
/// Each foreground change sends `()` on `tx`. Call once.
pub fn spawn_foreground_hook(tx: UnboundedSender<()>) {
    if FOREGROUND_TX.set(tx).is_err() {
        tracing::warn!("foreground hook already running");
        return;
    }
    std::thread::Builder::new()
        .name("win-events".into())
        .spawn(|| {
            // SAFETY: the hook and the message loop live on this thread for its whole
            // lifetime; the callback only touches a thread-safe static sender.
            unsafe {
                let hook = SetWinEventHook(
                    EVENT_SYSTEM_FOREGROUND,
                    EVENT_SYSTEM_FOREGROUND,
                    None,
                    Some(on_foreground),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT,
                );
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    DispatchMessageW(&msg);
                }
                let _ = UnhookWinEvent(hook);
            }
        })
        .expect("spawn win-events thread");
}

unsafe extern "system" fn on_foreground(
    _hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    if let Some(tx) = FOREGROUND_TX.get() {
        let _ = tx.send(());
    }
}
