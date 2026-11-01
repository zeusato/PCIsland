//! Commands of the core services: settings, activity hub, app info, sounds.

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use ts_rs::TS;

use crate::core::activity::{ActivityHub, IslandState};
use crate::core::audio::{AudioEngine, SoundId};
use crate::core::paths::Paths;
use crate::core::settings::{Settings, SettingsHandle};
use crate::ipc::{IpcError, IpcResult};

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub portable: bool,
    pub data_dir: String,
    pub startup_command: Option<String>,
}

#[tauri::command]
pub fn app_get_info(paths: State<'_, Paths>) -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        portable: paths.portable,
        data_dir: paths.data_dir.display().to_string(),
        startup_command: crate::platform::autostart::registered_command(),
    }
}

#[tauri::command]
pub fn app_quit(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn app_show_toast(app: AppHandle, title: String, body: String) {
    crate::platform::toast::show_brief(
        &app.config().identifier,
        &title.chars().take(100).collect::<String>(),
        &body.chars().take(1500).collect::<String>(),
    );
}

#[tauri::command]
pub fn app_open_data_folder(paths: State<'_, Paths>) -> IpcResult<()> {
    std::process::Command::new("explorer")
        .arg(&paths.data_dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| IpcError::new("app.open_folder", e.to_string()))
}

#[tauri::command]
pub fn app_play_sound(
    audio: State<'_, AudioEngine>,
    settings: State<'_, SettingsHandle>,
    sound: SoundId,
) {
    audio.play(sound, settings.get().app.sound_volume);
}

#[tauri::command]
pub fn app_pause_alerts(hub: State<'_, ActivityHub>, minutes: Option<u32>) {
    hub.pause_alerts(minutes.map(|m| crate::ipc::now_ms() + u64::from(m) * 60_000));
}

#[tauri::command]
pub fn settings_get(settings: State<'_, SettingsHandle>) -> Settings {
    settings.get()
}

#[tauri::command]
pub fn settings_update(
    app: AppHandle,
    settings: State<'_, SettingsHandle>,
    section: String,
    patch: serde_json::Value,
) -> IpcResult<Settings> {
    let before = settings.get();
    let next = settings
        .update_section(&section, &patch)
        .map_err(|e| IpcError::new("settings.invalid", e))?;
    if section == "app" && patch.get("autostart").is_some() {
        if let Err(error) = super::sync_autostart(&app, next.app.autostart) {
            let _ = settings.update_section(
                "app",
                &serde_json::json!({"autostart": before.app.autostart}),
            );
            return Err(IpcError::new("app.autostart", error));
        }
    }
    if before.app.shortcuts != next.app.shortcuts {
        super::shortcuts::register(&app, &next.app.shortcuts);
    }
    Ok(next)
}

#[tauri::command]
pub async fn activity_get_state(hub: State<'_, ActivityHub>) -> IpcResult<IslandState> {
    Ok(hub.snapshot().await)
}

#[tauri::command]
pub fn activity_invoke(
    hub: State<'_, ActivityHub>,
    activity_id: String,
    action_id: String,
) -> IpcResult<()> {
    hub.invoke(&activity_id, &action_id)
        .map_err(|e| IpcError::new("activity.action_failed", e))
}

#[tauri::command]
pub fn activity_dismiss(app: AppHandle, hub: State<'_, ActivityHub>, activity_id: String) {
    hub.dismiss(&activity_id);
    app.state::<AudioEngine>().stop_alert();
}

#[tauri::command]
pub fn activity_pin(hub: State<'_, ActivityHub>, activity_id: Option<String>) {
    hub.pin(activity_id);
}

#[tauri::command]
pub async fn app_check_update_now(
    db: State<'_, crate::core::storage::Db>,
) -> IpcResult<Option<super::update::UpdateInfo>> {
    Ok(super::update::check(true, &db).await)
}

#[tauri::command]
pub fn app_shortcut_errors() -> Vec<super::shortcuts::ShortcutError> {
    super::shortcuts::last_errors()
}

#[tauri::command]
pub fn app_open_url(url: String) {
    super::update::open_url(&url);
}

#[tauri::command]
pub fn app_modules(
    sup: State<'_, crate::core::supervisor::Supervisor>,
) -> std::collections::BTreeMap<String, crate::core::supervisor::ModuleState> {
    sup.states()
}
