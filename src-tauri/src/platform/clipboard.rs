//! Reads plain text from the clipboard so the video panel can prefill a copied link
//! (docs/modules/video-player.md §3). Read-only; never writes the clipboard.

use windows::Win32::Foundation::{HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
};
use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::Ole::CF_UNICODETEXT;

/// At most `max_chars` UTF-16 units of text, or `None` when there is no text.
pub fn read_text(max_chars: usize) -> Option<String> {
    // SAFETY: plain Win32 clipboard calls. The clipboard is closed on every path below and
    // the global memory is only read while locked, within its reported size.
    unsafe {
        IsClipboardFormatAvailable(u32::from(CF_UNICODETEXT.0)).ok()?;
        OpenClipboard(None).ok()?;
        let text = (|| {
            let handle: HANDLE = GetClipboardData(u32::from(CF_UNICODETEXT.0)).ok()?;
            let global = HGLOBAL(handle.0);
            let ptr = GlobalLock(global) as *const u16;
            if ptr.is_null() {
                return None;
            }
            let units = (GlobalSize(global) / 2).min(max_chars);
            let slice = std::slice::from_raw_parts(ptr, units);
            let end = slice.iter().position(|&c| c == 0).unwrap_or(units);
            let text = String::from_utf16_lossy(&slice[..end]);
            let _ = GlobalUnlock(global);
            Some(text)
        })();
        let _ = CloseClipboard();
        text
    }
}
