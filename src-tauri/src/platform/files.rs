//! Atomic same-volume publication without overwriting an existing destination.
use std::{os::windows::ffi::OsStrExt, path::Path};
use windows::{
    Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW},
    core::PCWSTR,
};
pub fn publish(temp: &Path, destination: &Path) -> std::io::Result<()> {
    let from: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // SAFETY: NUL-terminated paths remain alive across the call. Deliberately no
    // REPLACE_EXISTING flag: an existing destination, including a link, is kept.
    unsafe {
        MoveFileExW(
            PCWSTR(from.as_ptr()),
            PCWSTR(to.as_ptr()),
            MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(std::io::Error::other)
}
