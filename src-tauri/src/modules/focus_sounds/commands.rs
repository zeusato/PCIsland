//! IPC of the focus music downloads (docs/modules/focus-sounds.md §6).
use tauri::State;
use tauri::ipc::Channel;

use super::{FocusCatalog, FocusDownloadProgress, FocusSoundsHandle, FocusSoundsStatus};
use crate::ipc::{IpcError, IpcResult};

fn error(message: impl Into<String>) -> IpcError {
    IpcError::new("focus_sounds", message.into())
}

#[tauri::command]
pub fn focus_sounds_status(sounds: State<'_, FocusSoundsHandle>) -> FocusSoundsStatus {
    super::status(sounds.dir())
}

#[tauri::command]
pub async fn focus_sounds_catalog(sounds: State<'_, FocusSoundsHandle>) -> IpcResult<FocusCatalog> {
    let manifest = sounds.fetch_manifest().await.map_err(error)?;
    Ok(super::catalog(
        &manifest,
        &super::read_installed(sounds.dir()),
        sounds.dir(),
    ))
}

#[tauri::command]
pub async fn focus_sounds_download(
    sounds: State<'_, FocusSoundsHandle>,
    packs: Vec<String>,
    progress: Channel<FocusDownloadProgress>,
) -> IpcResult<FocusSoundsStatus> {
    let sounds = sounds.inner().clone();
    sounds
        .download(&packs, |p| {
            let _ = progress.send(p);
        })
        .await
        .map_err(error)
}

#[tauri::command]
pub fn focus_sounds_cancel(sounds: State<'_, FocusSoundsHandle>) {
    sounds.cancel();
}

#[tauri::command]
pub fn focus_sounds_remove(
    sounds: State<'_, FocusSoundsHandle>,
    packs: Vec<String>,
) -> IpcResult<FocusSoundsStatus> {
    sounds.remove(&packs).map_err(error)
}
