use std::path::PathBuf;

use tauri::State;

use super::library::{self, Folder, Playlist, ScanSummary, Track};
use super::queue::RepeatMode;
use super::{Cmd, LocalPlayerHandle, LocalPlayerSnapshot};
use crate::core::settings::SettingsHandle;
use crate::core::storage::StorageError;
use crate::ipc::{IpcError, IpcResult};
use crate::modules::media::MediaCommand;

fn db_err(e: StorageError) -> IpcError {
    IpcError::new("local_player.storage", e.to_string())
}

#[tauri::command]
pub async fn local_player_get_state(
    player: State<'_, LocalPlayerHandle>,
) -> IpcResult<LocalPlayerSnapshot> {
    Ok(player.snapshot().await)
}

#[tauri::command]
pub fn local_player_play_tracks(
    player: State<'_, LocalPlayerHandle>,
    track_ids: Vec<String>,
    start: u32,
) {
    player.send(Cmd::PlayTracks(track_ids, start as usize));
}

#[tauri::command]
pub fn local_player_play_files(player: State<'_, LocalPlayerHandle>, paths: Vec<String>) {
    player.send(Cmd::PlayFiles(
        paths.into_iter().map(PathBuf::from).collect(),
    ));
}

#[tauri::command]
pub fn local_player_enqueue(
    player: State<'_, LocalPlayerHandle>,
    track_ids: Vec<String>,
    next: bool,
) {
    player.send(Cmd::Enqueue(track_ids, next));
}

#[tauri::command]
pub fn local_player_control(
    player: State<'_, LocalPlayerHandle>,
    action: String,
    value: Option<f64>,
) -> IpcResult<()> {
    player.send(match action.as_str() {
        "play_pause" => Cmd::Control(MediaCommand::PlayPause),
        "play" => Cmd::Play,
        "pause" => Cmd::Control(MediaCommand::Pause),
        "next" => Cmd::Control(MediaCommand::Next),
        "previous" => Cmd::Control(MediaCommand::Previous),
        "seek" => Cmd::Seek(value.unwrap_or(0.0).max(0.0) as u64),
        "stop" => Cmd::Stop,
        "shuffle_on" => Cmd::Shuffle(true),
        "shuffle_off" => Cmd::Shuffle(false),
        "repeat_off" => Cmd::Repeat(RepeatMode::Off),
        "repeat_all" => Cmd::Repeat(RepeatMode::All),
        "repeat_one" => Cmd::Repeat(RepeatMode::One),
        _ => return Err(IpcError::new("local_player.bad_action", action)),
    });
    Ok(())
}

#[tauri::command]
pub fn local_player_set_volume(
    player: State<'_, LocalPlayerHandle>,
    settings: State<'_, SettingsHandle>,
    volume: f32,
) {
    let _ = settings.update_section("localPlayer", &serde_json::json!({ "volume": volume }));
    player.send(Cmd::Volume(volume.clamp(0.0, 1.0)));
}

#[tauri::command]
pub fn local_player_folders(player: State<'_, LocalPlayerHandle>) -> IpcResult<Vec<Folder>> {
    library::folders(&player.db).map_err(db_err)
}

#[tauri::command]
pub fn local_player_add_folder(
    player: State<'_, LocalPlayerHandle>,
    path: String,
) -> IpcResult<Folder> {
    if !std::path::Path::new(&path).is_dir() {
        return Err(IpcError::new("local_player.not_a_folder", path));
    }
    library::add_folder(&player.db, &path).map_err(db_err)
}

#[tauri::command]
pub fn local_player_remove_folder(
    player: State<'_, LocalPlayerHandle>,
    id: String,
) -> IpcResult<()> {
    library::remove_folder(&player.db, &id).map_err(db_err)
}

#[tauri::command]
pub async fn local_player_rescan(player: State<'_, LocalPlayerHandle>) -> IpcResult<ScanSummary> {
    let db = player.db.clone();
    tauri::async_runtime::spawn_blocking(move || library::scan(&db))
        .await
        .map_err(|e| IpcError::new("local_player.scan", e.to_string()))?
        .map_err(db_err)
}

#[tauri::command]
pub fn local_player_query(
    player: State<'_, LocalPlayerHandle>,
    search: String,
    limit: u32,
    offset: u32,
) -> IpcResult<Vec<Track>> {
    library::query(&player.db, &search, limit.min(500), offset).map_err(db_err)
}

#[tauri::command]
pub fn local_player_playlists(player: State<'_, LocalPlayerHandle>) -> IpcResult<Vec<Playlist>> {
    library::playlists(&player.db).map_err(db_err)
}

#[tauri::command]
pub fn local_player_playlist_create(
    player: State<'_, LocalPlayerHandle>,
    name: String,
) -> IpcResult<Playlist> {
    library::create_playlist(&player.db, &name).map_err(db_err)
}

#[tauri::command]
pub fn local_player_playlist_delete(
    player: State<'_, LocalPlayerHandle>,
    id: String,
) -> IpcResult<()> {
    library::delete_playlist(&player.db, &id).map_err(db_err)
}

#[tauri::command]
pub fn local_player_playlist_add(
    player: State<'_, LocalPlayerHandle>,
    id: String,
    track_ids: Vec<String>,
) -> IpcResult<()> {
    library::add_to_playlist(&player.db, &id, &track_ids).map_err(db_err)
}

#[tauri::command]
pub fn local_player_playlist_play(
    player: State<'_, LocalPlayerHandle>,
    id: String,
) -> IpcResult<()> {
    let ids = library::playlist_tracks(&player.db, &id).map_err(db_err)?;
    player.send(Cmd::PlayTracks(ids, 0));
    Ok(())
}
