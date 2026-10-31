use tauri::State;

use super::{ParsedPreview, Reminder, ReminderInput, RemindersHandle};
use crate::core::storage::StorageError;
use crate::ipc::{IpcError, IpcResult};

fn db_err(e: StorageError) -> IpcError {
    IpcError::new("reminders.storage", e.to_string())
}

#[tauri::command]
pub fn reminders_list(
    reminders: State<'_, RemindersHandle>,
    filter: String,
) -> IpcResult<Vec<Reminder>> {
    reminders.list(&filter).map_err(db_err)
}

#[tauri::command]
pub fn reminders_create(
    reminders: State<'_, RemindersHandle>,
    input: ReminderInput,
) -> IpcResult<Reminder> {
    if input.title.trim().is_empty() {
        return Err(IpcError::new("reminders.empty_title", "title is required"));
    }
    reminders.create(input).map_err(db_err)
}

#[tauri::command]
pub fn reminders_update(
    reminders: State<'_, RemindersHandle>,
    id: String,
    input: ReminderInput,
) -> IpcResult<()> {
    reminders.update(&id, input).map_err(db_err)
}

#[tauri::command]
pub fn reminders_delete(reminders: State<'_, RemindersHandle>, id: String) -> IpcResult<()> {
    reminders.delete(&id).map_err(db_err)
}

#[tauri::command]
pub fn reminders_complete(reminders: State<'_, RemindersHandle>, id: String) {
    reminders.complete(&id);
}

#[tauri::command]
pub fn reminders_snooze(reminders: State<'_, RemindersHandle>, id: String, minutes: u32) {
    reminders.snooze(&id, minutes);
}

#[tauri::command]
pub fn reminders_parse(text: String) -> ParsedPreview {
    super::parse_preview(&text)
}
