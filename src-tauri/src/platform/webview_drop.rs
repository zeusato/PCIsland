//! OS file drops for the island WebView (docs/modules/island-window.md §5).
//!
//! wry registers its OLE drop target on the WebView2 host child window, but on the
//! transparent island the window under the cursor is Chromium's render widget in the
//! browser process, so OLE never reaches that target and Windows shows "no drop".
//! The island therefore keeps Chromium's own drop handling and the page forwards the
//! dropped `File` objects with `postMessageWithAdditionalObjects`; this reads their real
//! paths from `ICoreWebView2File`.
use std::path::PathBuf;

use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2Controller, ICoreWebView2File, ICoreWebView2WebMessageReceivedEventArgs2,
};
use webview2_com::{WebMessageReceivedEventHandler, take_pwstr};
use windows::core::{Interface, PWSTR};

/// Message string the island page posts together with the dropped files.
pub const DROP_MESSAGE: &str = "pcisland:drop-files";
const MAX_FILES: u32 = 500;

/// Calls `on_drop` with the paths of files dropped on this WebView's own page.
pub fn install(
    controller: &ICoreWebView2Controller,
    on_drop: impl Fn(Vec<PathBuf>) + 'static,
) -> windows::core::Result<()> {
    // SAFETY: called on the WebView2 UI thread (Tauri's `with_webview`). Every PWSTR the
    // API returns is CoTaskMem-allocated and freed by `take_pwstr`; COM interfaces are
    // reference-counted by the `windows` wrappers.
    unsafe {
        let webview = controller.CoreWebView2()?;
        let handler = WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
            let Some(args) = args else { return Ok(()) };
            let mut text = PWSTR::null();
            if args.TryGetWebMessageAsString(&mut text).is_err() {
                return Ok(()); // JSON messages (Tauri IPC) are not ours.
            }
            if take_pwstr(text) != DROP_MESSAGE {
                return Ok(());
            }
            let mut source = PWSTR::null();
            args.Source(&mut source)?;
            if !is_app_page(&take_pwstr(source)) {
                return Ok(());
            }
            let objects = args
                .cast::<ICoreWebView2WebMessageReceivedEventArgs2>()?
                .AdditionalObjects()?;
            let mut count = 0;
            objects.Count(&mut count)?;
            let mut paths = Vec::new();
            for index in 0..count.min(MAX_FILES) {
                let Ok(file) = objects.GetValueAtIndex(index)?.cast::<ICoreWebView2File>() else {
                    continue;
                };
                let mut path = PWSTR::null();
                if file.Path(&mut path).is_ok() {
                    paths.push(PathBuf::from(take_pwstr(path)));
                }
            }
            if !paths.is_empty() {
                on_drop(paths);
            }
            Ok(())
        }));
        let mut token = 0;
        webview.add_WebMessageReceived(&handler, &mut token)
    }
}

/// Only the app's own bundled page (or the dev server) may hand files to the app.
fn is_app_page(source: &str) -> bool {
    [
        "http://tauri.localhost/",
        "https://tauri.localhost/",
        "tauri://localhost/",
    ]
    .iter()
    .any(|origin| source.starts_with(origin))
        || (cfg!(debug_assertions) && source.starts_with("http://localhost:"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_local_app_pages_may_deliver_files() {
        assert!(super::is_app_page("http://tauri.localhost/island.html"));
        assert!(!super::is_app_page("https://soundcloud.com/"));
        assert!(!super::is_app_page("http://tauri.localhost.evil.com/"));
    }
}
