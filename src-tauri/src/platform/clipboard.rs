//! Reads plain text from the clipboard so the video panel can prefill a copied link
//! (docs/modules/video-player.md §3). File transfer uses native CF_HDROP.

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

/// Copy actual file objects for Explorer/chat paste, not a text list of paths.
pub fn write_files(
    paths: &[std::path::PathBuf],
    owner: super::window::RawHwnd,
) -> windows::core::Result<()> {
    use windows::Win32::Foundation::GlobalFree;
    use windows::Win32::System::DataExchange::{EmptyClipboard, SetClipboardData};
    use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc};
    use windows::Win32::System::Ole::CF_HDROP;
    use windows::Win32::UI::Shell::DROPFILES;
    let mut names = Vec::<u16>::new();
    for path in paths {
        names.extend(path.as_os_str().to_string_lossy().encode_utf16());
        names.push(0);
    }
    names.push(0);
    // SAFETY: allocation includes header + double-NUL terminated UTF-16 paths.
    // Ownership passes to Windows only on successful SetClipboardData.
    unsafe {
        let size = std::mem::size_of::<DROPFILES>();
        let memory = GlobalAlloc(GMEM_MOVEABLE, size + names.len() * 2)?;
        let ptr = GlobalLock(memory).cast::<u8>();
        if ptr.is_null() {
            let _ = GlobalFree(Some(memory));
            return Err(windows::core::Error::from_thread());
        }
        ptr.cast::<DROPFILES>().write(DROPFILES {
            pFiles: size as u32,
            fWide: true.into(),
            ..Default::default()
        });
        std::ptr::copy_nonoverlapping(names.as_ptr().cast::<u8>(), ptr.add(size), names.len() * 2);
        let _ = GlobalUnlock(memory);
        let result = (|| {
            OpenClipboard(Some(owner.hwnd()))?;
            let result = (|| {
                EmptyClipboard()?;
                SetClipboardData(u32::from(CF_HDROP.0), Some(HANDLE(memory.0)))?;
                Ok(())
            })();
            let _ = CloseClipboard();
            result
        })();
        if result.is_err() {
            let _ = GlobalFree(Some(memory));
        }
        result
    }
}
