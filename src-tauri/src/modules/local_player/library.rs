//! Music library: watched folders, incremental scan, tags (docs/modules/local-player.md §5).

use std::path::Path;
use std::time::UNIX_EPOCH;

use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::Accessor;
use serde::Serialize;
use ts_rs::TS;

use crate::core::id::new_id;
use crate::core::storage::{Db, StorageError};
use crate::ipc::now_ms;
use crate::shared::nlp_vi::normalize;

pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a", "aac"];

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub path: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    #[ts(type = "number | null")]
    pub duration_ms: Option<u64>,
    pub has_artwork: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub path: String,
    #[ts(type = "number | null")]
    pub last_scan_at: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub added: u32,
    pub updated: u32,
    pub removed: u32,
}

pub fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| AUDIO_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

pub fn add_folder(db: &Db, path: &str) -> Result<Folder, StorageError> {
    let f = Folder {
        id: new_id(),
        path: path.into(),
        last_scan_at: None,
    };
    db.call(|c| {
        c.execute(
            "INSERT OR IGNORE INTO local_player_folders (id, path, added_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![f.id, f.path, now_ms() as i64],
        )
    })?;
    Ok(f)
}

pub fn remove_folder(db: &Db, id: &str) -> Result<(), StorageError> {
    db.call(|c| {
        c.execute("DELETE FROM local_player_folders WHERE id = ?1", [id])
            .map(|_| ())
    })
}

pub fn folders(db: &Db) -> Result<Vec<Folder>, StorageError> {
    db.call(|c| {
        let mut s =
            c.prepare("SELECT id, path, last_scan_at FROM local_player_folders ORDER BY path")?;
        let rows = s.query_map([], |r| {
            Ok(Folder {
                id: r.get(0)?,
                path: r.get(1)?,
                last_scan_at: r.get::<_, Option<i64>>(2)?.map(|v| v as u64),
            })
        })?;
        rows.collect()
    })
}

/// Reads tags; falls back to the file name for the title.
pub fn read_track(path: &Path) -> (String, Option<String>, Option<String>, Option<u64>, bool) {
    let fallback = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    match lofty::read_from_path(path) {
        Ok(file) => {
            let duration = Some(file.properties().duration().as_millis() as u64).filter(|d| *d > 0);
            let tag = file.primary_tag().or_else(|| file.first_tag());
            let title = tag
                .and_then(|t| t.title().map(|s| s.into_owned()))
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(fallback);
            let artist = tag.and_then(|t| t.artist().map(|s| s.into_owned()));
            let album = tag.and_then(|t| t.album().map(|s| s.into_owned()));
            let has_art = tag.is_some_and(|t| !t.pictures().is_empty());
            (title, artist, album, duration, has_art)
        }
        Err(_) => (fallback, None, None, None, false),
    }
}

/// Embedded cover, or cover.jpg / folder.jpg next to the file.
pub fn read_artwork(path: &Path) -> Option<Vec<u8>> {
    if let Ok(file) = lofty::read_from_path(path) {
        if let Some(pic) = file
            .primary_tag()
            .or_else(|| file.first_tag())
            .and_then(|t| t.pictures().first())
        {
            return Some(pic.data().to_vec());
        }
    }
    let dir = path.parent()?;
    ["cover.jpg", "cover.png", "folder.jpg", "folder.png"]
        .iter()
        .find_map(|n| std::fs::read(dir.join(n)).ok())
}

