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

#[tauri::command]
pub fn island_open_main(app: AppHandle, route: Option<String>) -> IpcResult<()> {
    Ok(super::open_main(&app, route.as_deref())?)
}

#[tauri::command]
pub async fn island_toggle_hidden(island: State<'_, IslandHandle>) -> IpcResult<VisibilityMode> {
    island
        .toggle_hidden()
        .await
        .ok_or_else(|| IpcError::new("island.unavailable", "island service stopped"))
}
