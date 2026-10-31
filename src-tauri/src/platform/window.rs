//! Window styling, z-order, cursor and monitor helpers.

use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetCursorPos, GetWindowLongPtrW, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SetWindowLongPtrW, SetWindowPos, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

/// `HWND` as a plain integer so it can cross threads and live inside `Send` futures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawHwnd(pub isize);

impl RawHwnd {
    pub fn from_hwnd(hwnd: HWND) -> Self {
        Self(hwnd.0 as isize)
    }
    pub(crate) fn hwnd(self) -> HWND {
        HWND(self.0 as *mut core::ffi::c_void)
    }
}

/// Physical-pixel rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhysRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl PhysRect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
    /// True when `self` covers `other` entirely.
    pub fn covers(&self, other: &PhysRect) -> bool {
        self.left <= other.left
            && self.top <= other.top
            && self.right >= other.right
            && self.bottom >= other.bottom
    }
}

impl From<RECT> for PhysRect {
    fn from(r: RECT) -> Self {
        Self {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MonitorRects {
    pub monitor: PhysRect,
    pub work_area: PhysRect,
}

/// Turns the island into a tool window that never takes focus and stays out of
/// the taskbar and Alt+Tab (docs/06-decision-log.md ADR-003).
pub fn make_tool_window(window: RawHwnd) {
    let hwnd = window.hwnd();
    // SAFETY: `hwnd` belongs to a live window of this process; we only OR extra
    // extended-style bits into its existing style.
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let extra = (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | extra);
    }
    reassert_topmost(window);
}

/// Puts the window back on top of the topmost band without activating it.
pub fn reassert_topmost(window: RawHwnd) {
    // SAFETY: plain z-order change on a window we own; flags keep size, position and focus.
    unsafe {
        let _ = SetWindowPos(
            window.hwnd(),
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

/// Cursor position in physical screen pixels.
pub fn cursor_pos() -> Option<(i32, i32)> {
    let mut p = POINT::default();
    // SAFETY: `p` is a valid out-pointer for the duration of the call.
    unsafe { GetCursorPos(&mut p) }.ok().map(|()| (p.x, p.y))
}

/// Whether the (logical) left mouse button is currently held, system-wide.
pub fn left_button_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
    // SAFETY: reads global key state; no pointers involved.
    let state = unsafe { GetAsyncKeyState(i32::from(VK_LBUTTON.0)) };
    state < 0
}

/// Bounds and work area of the monitor containing (or nearest to) a screen point.
pub fn monitor_at(x: i32, y: i32) -> Option<MonitorRects> {
    use windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST;
    // SAFETY: as in `primary_monitor`.
    unsafe {
        let hmon = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetMonitorInfoW(hmon, &mut info)
            .as_bool()
            .then(|| MonitorRects {
                monitor: info.rcMonitor.into(),
                work_area: info.rcWork.into(),
            })
    }
}

/// Bounds and work area of the primary monitor (docs/06-decision-log.md ADR-008).
pub fn primary_monitor() -> Option<MonitorRects> {
    // SAFETY: MonitorFromPoint with DEFAULTTOPRIMARY always returns a valid handle;
    // `info` is a properly sized out-struct.
    unsafe {
        let hmon = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut info).as_bool() {
            Some(MonitorRects {
                monitor: info.rcMonitor.into(),
                work_area: info.rcWork.into(),
            })
        } else {
            None
        }
    }
}

/// Lets the island take keyboard focus for text input, remembering who had it
/// (docs/02-system-design.md §6). Returns the previous foreground window.
pub fn take_focus(window: RawHwnd) -> RawHwnd {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};
    let hwnd = window.hwnd();
    // SAFETY: style change and activation of a window we own; GetForegroundWindow has no inputs.
    unsafe {
        let previous = GetForegroundWindow();
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex & !(WS_EX_NOACTIVATE.0 as isize));
        let _ = SetForegroundWindow(hwnd);
        RawHwnd::from_hwnd(previous)
    }
}

/// Restores `WS_EX_NOACTIVATE` and gives focus back to `previous` if it still exists.
pub fn release_focus(window: RawHwnd, previous: Option<RawHwnd>) {
    use windows::Win32::UI::WindowsAndMessaging::{IsWindow, SetForegroundWindow};
    let hwnd = window.hwnd();
    // SAFETY: as in `take_focus`; `previous` is validated with IsWindow before use.
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | WS_EX_NOACTIVATE.0 as isize);
        if let Some(prev) = previous {
            if IsWindow(Some(prev.hwnd())).as_bool() {
                let _ = SetForegroundWindow(prev.hwnd());
            }
        }
    }
}
