//! Stable checklist identities and persistent per-task alarm rules.
use crate::core::id::new_id;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct TaskAlarmInput {
    /// at | afterNow | afterPrevious
    pub mode: String,
    #[ts(type = "number | null")]
    pub at: Option<u64>,
    #[ts(type = "number | null")]
    pub delay_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct NoteTask {
    pub id: String,
    pub note_id: String,
    pub line_index: u32,
    pub label: String,
    pub checked: bool,
    pub alarm: Option<TaskAlarmInput>,
    #[ts(type = "number | null")]
    pub due_at: Option<u64>,
    #[ts(type = "number | null")]
    pub fired_at: Option<u64>,
}

pub fn list(c: &Connection, note_id: &str) -> rusqlite::Result<Vec<NoteTask>> {
    let mut statement = c.prepare("SELECT id,note_id,line_index,label,checked,alarm_mode,alarm_at,delay_ms,due_at,fired_at FROM note_tasks WHERE note_id=?1 ORDER BY line_index")?;
    statement
        .query_map([note_id], |row| {
            let mode: Option<String> = row.get(5)?;
            let at = row.get::<_, Option<i64>>(6)?.map(|v| v as u64);
            let delay_ms = row.get::<_, Option<i64>>(7)?.map(|v| v as u64);
            Ok(NoteTask {
                id: row.get(0)?,
                note_id: row.get(1)?,
                line_index: row.get(2)?,
                label: row.get(3)?,
                checked: row.get(4)?,
                alarm: mode.map(|mode| TaskAlarmInput { mode, at, delay_ms }),
                due_at: row.get::<_, Option<i64>>(8)?.map(|v| v as u64),
                fired_at: row.get::<_, Option<i64>>(9)?.map(|v| v as u64),
            })
        })?
        .collect()
}

/// Exact labels retain IDs across insertion/reordering; same-position renames retain
/// their IDs when the old task has not been matched elsewhere. Checkbox state is not identity.
pub fn sync(c: &Connection, note_id: &str, text: &str, now: u64) -> rusqlite::Result<()> {
    let old = list(c, note_id)?;
    let rows: Vec<_> = text
        .split('\n')
        .enumerate()
        .filter_map(|(line, text)| {
            super::checkbox_parts(text)
                .map(|(_, checked, label)| (line, checked, label.trim().to_owned()))
        })
        .collect();
    let mut used = std::collections::HashSet::new();
    let mut matches: Vec<Option<usize>> = rows
        .iter()
        .map(|(_, _, label)| {
            let found = old
                .iter()
                .enumerate()
                .position(|(index, task)| task.label == *label && !used.contains(&index));
            if let Some(index) = found {
                used.insert(index);
            }
            found
        })
        .collect();
    for (i, (line, _, _)) in rows.iter().enumerate() {
        if matches[i].is_none() {
            let found = old.iter().enumerate().position(|(index, task)| {
                task.line_index as usize == *line && !used.contains(&index)
            });
            if let Some(index) = found {
                matches[i] = Some(index);
                used.insert(index);
            }
        }
    }
    // Move indices out of the way before rearranging; this function runs in a transaction.
    c.execute(
        "UPDATE note_tasks SET line_index=line_index+1000000 WHERE note_id=?1",
        [note_id],
    )?;
    for (i, task) in old.iter().enumerate() {
        if !used.contains(&i) {
            c.execute("DELETE FROM note_tasks WHERE id=?1", [&task.id])?;
        }
    }
    let mut previous: Option<String> = None;
    for (i, (line, checked, label)) in rows.iter().enumerate() {
        let id = matches[i].map_or_else(new_id, |index| old[index].id.clone());
        c.execute("INSERT INTO note_tasks (id,note_id,line_index,label,checked,completed_at,predecessor_id) VALUES (?1,?2,?3,?4,?5,CASE WHEN ?5 THEN ?6 ELSE NULL END,?7)
            ON CONFLICT(id) DO UPDATE SET line_index=?3,label=?4,
            completed_at=CASE WHEN ?5 AND NOT note_tasks.checked THEN ?6 WHEN NOT ?5 THEN NULL ELSE note_tasks.completed_at END,
            due_at=CASE WHEN ?5 THEN NULL WHEN note_tasks.checked AND alarm_mode='afterNow' THEN ?6+delay_ms WHEN note_tasks.checked AND alarm_mode='at' THEN alarm_at ELSE due_at END,
            fired_at=CASE WHEN note_tasks.checked AND NOT ?5 THEN NULL ELSE fired_at END,
            checked=?5,predecessor_id=?7", params![id,note_id,*line as i64,label,checked,now as i64,previous])?;
        previous = Some(id);
    }
    refresh_dependencies(c, note_id)
}

fn refresh_dependencies(c: &Connection, note_id: &str) -> rusqlite::Result<()> {
    c.execute("UPDATE note_tasks SET due_at=(SELECT completed_at FROM note_tasks previous WHERE previous.id=note_tasks.predecessor_id AND previous.checked=1)+delay_ms
        WHERE note_id=?1 AND alarm_mode='afterPrevious' AND checked=0 AND fired_at IS NULL", [note_id])?;
    Ok(())
}

pub fn set_alarm(
    c: &Connection,
    task_id: &str,
    input: Option<&TaskAlarmInput>,
    now: u64,
) -> rusqlite::Result<()> {
    if let Some(input) = input {
        let delay = if input.mode == "at" {
            0
        } else {
            input.delay_ms.unwrap_or(0)
        };
        let valid = match input.mode.as_str() {
            "at" => input.at.is_some_and(|at| at > now && at < i64::MAX as u64),
            "afterNow" | "afterPrevious" => (1_000..=31_536_000_000).contains(&delay),
            _ => false,
        };
        if !valid {
            return Err(rusqlite::Error::InvalidParameterName(
                "invalid alarm time".into(),
            ));
        }
        let (note_id, checked, predecessor): (String, bool, Option<String>) = c.query_row(
            "SELECT note_id,checked,predecessor_id FROM note_tasks WHERE id=?1",
            [task_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        if checked || input.mode == "afterPrevious" && predecessor.is_none() {
            return Err(rusqlite::Error::InvalidParameterName(
                "task must be unfinished and have a previous task for this rule".into(),
            ));
        }
        let due = match input.mode.as_str() {
            "at" => input.at,
            "afterNow" => now.checked_add(delay),
            _ => None,
        };
        let fixed_at = if input.mode == "at" { input.at } else { None };
        c.execute("UPDATE note_tasks SET alarm_mode=?2,alarm_at=?3,delay_ms=?4,due_at=?5,fired_at=NULL WHERE id=?1", params![task_id,input.mode,fixed_at.map(|v|v as i64),delay as i64,due.map(|v|v as i64)])?;
        refresh_dependencies(c, &note_id)?;
    } else {
        c.execute("UPDATE note_tasks SET alarm_mode=NULL,alarm_at=NULL,delay_ms=NULL,due_at=NULL,fired_at=NULL WHERE id=?1", [task_id])?;
    }
    Ok(())
}

pub fn fire_due(c: &Connection, now: u64) -> rusqlite::Result<Vec<NoteTask>> {
    c.execute("UPDATE note_tasks SET fired_at=?1 WHERE checked=0 AND due_at<=?1 AND fired_at IS NULL AND note_id IN (SELECT id FROM notes_items WHERE deleted_at IS NULL)", [now as i64])?;
    let ids: Vec<String> = c.prepare("SELECT DISTINCT note_id FROM note_tasks WHERE checked=0 AND fired_at IS NOT NULL AND alarm_mode IS NOT NULL AND note_id IN (SELECT id FROM notes_items WHERE deleted_at IS NULL)")?.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
    let mut result = Vec::new();
    for id in ids {
        result.extend(
            list(c, &id)?
                .into_iter()
                .filter(|task| !task.checked && task.alarm.is_some() && task.fired_at.is_some()),
        );
    }
    Ok(result)
}

pub fn next_due(c: &Connection) -> rusqlite::Result<Option<u64>> {
    c.query_row("SELECT MIN(due_at) FROM note_tasks WHERE checked=0 AND fired_at IS NULL AND note_id IN (SELECT id FROM notes_items WHERE deleted_at IS NULL)", [], |r| r.get::<_, Option<i64>>(0).map(|value| value.map(|v| v as u64)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::storage::Db;
    fn fixture() -> Db {
        let db = Db::open_in_memory().expect("db");
        db.call(|c| c.execute("INSERT INTO notes_items(id,text,normalized,pinned,created_at,updated_at) VALUES ('n','','',0,0,0)", [])).expect("note");
        db
    }
    #[test]
    fn previous_task_completion_starts_persistent_countdown_once() {
        let db = fixture();
        db.call(|c| {
            sync(c, "n", "[] A\n[] B", 1000)?;
            let tasks = list(c, "n")?;
            set_alarm(
                c,
                &tasks[1].id,
                Some(&TaskAlarmInput {
                    mode: "afterPrevious".into(),
                    at: None,
                    delay_ms: Some(10000),
                }),
                1000,
            )?;
            assert_eq!(next_due(c)?, None);
            sync(c, "n", "[x] A\n[] B", 5000)?;
            assert_eq!(next_due(c)?, Some(15000));
            assert!(fire_due(c, 14999)?.is_empty());
            assert_eq!(fire_due(c, 15000)?.len(), 1);
            sync(c, "n", "[x] A\n[x] B", 16000)?;
            assert!(fire_due(c, 20000)?.is_empty());
            Ok(())
        })
        .expect("flow");
    }
    #[test]
    fn inserting_and_reordering_lines_keeps_alarm_on_its_task() {
        let db = fixture();
        db.call(|c| {
            sync(c, "n", "[] A\n[] B", 1000)?;
            let id = list(c, "n")?[1].id.clone();
            set_alarm(
                c,
                &id,
                Some(&TaskAlarmInput {
                    mode: "afterNow".into(),
                    at: None,
                    delay_ms: Some(5000),
                }),
                1000,
            )?;
            sync(c, "n", "Heading\n[] B\n\n[] A", 2000)?;
            let tasks = list(c, "n")?;
            assert_eq!(tasks[0].id, id);
            assert_eq!(tasks[0].line_index, 1);
            assert_eq!(tasks[0].due_at, Some(6000));
            sync(c, "n", "[] A", 3000)?;
            assert!(fire_due(c, 7000)?.is_empty());
            Ok(())
        })
        .expect("flow");
    }

    #[test]
    fn undoing_previous_task_cancels_then_restarts_countdown() {
        let db = fixture();
        db.call(|c| {
            sync(c, "n", "[] A\nText\n[] B", 1000)?;
            let id = list(c, "n")?[1].id.clone();
            set_alarm(
                c,
                &id,
                Some(&TaskAlarmInput {
                    mode: "afterPrevious".into(),
                    at: None,
                    delay_ms: Some(10000),
                }),
                1000,
            )?;
            sync(c, "n", "[x] A\nText\n[] B", 2000)?;
            assert_eq!(next_due(c)?, Some(12000));
            sync(c, "n", "[] A\nText\n[] B", 5000)?;
            assert_eq!(next_due(c)?, None);
            assert!(fire_due(c, 15000)?.is_empty());
            sync(c, "n", "[x] A\nText\n[] B", 16000)?;
            assert_eq!(next_due(c)?, Some(26000));
            assert_eq!(fire_due(c, 26000)?.len(), 1);
            sync(c, "n", "[x] A\nText\n[] Renamed B", 27000)?;
            assert_eq!(list(c, "n")?[1].id, id);
            assert_eq!(list(c, "n")?[1].fired_at, Some(26000));
            Ok(())
        })
        .expect("dependency undo");
    }

    #[test]
    fn fixed_deadlines_survive_reload_and_deleted_notes_do_not_ring() {
        let db = fixture();
        db.call(|c| {
            sync(c, "n", "[] A", 1000)?;
            let id = list(c, "n")?[0].id.clone();
            assert!(
                set_alarm(
                    c,
                    &id,
                    Some(&TaskAlarmInput {
                        mode: "afterPrevious".into(),
                        at: None,
                        delay_ms: Some(1000)
                    }),
                    1000
                )
                .is_err()
            );
            assert!(
                set_alarm(
                    c,
                    &id,
                    Some(&TaskAlarmInput {
                        mode: "at".into(),
                        at: Some(500),
                        delay_ms: None
                    }),
                    1000
                )
                .is_err()
            );
            set_alarm(
                c,
                &id,
                Some(&TaskAlarmInput {
                    mode: "at".into(),
                    at: Some(8000),
                    delay_ms: None,
                }),
                1000,
            )?;
            sync(c, "n", "[] A", 6000)?;
            assert_eq!(next_due(c)?, Some(8000));
            c.execute("UPDATE notes_items SET deleted_at=7000 WHERE id='n'", [])?;
            assert!(fire_due(c, 9000)?.is_empty());
            assert_eq!(next_due(c)?, None);
            c.execute("UPDATE notes_items SET deleted_at=NULL WHERE id='n'", [])?;
            assert_eq!(fire_due(c, 10000)?.len(), 1);
            set_alarm(c, &id, None, 11000)?;
            assert!(fire_due(c, 12000)?.is_empty());
            Ok(())
        })
        .expect("fixed deadline");
    }
}
