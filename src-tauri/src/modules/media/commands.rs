use tauri::State;

use super::dto::MediaSnapshot;
use super::{MediaCommand, MediaHandle};

#[tauri::command]
pub fn media_get_state(media: State<'_, MediaHandle>) -> MediaSnapshot {
    media.snapshot()
}

#[tauri::command]
pub fn media_play_pause(media: State<'_, MediaHandle>) {
    media.command(MediaCommand::PlayPause);
}

#[tauri::command]
pub fn media_next(media: State<'_, MediaHandle>) {
    media.command(MediaCommand::Next);
}

#[tauri::command]
pub fn media_previous(media: State<'_, MediaHandle>) {
    media.command(MediaCommand::Previous);
}
