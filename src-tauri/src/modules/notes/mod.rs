//! Quick notes: capture from the island, pin one, search (docs/modules/quick-notes.md).
#![forbid(unsafe_code)]
pub mod desktop;
pub mod tasks;

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
    changed: std::sync::Arc<tokio::sync::Notify>,
}

pub fn setup(
    app: &AppHandle,
    db: Db,
    hub: ActivityHub,
    trash_days: u32,
    bus: crate::core::events::EventBus,
    audio: crate::core::audio::AudioEngine,
    settings: tokio::sync::watch::Receiver<crate::core::settings::Settings>,
) -> NotesHandle {
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
        changed: std::sync::Arc::default(),
    };
    let for_actions = handle.clone();
    hub.register_handler(
        "notes",
        std::sync::Arc::new(move |id, action| match action {
            "unpin" => for_actions.pin(None).map_err(|e| e.to_string()),
            "complete" => for_actions
                .complete_task(id.trim_start_matches("notes:task:"))
                .map_err(|e| e.to_string()),
            _ => Err(format!("unknown notes action `{action}`")),
        }),
    );
    handle.publish();
    spawn_alarms(handle.clone(), bus, audio, settings);
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

    fn save(
        &self,
        id: Option<&str>,
        text: &str,
        expected: Option<u64>,
    ) -> Result<Note, StorageError> {
        let id = id.map_or_else(new_id, str::to_owned);
        self.db
            .call(|c| persist_note(c, &id, text, expected, now_ms()))?;
        self.publish();
        self.list("")?
            .into_iter()
            .find(|n| n.id == id)
            .ok_or(StorageError::Sqlite(rusqlite::Error::QueryReturnedNoRows))
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
        self.changed.notify_one();
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
        for label in ["island", "main", "note-desktop"] {
            let _ = self.app.emit_to(label, events::NOTES_STATE, &pinned);
        }
    }

    fn complete_task(&self, task_id: &str) -> Result<(), StorageError> {
        let (id, text, line, checked, version): (String, String, u32, bool, u64) = self.db.call(|c| c.query_row("SELECT n.id,n.text,t.line_index,t.checked,n.updated_at FROM note_tasks t JOIN notes_items n ON n.id=t.note_id WHERE t.id=?1 AND n.deleted_at IS NULL", [task_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get::<_, i64>(4)? as u64))))?;
        if !checked {
            self.save(Some(&id), &toggle_line(&text, line as usize), Some(version))?;
        }
        Ok(())
    }
}

/// CAS and task synchronization share the same SQLite transaction.
fn persist_note(
    c: &rusqlite::Connection,
    id: &str,
    text: &str,
    expected: Option<u64>,
    now: u64,
) -> rusqlite::Result<()> {
    use rusqlite::OptionalExtension;
    let text: String = text.chars().take(MAX_LEN).collect();
    let tx = c.unchecked_transaction()?;
    let previous: Option<(u64, bool)> = tx
        .query_row(
            "SELECT updated_at,deleted_at IS NOT NULL FROM notes_items WHERE id=?1",
            [id],
            |r| Ok((r.get::<_, i64>(0)? as u64, r.get(1)?)),
        )
        .optional()?;
    let now = match previous {
        Some((version, false)) if expected == Some(version) => now.max(version + 1),
        None if expected.is_none() => now,
        _ => {
            return Err(rusqlite::Error::InvalidParameterName(
                "notes.conflict".into(),
            ));
        }
    };
    tx.execute("INSERT INTO notes_items (id,text,normalized,pinned,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4) ON CONFLICT(id) DO UPDATE SET text=?2,normalized=?3,updated_at=?4",rusqlite::params![id,text,normalize(&text),now as i64])?;
    tasks::sync(&tx, id, &text, now)?;
    tx.commit()
}