/// Incremental scan of every folder: only new or changed files are re-read.
pub fn scan(db: &Db) -> Result<ScanSummary, StorageError> {
    let mut summary = ScanSummary::default();
    for folder in folders(db)? {
        let known: std::collections::HashMap<String, (i64, i64)> = db.call(|c| {
            let mut s = c.prepare(
                "SELECT path, size, mtime FROM local_player_tracks WHERE folder_id = ?1",
            )?;
            let rows = s.query_map([&folder.id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    (r.get::<_, i64>(1)?, r.get::<_, i64>(2)?),
                ))
            })?;
            rows.collect()
        })?;
        let mut seen = std::collections::HashSet::new();
        let mut batch = Vec::new();
        for entry in walkdir::WalkDir::new(&folder.path)
            .follow_links(true)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !entry.file_type().is_file() || !is_audio(path) {
                continue;
            }
            let key = path.to_string_lossy().into_owned();
            let meta = entry.metadata().ok();
            let size = meta.as_ref().map_or(0, |m| m.len() as i64);
            let mtime = meta
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs() as i64);
            seen.insert(key.clone());
            match known.get(&key) {
                Some(&(s, m)) if s == size && m == mtime => {}
                Some(_) => {
                    summary.updated += 1;
                    batch.push((key, size, mtime));
                }
                None => {
                    summary.added += 1;
                    batch.push((key, size, mtime));
                }
            }
        }
        for chunk in batch.chunks(500) {
            let rows: Vec<_> = chunk
                .iter()
                .map(|(p, s, m)| (p.clone(), *s, *m, read_track(Path::new(p))))
                .collect();
            db.call(|c| {
                let tx = c.unchecked_transaction()?;
                for (path, size, mtime, (title, artist, album, duration, art)) in &rows {
                    let normalized = normalize(&format!("{title} {} {}", artist.as_deref().unwrap_or(""), album.as_deref().unwrap_or("")));
                    tx.execute(
                        "INSERT INTO local_player_tracks (id, path, folder_id, title, artist, album, duration_ms, has_artwork, size, mtime, normalized, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)
                         ON CONFLICT(path) DO UPDATE SET title = ?4, artist = ?5, album = ?6, duration_ms = ?7, has_artwork = ?8, size = ?9, mtime = ?10, normalized = ?11, updated_at = ?12",
                        rusqlite::params![new_id(), path, folder.id, title, artist, album, duration.map(|d| d as i64), art, size, mtime, normalized, now_ms() as i64],
                    )?;
                }
                tx.commit()
            })?;
        }
        let gone: Vec<String> = known
            .keys()
            .filter(|k| !seen.contains(*k))
            .cloned()
            .collect();
        summary.removed += gone.len() as u32;
        db.call(|c| {
            for p in &gone {
                c.execute("DELETE FROM local_player_tracks WHERE path = ?1", [p])?;
            }
            c.execute(
                "UPDATE local_player_folders SET last_scan_at = ?1 WHERE id = ?2",
                rusqlite::params![now_ms() as i64, folder.id],
            )?;
            Ok(())
        })?;
    }
    Ok(summary)
}

pub fn query(db: &Db, search: &str, limit: u32, offset: u32) -> Result<Vec<Track>, StorageError> {
    let pattern = format!("%{}%", normalize(search.trim()));
    db.call(|c| {
        let mut s = c.prepare(
            "SELECT id, path, title, artist, album, duration_ms, has_artwork FROM local_player_tracks
             WHERE normalized LIKE ?1 ORDER BY artist, album, track_no, title LIMIT ?2 OFFSET ?3",
        )?;
        let rows = s.query_map(rusqlite::params![pattern, limit, offset], row_to_track)?;
        rows.collect()
    })
}

pub fn get(db: &Db, id: &str) -> Option<Track> {
    db.call(|c| {
        c.query_row(
            "SELECT id, path, title, artist, album, duration_ms, has_artwork FROM local_player_tracks WHERE id = ?1",
            [id],
            row_to_track,
        )
    })
    .ok()
}

fn row_to_track(r: &rusqlite::Row<'_>) -> rusqlite::Result<Track> {
    Ok(Track {
        id: r.get(0)?,
        path: r.get(1)?,
        title: r.get(2)?,
        artist: r.get(3)?,
        album: r.get(4)?,
        duration_ms: r.get::<_, Option<i64>>(5)?.map(|v| v as u64),
        has_artwork: r.get(6)?,
    })
}

/// Tracks for loose files (dropped / opened): stored without a folder.
pub fn track_for_file(db: &Db, path: &Path) -> Result<Track, StorageError> {
    let key = path.to_string_lossy().into_owned();
    let (title, artist, album, duration, art) = read_track(path);
    let normalized = normalize(&title);
    db.call(|c| {
        c.execute(
            "INSERT INTO local_player_tracks (id, path, folder_id, title, artist, album, duration_ms, has_artwork, size, mtime, normalized, created_at, updated_at)
             VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7, 0, 0, ?8, ?9, ?9)
             ON CONFLICT(path) DO UPDATE SET title = ?3, artist = ?4, album = ?5, duration_ms = ?6, has_artwork = ?7",
            rusqlite::params![new_id(), key, title, artist, album, duration.map(|d| d as i64), art, normalized, now_ms() as i64],
        )?;
        c.query_row(
            "SELECT id, path, title, artist, album, duration_ms, has_artwork FROM local_player_tracks WHERE path = ?1",
            [&key],
            row_to_track,
        )
    })
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub count: u32,
}

pub fn playlists(db: &Db) -> Result<Vec<Playlist>, StorageError> {
    db.call(|c| {
        let mut s = c.prepare(
            "SELECT p.id, p.name, COUNT(i.track_id) FROM local_player_playlists p
             LEFT JOIN local_player_playlist_items i ON i.playlist_id = p.id GROUP BY p.id ORDER BY p.name",
        )?;
        let rows = s.query_map([], |r| Ok(Playlist { id: r.get(0)?, name: r.get(1)?, count: r.get::<_, i64>(2)? as u32 }))?;
        rows.collect()
    })
}

