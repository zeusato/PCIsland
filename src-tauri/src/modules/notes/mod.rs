//! Quick notes: capture from the island, pin one, search (docs/modules/quick-notes.md).
#![forbid(unsafe_code)]

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use ts_rs::TS;

use crate::core::activity::{Activity, ActivityAction, ActivityHub};
use crate::core::id::new_id;
use crate::core::priority;
use crate::core::storage::{Db, StorageError};
use crate::ipc::{IpcError, IpcResult, events, now_ms};
use crate::shared::nlp_vi::normalize;

const MAX_LEN: usize = 20_000;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub text: String,
    pub pinned: bool,
    #[ts(type = "number")]
    pub created_at: u64,
    #[ts(type = "number")]
    pub updated_at: u64,
}

#[derive(Clone)]
pub struct NotesHandle {
    db: Db,
    hub: ActivityHub,
    app: AppHandle,
}

pub fn setup(app: &AppHandle, db: Db, hub: ActivityHub, trash_days: u32) -> NotesHandle {
    let cutoff = now_ms().saturating_sub(u64::from(trash_days) * 86_400_000);
    let _ = db.call(|c| {
        c.execute(
            "DELETE FROM notes_items WHERE deleted_at IS NOT NULL AND deleted_at < ?1",
            [cutoff as i64],
        )
    });
    let handle = NotesHandle {
        db,
        hub: hub.clone(),
        app: app.clone(),
    };
    let for_actions = handle.clone();
    hub.register_handler(
        "notes",
        std::sync::Arc::new(move |_, action| match action {
            "unpin" => for_actions.pin(None).map_err(|e| e.to_string()),
            _ => Err(format!("unknown notes action `{action}`")),
        }),
    );
    handle.publish();
    handle
}

impl NotesHandle {
    fn list(&self, query: &str) -> Result<Vec<Note>, StorageError> {
        let pattern = format!("%{}%", normalize(query.trim()));
        self.db.call(|c| {
            let mut s = c.prepare(
                "SELECT id, text, pinned, created_at, updated_at FROM notes_items
                 WHERE deleted_at IS NULL AND normalized LIKE ?1 ORDER BY pinned DESC, updated_at DESC LIMIT 500",
            )?;
            let rows = s.query_map([pattern], |r| {
                Ok(Note {
                    id: r.get(0)?,
                    text: r.get(1)?,
                    pinned: r.get(2)?,
                    created_at: r.get::<_, i64>(3)? as u64,
                    updated_at: r.get::<_, i64>(4)? as u64,
                })
            })?;
            rows.collect()
        })
    }

    fn save(&self, id: Option<&str>, text: &str) -> Result<Note, StorageError> {
        let text: String = text.chars().take(MAX_LEN).collect();
        let now = now_ms();
        let id = id.map_or_else(new_id, str::to_owned);
        self.db.call(|c| {
            c.execute(
                "INSERT INTO notes_items (id, text, normalized, pinned, created_at, updated_at) VALUES (?1, ?2, ?3, 0, ?4, ?4)
                 ON CONFLICT(id) DO UPDATE SET text = ?2, normalized = ?3, updated_at = ?4",
                rusqlite::params![id, text, normalize(&text), now as i64],
            )
        })?;
        self.publish();
        Ok(Note {
            id,
            text,
            pinned: false,
            created_at: now,
            updated_at: now,
        })
    }

    fn pin(&self, id: Option<&str>) -> Result<(), StorageError> {
        self.db.call(|c| {
            c.execute("UPDATE notes_items SET pinned = 0", [])?;
            if let Some(id) = id {
                c.execute("UPDATE notes_items SET pinned = 1 WHERE id = ?1", [id])?;
            }
            Ok(())
        })?;
        self.publish();
        Ok(())
    }

