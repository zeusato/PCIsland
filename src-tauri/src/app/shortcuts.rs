//! Global shortcuts (docs/modules/app-lifecycle.md §5). Failures are reported, never fatal.

use std::sync::{Mutex, PoisonError};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use ts_rs::TS;

use crate::core::settings::Shortcuts;
use crate::modules::island::{self, IslandHandle};

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutError {
    pub action: String,
    pub accelerator: String,
    pub reason: String,
}

/// Failures of the last registration, read by the settings page.
static LAST_ERRORS: Mutex<Vec<ShortcutError>> = Mutex::new(Vec::new());

pub fn last_errors() -> Vec<ShortcutError> {
    LAST_ERRORS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

pub fn register(app: &AppHandle, shortcuts: &Shortcuts) {
    let mut errors = Vec::new();
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let bindings: [(&'static str, &str); 5] = [
        ("togglePeek", &shortcuts.toggle_peek),
        ("openMain", &shortcuts.open_main),
        ("toggleHidden", &shortcuts.toggle_hidden),
        ("quickNote", &shortcuts.quick_note),
        ("videoOpen", &shortcuts.video_open),
    ];
    for (action, accel) in bindings {
        if accel.trim().is_empty() {
            continue;
        }
        let result = accel
            .parse::<Shortcut>()
            .map_err(|e| e.to_string())
            .and_then(|sc| {
                gs.on_shortcut(sc, move |app, _, event| {
                    if event.state() == ShortcutState::Pressed {
                        run(app, action);
                    }
                })
                .map_err(|e| e.to_string())
            });
        if let Err(reason) = result {
            tracing::warn!(module = "app", action, accelerator = accel, %reason, "shortcut registration failed");
            errors.push(ShortcutError {
                action: action.into(),
                accelerator: accel.into(),
                reason,
            });
        }
    }
    let _ = app.emit_to("main", "app://shortcut-errors", &errors);
    *LAST_ERRORS.lock().unwrap_or_else(PoisonError::into_inner) = errors;
}

fn run(app: &AppHandle, action: &str) {
    match action {
        "openMain" => {
            let _ = island::open_main(app, None);
        }
        "toggleHidden" => {
            let island = app.state::<IslandHandle>().inner().clone();
            tauri::async_runtime::spawn(async move {
                island.toggle_hidden().await;
            });
        }
        // The island frontend owns peek and the note input.
        other => {
            let _ = app.emit_to(island::LABEL, "island://hotkey", other);
        }
    }
}