fn spawn_alarms(
    handle: NotesHandle,
    bus: crate::core::events::EventBus,
    audio: crate::core::audio::AudioEngine,
    settings: tokio::sync::watch::Receiver<crate::core::settings::Settings>,
) {
    let mut events = bus.subscribe();
    tauri::async_runtime::spawn(async move {
        let mut shown = std::collections::HashSet::<String>::new();
        let mut hidden = false;
        loop {
            let now = now_ms();
            let due = handle
                .db
                .call(|c| tasks::fire_due(c, now))
                .unwrap_or_default();
            let next: std::collections::HashSet<_> =
                due.iter().map(|task| task.id.clone()).collect();
            for id in shown.difference(&next) {
                handle.hub.remove(&format!("notes:task:{id}"));
            }
            for task in due.iter().filter(|task| !shown.contains(&task.id)) {
                handle.hub.upsert(
                    Activity::new(
                        "notes",
                        format!("notes:task:{}", task.id),
                        "notes.task_due",
                        priority::CRITICAL,
                        crate::core::activity::payload(task),
                    )
                    .with_actions(vec![
                        ActivityAction::new("complete", "check", "reminders.action.done").primary(),
                    ]),
                );
                let settings = settings.borrow().clone();
                if let Some(sound) =
                    crate::core::audio::SoundId::named(&settings.app.sound_reminder)
                {
                    audio.play(sound, settings.app.sound_volume);
                }
                if hidden {
                    crate::platform::toast::show(
                        &handle.app.config().identifier,
                        "PCisland",
                        &task.label,
                    );
                }
            }
            if next != shown {
                for window in ["island", "main", "note-desktop"] {
                    let _ = handle
                        .app
                        .emit_to(window, events::NOTES_STATE, Option::<Note>::None);
                }
            }
            shown = next;
            let wait = handle
                .db
                .call(tasks::next_due)
                .ok()
                .flatten()
                .map_or(60000, |due| due.saturating_sub(now).clamp(1, 60000));
            tokio::select! {
                () = tokio::time::sleep(std::time::Duration::from_millis(wait)) => {},
                () = handle.changed.notified() => {},
                event = events.recv() => { if let Ok(crate::core::events::AppEvent::IslandHidden(value)) = event { hidden = value; } }
            }
        }
    });
}

#[tauri::command]
pub fn notes_tasks(notes: State<'_, NotesHandle>, id: String) -> IpcResult<Vec<tasks::NoteTask>> {
    notes
        .db
        .call(|c| {
            let text: String = c.query_row(
                "SELECT text FROM notes_items WHERE id=?1 AND deleted_at IS NULL",
                [&id],
                |r| r.get(0),
            )?;
            let current = tasks::list(c, &id)?;
            if !current.is_empty() {
                return Ok(current);
            }
            let tx = c.unchecked_transaction()?;
            tasks::sync(&tx, &id, &text, now_ms())?;
            tx.commit()?;
            tasks::list(c, &id)
        })
        .map_err(db_err)
}

#[tauri::command]
pub fn notes_set_task_alarm(
    notes: State<'_, NotesHandle>,
    task_id: String,
    input: Option<tasks::TaskAlarmInput>,
) -> IpcResult<()> {
    notes
        .db
        .call(|c| {
            let tx = c.unchecked_transaction()?;
            tasks::set_alarm(&tx, &task_id, input.as_ref(), now_ms())?;
            tx.commit()
        })
        .map_err(db_err)?;
    notes.publish();
    Ok(())
}

fn db_err(e: StorageError) -> IpcError {
    IpcError::new("notes.storage", e.to_string())
}

/// All live notes as one Markdown document, newest first, pinned on top.
pub fn to_markdown(notes: &[Note]) -> String {
    use chrono::{Local, TimeZone};
    let mut out = String::from(
        "# PCisland notes
",
    );
    for n in notes {
        let when = Local
            .timestamp_millis_opt(n.updated_at as i64)
            .single()
            .map_or_else(String::new, |d| d.format("%Y-%m-%d %H:%M").to_string());
        let pin = if n.pinned { " 📌" } else { "" };
        // Checkbox lines become Markdown task items.
        let body: Vec<String> = n
            .text
            .lines()
            .map(|l| {
                if let Some((prefix, checked, rest)) = checkbox_parts(l) {
                    let indent = prefix.strip_suffix("- ").unwrap_or(prefix);
                    format!("{indent}- [{}]{rest}", if checked { "x" } else { " " })
                } else {
                    l.to_owned()
                }
            })
            .collect();
        out.push_str(&format!(
            "
## {when}{pin}

{}
",
            body.join(
                "
"
            )
            .trim_end()
        ));
    }
    out
}

