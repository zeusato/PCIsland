//! On-demand, editable note window. Closing it never cancels task alarms.
#![forbid(unsafe_code)]
use super::NotesHandle;
use crate::{
    core::storage::Db,
    ipc::{IpcError, IpcResult},
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
const LABEL: &str = "note-desktop";
#[derive(Serialize, Deserialize)]
pub struct DesktopState {
    pub id: String,
    pub font: u32,
    pub topmost: bool,
}
fn state(db: &Db) -> DesktopState {
    DesktopState {
        id: db.kv_get("notes.desktop.id").unwrap_or_default(),
        font: db
            .kv_get("notes.desktop.font")
            .and_then(|s| s.parse().ok())
            .unwrap_or(18),
        topmost: db.kv_get("notes.desktop.topmost").as_deref() != Some("false"),
    }
}
fn error(e: impl std::fmt::Display) -> IpcError {
    IpcError::new("notes.desktop", e.to_string())
}
#[tauri::command]
pub fn notes_desktop_state(db: State<'_, Db>) -> DesktopState {
    state(&db)
}
#[tauri::command]
pub async fn notes_desktop_open(
    app: AppHandle,
    notes: State<'_, NotesHandle>,
    id: String,
) -> IpcResult<()> {
    if !notes.list("").map_err(error)?.iter().any(|n| n.id == id) {
        return Err(error("Note no longer exists"));
    }
    if let Some(window) = app.get_webview_window(LABEL) {
        window.emit("notes://desktop-select", &id).map_err(error)?;
        window.show().map_err(error)?;
        window.set_focus().map_err(error)?;
    } else {
        notes.db.kv_set("notes.desktop.id", &id).map_err(error)?;
        let options = state(&notes.db);
        let window =
            WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::App("note-desktop.html".into()))
                .title("PCisland — Note")
                .additional_browser_args(crate::modules::island::APP_BROWSER_ARGS)
                .inner_size(440.0, 420.0)
                .min_inner_size(280.0, 240.0)
                .resizable(true)
                .decorations(false)
                .always_on_top(options.topmost)
                .visible(false)
                .build()
                .map_err(error)?;
        crate::app::main_window::restore_with_key(&window, "notes.desktop.geometry");
    }
    Ok(())
}
#[tauri::command]
pub fn notes_desktop_options(
    app: AppHandle,
    db: State<'_, Db>,
    font: u32,
    topmost: bool,
    id: Option<String>,
) -> IpcResult<()> {
    if let Some(id) = id {
        db.kv_set("notes.desktop.id", &id).map_err(error)?;
    }
    db.kv_set("notes.desktop.font", &font.clamp(12, 40).to_string())
        .map_err(error)?;
    db.kv_set(
        "notes.desktop.topmost",
        if topmost { "true" } else { "false" },
    )
    .map_err(error)?;
    if let Some(window) = app.get_webview_window(LABEL) {
        window.set_always_on_top(topmost).map_err(error)?;
    }
    Ok(())
}
