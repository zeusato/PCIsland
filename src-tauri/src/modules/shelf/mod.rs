//! File shelf: park files on the island, drag them out later (docs/modules/file-shelf.md).
#![forbid(unsafe_code)]

use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::watch;
use ts_rs::TS;

use crate::core::activity::{Activity, ActivityAction, ActivityHub};
use crate::core::id::new_id;
use crate::core::priority;
use crate::core::settings::Settings;
use crate::core::storage::{Db, StorageError};
use crate::ipc::{IpcError, IpcResult, events, now_ms};

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ShelfItem {
    pub id: String,
    pub path: String,
    pub name: String,
    #[ts(type = "number")]
    pub size: u64,
    pub is_dir: bool,
    #[ts(type = "number")]
    pub added_at: u64,
    pub missing: bool,
}

#[derive(Clone)]
pub struct ShelfHandle {
    db: Db,
    hub: ActivityHub,
    app: AppHandle,
    settings: watch::Receiver<Settings>,
}

pub fn setup(
    app: &AppHandle,
    db: Db,
    hub: ActivityHub,
    settings: watch::Receiver<Settings>,
) -> ShelfHandle {
    let handle = ShelfHandle {
        db,
        hub: hub.clone(),
        app: app.clone(),
        settings,
    };
    let for_actions = handle.clone();
    hub.register_handler(
        "shelf",
        std::sync::Arc::new(move |_, action| match action {
            "clear" => for_actions.clear().map_err(|e| e.to_string()),
            _ => Err(format!("unknown shelf action `{action}`")),
        }),
    );
    // Hourly expiry, only does work when the shelf has items.
    let for_expiry = handle.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            for_expiry.expire();
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    });
    handle
}

impl ShelfHandle {
    fn items(&self) -> Result<Vec<ShelfItem>, StorageError> {
        self.db.call(|c| {
            let mut s = c.prepare("SELECT id, path, name, size, is_dir, added_at FROM shelf_items ORDER BY added_at DESC")?;
            let rows = s.query_map([], |r| {
                let path: String = r.get(1)?;
                Ok(ShelfItem {
                    missing: !Path::new(&path).exists(),
                    id: r.get(0)?,
                    path,
                    name: r.get(2)?,
                    size: r.get::<_, i64>(3)? as u64,
                    is_dir: r.get(4)?,
                    added_at: r.get::<_, i64>(5)? as u64,
                })
            })?;
            rows.collect()
        })
    }

    fn add(&self, paths: &[String]) -> Result<u32, StorageError> {
        let max = self.settings.borrow().shelf.max_items as usize;
        let existing = self.items()?;
        let room = max.saturating_sub(existing.len());
        let mut added = 0;
        for p in paths
            .iter()
            .filter(|p| !existing.iter().any(|e| &e.path == *p))
            .take(room)
        {
            let path = Path::new(p);
            let Ok(meta) = std::fs::metadata(path) else {
                continue;
            };
            let name = path
                .file_name()
                .map_or_else(|| p.clone(), |n| n.to_string_lossy().into_owned());
            self.db.call(|c| {
                c.execute(
                    "INSERT INTO shelf_items (id, path, name, size, is_dir, added_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![new_id(), p, name, meta.len() as i64, meta.is_dir(), now_ms() as i64],
                )
            })?;
            added += 1;
        }
        self.publish();
        Ok(added)
    }

    fn clear(&self) -> Result<(), StorageError> {
        self.db
            .call(|c| c.execute("DELETE FROM shelf_items", []).map(|_| ()))?;
        self.publish();
        Ok(())
    }

    fn expire(&self) {
        let hours = u64::from(self.settings.borrow().shelf.expire_hours);
        if hours > 0 {
            let cutoff = now_ms().saturating_sub(hours * 3_600_000) as i64;
            let _ = self
                .db
                .call(|c| c.execute("DELETE FROM shelf_items WHERE added_at < ?1", [cutoff]));
        }
        self.publish();
    }

    fn publish(&self) {
        let items = self.items().unwrap_or_default();
        if items.is_empty() {
            self.hub.remove("shelf:items");
        } else {
            self.hub.upsert(
                Activity::new(
                    "shelf",
                    "shelf:items",
                    "shelf.items",
                    priority::AMBIENT,
                    serde_json::json!({ "count": items.len(), "items": items }),
                )
                .with_actions(vec![ActivityAction::new(
                    "clear",
                    "trash-2",
                    "shelf.action.clear",
                )]),
            );
        }
        for label in ["island", "main"] {
            let _ = self.app.emit_to(label, events::SHELF_STATE, &items);
        }
    }
}

fn db_err(e: StorageError) -> IpcError {
    IpcError::new("shelf.storage", e.to_string())
}

#[tauri::command]
pub fn shelf_list(shelf: State<'_, ShelfHandle>) -> IpcResult<Vec<ShelfItem>> {
    shelf.items().map_err(db_err)
}

#[tauri::command]
pub fn shelf_add(shelf: State<'_, ShelfHandle>, paths: Vec<String>) -> IpcResult<u32> {
    let added = shelf.add(&paths).map_err(db_err)?;
    if added > 0 {
        shelf.hub.alert(
            Activity::new(
                "shelf",
                "shelf:dropped",
                "shelf.dropped",
                priority::ALERT,
                serde_json::json!({ "count": added }),
            )
            .transient(1_500),
        );
    }
    Ok(added)
}

#[tauri::command]
pub fn shelf_remove(shelf: State<'_, ShelfHandle>, id: String) -> IpcResult<()> {
    shelf
        .db
        .call(|c| c.execute("DELETE FROM shelf_items WHERE id = ?1", [id]))
        .map_err(db_err)?;
    shelf.publish();
    Ok(())
}

#[tauri::command]
pub fn shelf_clear(shelf: State<'_, ShelfHandle>) -> IpcResult<()> {
    shelf.clear().map_err(db_err)
}

/// Opens the item, or its folder with the file selected.
#[tauri::command]
pub fn shelf_open(path: String, reveal: bool) -> IpcResult<()> {
    let mut cmd = std::process::Command::new("explorer");
    if reveal {
        cmd.arg(format!("/select,{path}"));
    } else {
        cmd.arg(&path);
    }
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| IpcError::new("shelf.open", e.to_string()))
}
