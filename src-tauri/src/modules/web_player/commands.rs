use tauri::{AppHandle, State, Webview};

use super::dto::{BridgeMessage, SiteDto, WebPlayerSnapshot};
use super::{WebPlayerHandle, sites};
use crate::ipc::{IpcError, IpcResult};

#[tauri::command]
pub async fn web_player_search(
    app: AppHandle,
    player: State<'_, WebPlayerHandle>,
    site_id: String,
    query: String,
) -> IpcResult<Vec<super::dto::WebQueueItem>> {
    super::search::run(&app, &player, &site_id, &query).await
}

#[tauri::command]
pub fn web_player_search_report(
    player: State<'_, WebPlayerHandle>,
    webview: Webview,
    request_id: String,
    items: Vec<super::dto::WebQueueItem>,
) -> IpcResult<()> {
    super::search::report(&player, &webview, &request_id, items)
}

#[tauri::command]
pub async fn web_player_play_result(
    app: AppHandle,
    player: State<'_, WebPlayerHandle>,
    site_id: String,
    url: String,
) -> IpcResult<()> {
    super::search::play(&app, &player, &site_id, &url)
}

#[tauri::command]
pub fn web_player_get_state(player: State<'_, WebPlayerHandle>) -> WebPlayerSnapshot {
    player.snapshot()
}

#[tauri::command]
pub fn web_player_list_sites() -> Vec<SiteDto> {
    sites::SITES
        .iter()
        .map(|s| SiteDto {
            id: s.id.to_owned(),
            name: s.name.to_owned(),
            home_url: s.home_url.to_owned(),
        })
        .collect()
}

/// Async on purpose: window creation in a sync command deadlocks WebView2 on Windows.
#[tauri::command]
pub async fn web_player_open(
    app: AppHandle,
    player: State<'_, WebPlayerHandle>,
    site_id: Option<String>,
    url: Option<String>,
) -> IpcResult<()> {
    super::open(&app, &player, site_id.as_deref(), url.as_deref())
}

#[tauri::command]
pub fn web_player_show(app: AppHandle, player: State<'_, WebPlayerHandle>) -> IpcResult<()> {
    Ok(super::set_visible(&app, &player, true)?)
}

#[tauri::command]
pub fn web_player_hide(app: AppHandle, player: State<'_, WebPlayerHandle>) -> IpcResult<()> {
    Ok(super::set_visible(&app, &player, false)?)
}

#[tauri::command]
pub fn web_player_close(app: AppHandle, player: State<'_, WebPlayerHandle>) -> IpcResult<()> {
    Ok(super::close(&app, &player)?)
}

/// The only command remote pages can call (capabilities/web-player-remote.json).
#[tauri::command]
pub fn web_player_report(
    app: AppHandle,
    webview: Webview,
    player: State<'_, WebPlayerHandle>,
    msg: BridgeMessage,
) -> IpcResult<()> {
    if webview.label() != super::LABEL || msg.kind != "state" {
        return Err(IpcError::new(
            "web_player.rejected",
            "unexpected sender or message",
        ));
    }
    let url = webview.url()?;
    super::report(&app, &player, &url, msg)
}

#[tauri::command]
pub async fn web_player_resume(
    app: AppHandle,
    player: State<'_, WebPlayerHandle>,
    site_id: String,
) -> IpcResult<()> {
    super::resume(&app, &player, &site_id)
}

#[tauri::command]
pub fn web_player_select_queue(
    app: AppHandle,
    player: State<'_, WebPlayerHandle>,
    url: String,
) -> IpcResult<()> {
    super::select_queue(&app, &player, &url)
}

#[tauri::command]
pub fn web_player_request_queue(app: AppHandle) -> IpcResult<()> {
    use tauri::Manager;
    if let Some(win) = app.get_webview_window(super::LABEL) {
        win.eval("window.__pcisland?.requestQueue()")?;
    }
    Ok(())
}

#[tauri::command]
pub fn web_player_navigate(
    app: AppHandle,
    player: State<'_, WebPlayerHandle>,
    action: String,
) -> IpcResult<()> {
    super::navigate(&app, &player, &action)
}

#[tauri::command]
pub async fn web_player_clear_data(
    app: AppHandle,
    player: State<'_, WebPlayerHandle>,
) -> IpcResult<()> {
    let player = player.inner().clone();
    tauri::async_runtime::spawn_blocking(move || super::clear_data(&app, &player))
        .await
        .map_err(|e| IpcError::new("web_player.clear_failed", e.to_string()))?
}