    fn publish(&self) {
        let pinned = self
            .list("")
            .ok()
            .and_then(|l| l.into_iter().find(|n| n.pinned));
        match &pinned {
            Some(n) => self.hub.upsert(
                Activity::new(
                    "notes",
                    "notes:pinned",
                    "notes.pinned",
                    priority::AMBIENT,
                    serde_json::to_value(n).unwrap_or_default(),
                )
                .with_actions(vec![ActivityAction::new(
                    "unpin",
                    "pin-off",
                    "notes.action.unpin",
                )]),
            ),
            None => self.hub.remove("notes:pinned"),
        }
        for label in ["island", "main"] {
            let _ = self.app.emit_to(label, events::NOTES_STATE, &pinned);
        }
    }
}

fn db_err(e: StorageError) -> IpcError {
    IpcError::new("notes.storage", e.to_string())
}

#[tauri::command]
pub fn notes_list(notes: State<'_, NotesHandle>, query: Option<String>) -> IpcResult<Vec<Note>> {
    notes.list(query.as_deref().unwrap_or("")).map_err(db_err)
}

#[tauri::command]
pub fn notes_save(
    notes: State<'_, NotesHandle>,
    id: Option<String>,
    text: String,
) -> IpcResult<Note> {
    if text.trim().is_empty() {
        return Err(IpcError::new("notes.empty", "empty note"));
    }
    let note = notes.save(id.as_deref(), &text).map_err(db_err)?;
    if id.is_none() {
        notes.hub.alert(
            Activity::new(
                "notes",
                "notes:saved",
                "notes.saved",
                priority::ALERT,
                serde_json::Value::Null,
            )
            .transient(1_500),
        );
    }
    Ok(note)
}

#[tauri::command]
pub fn notes_delete(notes: State<'_, NotesHandle>, id: String) -> IpcResult<()> {
    notes
        .db
        .call(|c| {
            c.execute(
                "UPDATE notes_items SET deleted_at = ?1, pinned = 0 WHERE id = ?2",
                rusqlite::params![now_ms() as i64, id],
            )
        })
        .map_err(db_err)?;
    notes.publish();
    Ok(())
}

#[tauri::command]
pub fn notes_restore(notes: State<'_, NotesHandle>, id: String) -> IpcResult<()> {
    notes
        .db
        .call(|c| {
            c.execute(
                "UPDATE notes_items SET deleted_at = NULL WHERE id = ?1",
                [id],
            )
        })
        .map_err(db_err)?;
    notes.publish();
    Ok(())
}

#[tauri::command]
pub fn notes_pin(notes: State<'_, NotesHandle>, id: Option<String>) -> IpcResult<()> {
    notes.pin(id.as_deref()).map_err(db_err)
}

/// Toggles the `[ ]` / `[x]` checkbox on line `line` (docs/modules/quick-notes.md §5).
pub fn toggle_line(text: &str, line: usize) -> String {
    text.lines()
        .enumerate()
        .map(|(i, l)| {
            if i != line {
                return l.to_owned();
            }
            if let Some(rest) = l.trim_start().strip_prefix("[ ]") {
                format!("{}[x]{rest}", &l[..l.len() - l.trim_start().len()])
            } else if let Some(rest) = l.trim_start().strip_prefix("[x]") {
                format!("{}[ ]{rest}", &l[..l.len() - l.trim_start().len()])
            } else {
                l.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[tauri::command]
pub fn notes_toggle_check(notes: State<'_, NotesHandle>, id: String, line: u32) -> IpcResult<Note> {
    let text: String = notes
        .db
        .call(|c| {
            c.query_row("SELECT text FROM notes_items WHERE id = ?1", [&id], |r| {
                r.get(0)
            })
        })
        .map_err(db_err)?;
    notes
        .save(Some(&id), &toggle_line(&text, line as usize))
        .map_err(db_err)
}

#[cfg(test)]
mod tests {
    #[test]
    fn toggles_only_the_given_checkbox_line() {
        let t = "Mua:\n[ ] sữa\n  [x] trứng";
        assert_eq!(super::toggle_line(t, 1), "Mua:\n[x] sữa\n  [x] trứng");
        assert_eq!(super::toggle_line(t, 2), "Mua:\n[ ] sữa\n  [ ] trứng");
        assert_eq!(super::toggle_line(t, 0), t);
    }
}