/// Writes every live note to `path` as Markdown; returns how many were written.
#[tauri::command]
pub fn notes_export(notes: State<'_, NotesHandle>, path: String) -> IpcResult<u32> {
    let all = notes.list("").map_err(db_err)?;
    std::fs::write(&path, to_markdown(&all))
        .map_err(|e| IpcError::new("notes.export", e.to_string()))?;
    Ok(all.len() as u32)
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
    expected_updated_at: Option<u64>,
) -> IpcResult<Note> {
    if text.trim().is_empty() {
        return Err(IpcError::new("notes.empty", "empty note"));
    }
    let note = notes
        .save(id.as_deref(), &text, expected_updated_at)
        .map_err(db_err)?;
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

/// Returns the prefix, checked state and remaining text for supported checkbox forms.
fn checkbox_parts(line: &str) -> Option<(&str, bool, &str)> {
    let trimmed = line.trim_start();
    let body = trimmed.strip_prefix("- ").unwrap_or(trimmed);
    for (marker, checked) in [("[]", false), ("[ ]", false), ("[x]", true), ("[X]", true)] {
        if let Some(rest) = body.strip_prefix(marker) {
            return Some((&line[..line.len() - body.len()], checked, rest));
        }
    }
    None
}

/// Toggles shorthand and Markdown checkboxes, preserving indentation and line indices.
pub fn toggle_line(text: &str, line: usize) -> String {
    text.split('\n')
        .enumerate()
        .map(|(i, l)| {
            if i != line {
                return l.to_owned();
            }
            if let Some((prefix, checked, rest)) = checkbox_parts(l) {
                format!("{prefix}[{}]{rest}", if checked { " " } else { "x" })
            } else {
                l.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[tauri::command]
pub fn notes_toggle_check(notes: State<'_, NotesHandle>, id: String, line: u32) -> IpcResult<Note> {
    let (text, version): (String, u64) = notes
        .db
        .call(|c| {
            c.query_row(
                "SELECT text, updated_at FROM notes_items WHERE id = ?1 AND deleted_at IS NULL",
                [&id],
                |r| Ok((r.get(0)?, r.get::<_, i64>(1)? as u64)),
            )
        })
        .map_err(db_err)?;
    notes
        .save(Some(&id), &toggle_line(&text, line as usize), Some(version))
        .map_err(db_err)
}

#[cfg(test)]
mod tests {
    #[test]
    fn concurrent_note_edits_do_not_overwrite_or_resurrect_deleted_notes() {
        let db = crate::core::storage::Db::open_in_memory().unwrap();
        db.call(|c| super::persist_note(c, "n", "[ ] first", None, 100))
            .unwrap();
        db.call(|c| super::persist_note(c, "n", "[ ] newer", Some(100), 100))
            .unwrap();
        assert!(
            db.call(|c| super::persist_note(c, "n", "stale", Some(100), 100))
                .is_err()
        );
        assert_eq!(
            db.call(
                |c| c.query_row("SELECT text FROM notes_items WHERE id='n'", [], |r| r
                    .get::<_, String>(0))
            )
            .unwrap(),
            "[ ] newer"
        );
        db.call(|c| c.execute("UPDATE notes_items SET deleted_at=102 WHERE id='n'", []))
            .unwrap();
        assert!(
            db.call(|c| super::persist_note(c, "n", "resurrect", Some(101), 103))
                .is_err()
        );
    }
    #[test]
    fn shorthand_uppercase_and_markdown_boxes_toggle_without_losing_lines() {
        let text = "[] việc một\n\n  [X] việc hai\n- [ ] việc ba\n";
        assert_eq!(
            super::toggle_line(text, 0),
            "[x] việc một\n\n  [X] việc hai\n- [ ] việc ba\n"
        );
        assert_eq!(
            super::toggle_line(text, 2),
            "[] việc một\n\n  [ ] việc hai\n- [ ] việc ba\n"
        );
        assert_eq!(
            super::toggle_line(text, 3),
            "[] việc một\n\n  [X] việc hai\n- [x] việc ba\n"
        );
        assert_eq!(super::toggle_line(text, 20), text);
    }

    #[test]
    fn exports_markdown_with_task_items() {
        let n = super::Note {
            id: "1".into(),
            text: "Mua:
[ ] sữa
  [x] trứng"
                .into(),
            pinned: true,
            created_at: 0,
            updated_at: 0,
        };
        let md = super::to_markdown(&[n]);
        assert!(md.starts_with(
            "# PCisland notes
"
        ));
        assert!(md.contains("📌"));
        assert!(md.contains(
            "Mua:
- [ ] sữa
  - [x] trứng"
        ));
    }

    #[test]
    fn toggles_only_the_given_checkbox_line() {
        let t = "Mua:\n[ ] sữa\n  [x] trứng";
        assert_eq!(super::toggle_line(t, 1), "Mua:\n[x] sữa\n  [x] trứng");
        assert_eq!(super::toggle_line(t, 2), "Mua:\n[ ] sữa\n  [ ] trứng");
        assert_eq!(super::toggle_line(t, 0), t);
    }
}