pub fn create_playlist(db: &Db, name: &str) -> Result<Playlist, StorageError> {
    let p = Playlist {
        id: new_id(),
        name: name.trim().chars().take(100).collect(),
        count: 0,
    };
    let now = now_ms() as i64;
    db.call(|c| c.execute("INSERT INTO local_player_playlists (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)", rusqlite::params![p.id, p.name, now]))?;
    Ok(p)
}

pub fn delete_playlist(db: &Db, id: &str) -> Result<(), StorageError> {
    db.call(|c| {
        c.execute("DELETE FROM local_player_playlists WHERE id = ?1", [id])
            .map(|_| ())
    })
}

pub fn add_to_playlist(db: &Db, playlist: &str, tracks: &[String]) -> Result<(), StorageError> {
    db.call(|c| {
        let start: i64 = c.query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM local_player_playlist_items WHERE playlist_id = ?1",
            [playlist],
            |r| r.get(0),
        )?;
        for (k, t) in tracks.iter().enumerate() {
            c.execute(
                "INSERT INTO local_player_playlist_items (playlist_id, position, track_id) VALUES (?1, ?2, ?3)",
                rusqlite::params![playlist, start + k as i64, t],
            )?;
        }
        Ok(())
    })
}

pub fn playlist_tracks(db: &Db, playlist: &str) -> Result<Vec<String>, StorageError> {
    db.call(|c| {
        let mut s = c.prepare("SELECT track_id FROM local_player_playlist_items WHERE playlist_id = ?1 ORDER BY position")?;
        let rows = s.query_map([playlist], |r| r.get(0))?;
        rows.collect()
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Station {
    pub id: String,
    pub name: String,
    pub url: String,
}

const STATIONS_KEY: &str = "local_player.stations";

pub fn stations(db: &Db) -> Vec<Station> {
    db.kv_get(STATIONS_KEY)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_stations(db: &Db, list: &[Station]) -> Result<(), StorageError> {
    db.kv_set(
        STATIONS_KEY,
        &serde_json::to_string(list).unwrap_or_else(|_| "[]".into()),
    )
}

/// Writes an extended M3U playlist with absolute paths.
pub fn export_m3u(db: &Db, playlist: &str, dest: &Path) -> Result<u32, StorageError> {
    let mut out = String::from("#EXTM3U\n");
    let ids = playlist_tracks(db, playlist)?;
    for t in ids.iter().filter_map(|id| get(db, id)) {
        let secs = t.duration_ms.map_or(-1, |d| (d / 1000) as i64);
        let label = match &t.artist {
            Some(a) => format!("{a} - {}", t.title),
            None => t.title.clone(),
        };
        out.push_str(&format!("#EXTINF:{secs},{label}\n{}\n", t.path));
    }
    std::fs::write(dest, out)?;
    Ok(ids.len() as u32)
}

/// Parses M3U/M3U8 lines into file paths (relative entries resolve against the
/// playlist's folder). Comments and URLs are skipped.
pub fn parse_m3u(text: &str, base: &Path) -> Vec<std::path::PathBuf> {
    text.lines()
        .map(|l| l.trim().trim_start_matches('\u{feff}'))
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.contains("://"))
        .map(|l| {
            let p = Path::new(l);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                base.join(p)
            }
        })
        .collect()
}

/// Imports an M3U file as a new playlist named after the file.
pub fn import_m3u(db: &Db, src: &Path) -> Result<Playlist, StorageError> {
    let text = std::fs::read_to_string(src)?;
    let base = src.parent().unwrap_or(Path::new("."));
    let ids: Vec<String> = parse_m3u(&text, base)
        .iter()
        .filter(|p| p.is_file() && is_audio(p))
        .filter_map(|p| track_for_file(db, p).ok().map(|t| t.id))
        .collect();
    let name = src
        .file_stem()
        .map_or_else(|| "Playlist".into(), |s| s.to_string_lossy().into_owned());
    let pl = create_playlist(db, &name)?;
    add_to_playlist(db, &pl.id, &ids)?;
    Ok(Playlist {
        count: ids.len() as u32,
        ..pl
    })
}

#[cfg(test)]
mod m3u_tests {
    use super::*;

    #[test]
    fn parses_relative_absolute_and_skips_comments_and_urls() {
        let text = "\u{feff}#EXTM3U\n#EXTINF:120,A - B\nsongs/a.mp3\nC:\\Music\\b.flac\nhttp://radio/x\n\n";
        let got = parse_m3u(text, Path::new("D:\\Lists"));
        assert_eq!(
            got,
            vec![
                Path::new("D:\\Lists").join("songs/a.mp3"),
                std::path::PathBuf::from("C:\\Music\\b.flac")
            ]
        );
    }
}
