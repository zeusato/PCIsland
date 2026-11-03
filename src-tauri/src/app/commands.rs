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

/// Copies the link of the playing track for sharing (docs/modules/media-hub.md).
/// Only plain http(s) links without credentials are accepted.
#[tauri::command]
pub fn app_copy_link(window: tauri::WebviewWindow, url: String) -> IpcResult<()> {
    let error = |m: String| IpcError::new("app.copy_link", m);
    let parsed = shareable_link(&url).ok_or_else(|| error("not a shareable link".into()))?;
    let owner = crate::platform::window::RawHwnd::from_hwnd(
        window.hwnd().map_err(|e| error(e.to_string()))?,
    );
    crate::platform::clipboard::write_text(parsed.as_str(), owner).map_err(|e| error(e.to_string()))
}

fn shareable_link(url: &str) -> Option<tauri::Url> {
    let parsed = tauri::Url::parse(url.trim()).ok()?;
    (url.len() <= 2048
        && matches!(parsed.scheme(), "https" | "http")
        && parsed.host_str().is_some()
        && parsed.username().is_empty()
        && parsed.password().is_none())
    .then_some(parsed)
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
    updater: State<'_, super::update::Updater>,
) -> IpcResult<Option<super::update::UpdateInfo>> {
    updater
        .check(true, &db)
        .await
        .map_err(|e| IpcError::new("app.update_failed", e))
}

/// Applies the already verified package, then reopens the application.
#[tauri::command]
pub fn app_install_update(
    app: AppHandle,
    updater: State<'_, super::update::Updater>,
) -> IpcResult<()> {
    if !updater
        .apply(true)
        .map_err(|e| IpcError::new("app.update_failed", e))?
    {
        return Err(IpcError::new("app.no_update", "No update is ready"));
    }
    app.exit(0);
    Ok(())
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

#[cfg(test)]
mod tests {
    #[test]
    fn only_plain_web_links_are_shareable() {
        assert!(super::shareable_link("https://open.spotify.com/track/abc").is_some());
        assert!(super::shareable_link("https://user:pw@example.com/").is_none());
        assert!(super::shareable_link("vstream://local/1").is_none());
        assert!(super::shareable_link("file:///C:/music.mp3").is_none());
        assert!(super::shareable_link(&format!("https://a.com/{}", "x".repeat(2100))).is_none());
    }
}
