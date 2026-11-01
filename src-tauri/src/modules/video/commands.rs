//! Video commands (docs/modules/video-player.md §6). Commands that may create a window
//! are async (docs/03-coding-principles.md §3).

use tauri::{State, Webview};

use super::VideoHandle;
use super::dto::{PlaybackSource, SniffItem, VideoOpenResult, VideoReport, VideoSnapshot};
use super::link::LinkKind;
use super::tools::ToolsStatus;
use crate::ipc::{IpcError, IpcResult};

fn from_video_window(webview: &Webview) -> IpcResult<()> {
    if webview.label() == super::window::LABEL {
        Ok(())
    } else {
        Err(IpcError::new(
            "video.rejected",
            "only the video window may call this",
        ))
    }
}

#[tauri::command]
pub fn video_classify(url: String) -> Option<LinkKind> {
    super::link::classify(&url)
}

/// A link copied to the clipboard, if it is one the player accepts.
#[tauri::command]
pub fn video_clipboard() -> Option<String> {
    super::clipboard_link()
}

#[tauri::command]
pub async fn video_open(
    video: State<'_, VideoHandle>,
    url: String,
    embed: Option<bool>,
) -> IpcResult<VideoOpenResult> {
    video.open(&url, embed.unwrap_or(false)).await
}

#[tauri::command]
pub async fn video_open_stream(video: State<'_, VideoHandle>, id: String) -> IpcResult<()> {
    video.open_stream(&id).await
}

#[tauri::command]
pub fn video_get_state(video: State<'_, VideoHandle>) -> VideoSnapshot {
    video.snapshot()
}

#[tauri::command]
pub fn video_tools_status(video: State<'_, VideoHandle>) -> ToolsStatus {
    video.tools_status()
}

/// Downloads yt-dlp. The panel calls this only after the user agreed.
#[tauri::command]
pub async fn video_tools_install(video: State<'_, VideoHandle>) -> IpcResult<ToolsStatus> {
    video.install_tools().await
}

/// Island / main: play_pause, pause, play, mute, back, forward, seek (value = ms).
#[tauri::command]
pub fn video_control(
    video: State<'_, VideoHandle>,
    action: String,
    value: Option<f64>,
) -> IpcResult<()> {
    const ALLOWED: [&str; 7] = [
        "play_pause",
        "pause",
        "play",
        "mute",
        "back",
        "forward",
        "seek",
    ];
    if !ALLOWED.contains(&action.as_str()) {
        return Err(IpcError::new("video.bad_action", action));
    }
    video.command(&action, value.filter(|v| v.is_finite()));
    Ok(())
}

#[tauri::command]
pub fn video_open_in_browser(video: State<'_, VideoHandle>) -> IpcResult<()> {
    video.open_in_browser()
}

/// pin / unpin / reset / show / close.
#[tauri::command]
pub async fn video_window(video: State<'_, VideoHandle>, action: String) -> IpcResult<()> {
    video.window_action(&action)
}

#[tauri::command]
pub fn video_current(
    webview: Webview,
    video: State<'_, VideoHandle>,
) -> IpcResult<Option<PlaybackSource>> {
    from_video_window(&webview)?;
    Ok(video.current())
}

#[tauri::command]
pub fn video_report(
    webview: Webview,
    video: State<'_, VideoHandle>,
    report: VideoReport,
) -> IpcResult<()> {
    from_video_window(&webview)?;
    video.report(report);
    Ok(())
}

/// The signed URL expired (403/410): resolve again and resume at `position_ms`.
#[tauri::command]
pub async fn video_refresh(
    webview: Webview,
    video: State<'_, VideoHandle>,
    position_ms: f64,
) -> IpcResult<PlaybackSource> {
    from_video_window(&webview)?;
    video.reload(position_ms.max(0.0) as u64, None).await
}

/// `height = None` → automatic (HLS when available).
#[tauri::command]
pub async fn video_set_quality(
    webview: Webview,
    video: State<'_, VideoHandle>,
    height: Option<u32>,
    position_ms: f64,
) -> IpcResult<PlaybackSource> {
    from_video_window(&webview)?;
    video
        .reload(position_ms.max(0.0) as u64, Some(height))
        .await
}

/// The only command the hidden sniffing view can call (capabilities/video-sniff-remote.json).
#[tauri::command]
pub fn video_sniff_report(
    webview: Webview,
    video: State<'_, VideoHandle>,
    request_id: String,
    items: Vec<SniffItem>,
    blobs: u32,
    done: bool,
) -> IpcResult<()> {
    super::sniff::report(
        video.sniff_state(),
        &webview,
        &request_id,
        items,
        blobs,
        done,
    )
}
