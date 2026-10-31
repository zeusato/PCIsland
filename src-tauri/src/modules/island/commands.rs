use tauri::{AppHandle, State};

use super::IslandHandle;
use super::dto::{HitRect, IslandSnapshot, VisibilityMode};
use crate::ipc::{IpcError, IpcResult};

#[tauri::command]
pub fn island_set_hit_regions(
    island: State<'_, IslandHandle>,
    rects: Vec<HitRect>,
) -> IpcResult<()> {
    Ok(island.set_hit_regions(&rects)?)
}

#[tauri::command]
pub fn island_get_state(island: State<'_, IslandHandle>) -> IslandSnapshot {
    IslandSnapshot {
        visibility: island.visibility(),
        scale_factor: island.scale_factor(),
    }
}

/// Async on purpose: creating a WebView2 window inside a sync command (main thread)
/// deadlocks on Windows and freezes all IPC.
#[tauri::command]
pub async fn island_open_main(app: AppHandle, route: Option<String>) -> IpcResult<()> {
    Ok(super::open_main(&app, route.as_deref())?)
}

#[tauri::command]
pub async fn island_toggle_hidden(island: State<'_, IslandHandle>) -> IpcResult<VisibilityMode> {
    island
        .toggle_hidden()
        .await
        .ok_or_else(|| IpcError::new("island.unavailable", "island service stopped"))
}

#[tauri::command]
pub fn island_request_focus(island: State<'_, IslandHandle>) {
    island.request_focus();
}

#[tauri::command]
pub fn island_release_focus(island: State<'_, IslandHandle>) {
    island.release_focus();
}

/// Seconds since the last user input (sleepy eyes, docs/04-design-system.md §11).
#[tauri::command]
pub fn island_idle_seconds() -> u32 {
    crate::platform::shell::idle_seconds()
}
