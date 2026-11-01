//! SQLite storage with embedded migrations (docs/modules/settings-storage.md §5).
//! One connection behind a mutex; queries are short, heavy work runs in `spawn_blocking`.

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use rusqlite::Connection;

/// Ordered, append-only. Never edit a migration that has shipped.
const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0001_core_kv",
        include_str!("../../migrations/0001_core_kv.sql"),
    ),
    (
        "0002_timer",
        include_str!("../../migrations/0002_timer.sql"),
    ),
    (
        "0003_reminders",
        include_str!("../../migrations/0003_reminders.sql"),
    ),
    (
        "0004_notes",
        include_str!("../../migrations/0004_notes.sql"),
    ),
    (
        "0005_shelf",
        include_str!("../../migrations/0005_shelf.sql"),
    ),
    (
        "0006_local_player",
        include_str!("../../migrations/0006_local_player.sql"),
    ),
    (
        "0007_lyrics",
        include_str!("../../migrations/0007_lyrics.sql"),
    ),
    (
        "0008_note_tasks",
        include_str!("../../migrations/0008_note_tasks.sql"),
    ),
];

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error("database was created by a newer PCisland (schema {found}, supported {supported})")]
    TooNew { found: usize, supported: usize },
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone)]
pub struct Db(Arc<Mutex<Connection>>);

impl Db {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, StorageError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self, StorageError> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(2))?;
        migrate(&mut conn)?;
        Ok(Self(Arc::new(Mutex::new(conn))))
    }

    pub fn call<T>(
        &self,
        f: impl FnOnce(&Connection) -> rusqlite::Result<T>,
    ) -> Result<T, StorageError> {
        let conn = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(f(&conn)?)
    }

    pub fn kv_get(&self, key: &str) -> Option<String> {
        self.call(|c| {
            c.query_row("SELECT value FROM kv WHERE key = ?1", [key], |r| r.get(0))
                .map(Some)
                .or_else(|e| {
                    if e == rusqlite::Error::QueryReturnedNoRows {
                        Ok(None)
                    } else {
                        Err(e)
                    }
                })
        })
        .ok()
        .flatten()
    }

    pub fn kv_set(&self, key: &str, value: &str) -> Result<(), StorageError> {
        self.call(|c| {
            c.execute(
                "INSERT INTO kv (key, value, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                rusqlite::params![key, value, crate::ipc::now_ms() as i64],
            )
            .map(|_| ())
        })
    }
}

fn migrate(conn: &mut Connection) -> Result<(), StorageError> {
    let current: usize =
        conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))? as usize;
    if current > MIGRATIONS.len() {
        return Err(StorageError::TooNew {
            found: current,
            supported: MIGRATIONS.len(),
        });
    }
    for (index, (name, sql)) in MIGRATIONS.iter().enumerate().skip(current) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (index + 1) as i64)?;
        tx.commit()?;
        tracing::info!(module = "storage", migration = name, "applied migration");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_from_empty_and_kv_roundtrips() {
        let db = Db::open_in_memory().expect("db");
        let version: i64 = db
            .call(|c| c.pragma_query_value(None, "user_version", |r| r.get(0)))
            .expect("pragma");
        assert_eq!(version as usize, MIGRATIONS.len());
        assert_eq!(db.kv_get("missing"), None);
        db.kv_set("k", "v1").expect("set");
        db.kv_set("k", "v2").expect("set");
        assert_eq!(db.kv_get("k").as_deref(), Some("v2"));
    }
}
