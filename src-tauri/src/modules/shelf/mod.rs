//! File shelf: park files on the island, drag them out later (docs/modules/file-shelf.md).
#![forbid(unsafe_code)]
pub mod operations;

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
    /// Shell thumbnail served by the artwork protocol.
    pub thumbnail_url: Option<String>,
}

#[derive(Clone)]
pub struct ShelfHandle {
    db: Db,
    hub: ActivityHub,
    app: AppHandle,
    settings: watch::Receiver<Settings>,
    artwork: std::sync::Arc<crate::core::artwork::ArtworkCache>,
    /// path → thumbnail URL, computed once per item.
    thumbs: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, Option<String>>>>,
}

pub fn setup(
    app: &AppHandle,
    db: Db,
    hub: ActivityHub,
    settings: watch::Receiver<Settings>,
    artwork: std::sync::Arc<crate::core::artwork::ArtworkCache>,
) -> ShelfHandle {
    let handle = ShelfHandle {
        db,
        hub: hub.clone(),
        app: app.clone(),
        settings,
        artwork,
        thumbs: std::sync::Arc::default(),
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
        let mut items = self.raw_items()?;
        // Thumbnails are cached per session; compute missing ones once (e.g. after restart).
        let missing: Vec<String> = {
            let cache = self
                .thumbs
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            items
                .iter()
                .filter(|i| !cache.contains_key(&i.path))
                .take(50)
                .map(|i| i.path.clone())
                .collect()
        };
        for path in &missing {
            self.thumbnail(path);
        }
        let thumbs = self
            .thumbs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for item in &mut items {
            item.thumbnail_url = thumbs.get(&item.path).cloned().flatten();
        }
        Ok(items)
    }

    fn thumbnail(&self, path: &str) {
        let url = crate::platform::shell::thumbnail(path, 96)
            .map(|bmp| self.artwork.put(std::sync::Arc::new(bmp)));
        self.thumbs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(path.to_owned(), url);
    }

    fn raw_items(&self) -> Result<Vec<ShelfItem>, StorageError> {
        self.db.call(|c| {
            let mut s = c.prepare("SELECT id, path, name, size, is_dir, added_at FROM shelf_items ORDER BY added_at ASC, id ASC")?;
            let rows = s.query_map([], |r| {
                let path: String = r.get(1)?;
                Ok(ShelfItem {
                    missing: !Path::new(&path).exists(),
                    thumbnail_url: None,
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
        let existing = self.raw_items()?;
        let mut added = 0;
        let mut seen: std::collections::HashSet<String> =
            existing.iter().map(|item| item.path.clone()).collect();
        for p in paths {
            if !seen.insert(p.clone()) {
                continue;
            }
            let path = Path::new(p);
            let Ok(meta) = std::fs::metadata(path) else {
                continue;
            };
            let name = path
                .file_name()
                .map_or_else(|| p.clone(), |n| n.to_string_lossy().into_owned());
            let inserted = self.db.call(|c| {
                c.execute(
                    "INSERT INTO shelf_items (id, path, name, size, is_dir, added_at) SELECT ?1, ?2, ?3, ?4, ?5, ?6 WHERE NOT EXISTS(SELECT 1 FROM shelf_items WHERE path=?2)",
                    rusqlite::params![new_id(), p, name, meta.len() as i64, meta.is_dir(), now_ms() as i64],
                )
            })?;
            if inserted == 0 {
                continue;
            }
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
                    crate::core::activity::payload(&ShelfPayload {
                        count: items.len() as u32,
                        items: items.clone(),
                    }),
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

#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ShelfPayload {
    pub count: u32,
    pub items: Vec<ShelfItem>,
}

/// Preview image shown under the cursor while dragging items out of the shelf.
#[tauri::command]
pub fn shelf_drag_icon() -> IpcResult<String> {
    let path = std::env::temp_dir().join("pcisland-drag.png");
    if !path.exists() {
        std::fs::write(&path, include_bytes!("../../../icons/32x32.png"))
            .map_err(|e| IpcError::new("shelf.drag_icon", e.to_string()))?;
    }
    Ok(path.to_string_lossy().into_owned())
}
