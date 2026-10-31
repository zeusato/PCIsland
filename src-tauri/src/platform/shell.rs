//! Shell helpers: file thumbnails and user idle time (docs/modules/file-shelf.md §5).

use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    BI_BITFIELDS, BITMAP, BITMAPINFO, BITMAPV4HEADER, DIB_RGB_COLORS, DeleteObject, GetDC,
    GetDIBits, GetObjectW, HGDIOBJ, ReleaseDC,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK,
};
use windows::core::HSTRING;

/// Shell thumbnail (or icon) of `path` as a 32-bit BMP with alpha, `size`×`size` max.
/// Runs on a fresh STA thread because the shell requires one.
pub fn thumbnail(path: &str, size: i32) -> Option<Vec<u8>> {
    let path = path.to_owned();
    std::thread::spawn(move || thumbnail_sta(&path, size))
        .join()
        .ok()
        .flatten()
}

fn thumbnail_sta(path: &str, size: i32) -> Option<Vec<u8>> {
    // SAFETY: COM/GDI calls on this short-lived thread; the bitmap and DC are released
    // before returning and the pixel buffer is sized from the bitmap's own dimensions.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(&HSTRING::from(path), None).ok()?;
        let hbmp = factory
            .GetImage(SIZE { cx: size, cy: size }, SIIGBF_BIGGERSIZEOK)
            .ok()?;
        let mut bm = BITMAP::default();
        GetObjectW(
            HGDIOBJ(hbmp.0),
            size_of::<BITMAP>() as i32,
            Some((&raw mut bm).cast()),
        );
        let (w, h) = (bm.bmWidth, bm.bmHeight);
        let mut header = BITMAPV4HEADER {
            bV4Size: size_of::<BITMAPV4HEADER>() as u32,
            bV4Width: w,
            bV4Height: -h, // top-down rows
            bV4Planes: 1,
            bV4BitCount: 32,
            bV4V4Compression: BI_BITFIELDS,
            bV4RedMask: 0x00FF_0000,
            bV4GreenMask: 0x0000_FF00,
            bV4BlueMask: 0x0000_00FF,
            bV4AlphaMask: 0xFF00_0000,
            ..Default::default()
        };
        let mut pixels = vec![0u8; (w * h * 4).max(0) as usize];
        let dc = GetDC(None);
        let lines = GetDIBits(
            dc,
            hbmp,
            0,
            h as u32,
            Some(pixels.as_mut_ptr().cast()),
            (&raw mut header).cast::<BITMAPINFO>(),
            DIB_RGB_COLORS,
        );
        ReleaseDC(None, dc);
        let _ = DeleteObject(HGDIOBJ(hbmp.0));
        (lines > 0).then(|| bmp_file(&header, &pixels))
    }
}

/// BITMAPFILEHEADER + BITMAPV4HEADER + pixels.
fn bmp_file(header: &BITMAPV4HEADER, pixels: &[u8]) -> Vec<u8> {
    let header_len = size_of::<BITMAPV4HEADER>();
    let offset = 14 + header_len;
    let mut out = Vec::with_capacity(offset + pixels.len());
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&((offset + pixels.len()) as u32).to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(offset as u32).to_le_bytes());
    // SAFETY: BITMAPV4HEADER is a plain #[repr(C)] struct; we copy exactly its bytes.
    let raw = unsafe {
        std::slice::from_raw_parts((header as *const BITMAPV4HEADER).cast::<u8>(), header_len)
    };
    out.extend_from_slice(raw);
    out.extend_from_slice(pixels);
    out
}

/// Seconds since the last keyboard/mouse input (for the sleepy eyes, F33).
pub fn idle_seconds() -> u32 {
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    let mut info = LASTINPUTINFO {
        cbSize: size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    // SAFETY: `info` is a correctly sized out-struct; GetTickCount has no inputs.
    unsafe {
        if !GetLastInputInfo(&mut info).as_bool() {
            return 0;
        }
        GetTickCount().wrapping_sub(info.dwTime) / 1000
    }
}
