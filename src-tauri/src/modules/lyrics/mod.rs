//! Synced lyrics from LRCLIB with a local cache (docs/modules/lyrics-visualizer.md §A).
#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::watch;
use ts_rs::TS;

use crate::core::settings::Settings;
use crate::core::storage::Db;
use crate::ipc::{events, now_ms};
use crate::modules::media::dto::NowPlayingDto;
use crate::shared::lrc::{self, LyricLine};
use crate::shared::nlp_vi::normalize;

const CACHE_MISS_TTL_MS: u64 = 7 * 86_400_000;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct LyricsDto {
    pub track_key: String,
    pub synced: bool,
    pub lines: Vec<LyricLine>,
    pub plain: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LrclibResponse {
    synced_lyrics: Option<String>,
    plain_lyrics: Option<String>,
}

#[derive(Clone, Default)]
pub struct LyricsHandle(Arc<Mutex<Option<LyricsDto>>>);

/// Strips "(Official MV)", "[Lyrics]", "- Topic" and similar noise from web titles.
pub fn clean_title(title: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in title.chars() {
        match c {
            '(' | '[' | '【' => depth += 1,
            ')' | ']' | '】' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    let out = out.split(" | ").next().unwrap_or(&out).to_owned();
    let lower = out.to_lowercase();
    let cut = ["official", "lyrics", "mv", "m/v", "audio"]
        .iter()
        .filter_map(|w| lower.find(&format!(" {w}")))
        .min();
    cut.map_or(out.clone(), |i| out[..i].to_owned())
        .trim()
        .trim_end_matches(['-', '|'])
        .trim()
        .to_owned()
}

fn track_key(np: &NowPlayingDto) -> String {
    format!(
        "{}|{}|{}",
        normalize(&clean_title(&np.title)),
        normalize(np.artist.as_deref().unwrap_or("")),
        np.duration_ms.map_or(0, |d| d / 1000)
    )
}

pub fn setup(
    app: &AppHandle,
    db: Db,
    mut now_playing: watch::Receiver<Option<NowPlayingDto>>,
    settings: watch::Receiver<Settings>,
) -> LyricsHandle {
    let handle = LyricsHandle::default();
    let current = handle.0.clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "PCisland/",
                env!("CARGO_PKG_VERSION"),
                " (https://github.com/)"
            ))
            .timeout(Duration::from_secs(5))
            .build()
            .ok();
        let mut last_key = String::new();
        while now_playing.changed().await.is_ok() {
            let np = now_playing.borrow_and_update().clone();
            let s = settings.borrow().lyrics.clone();
            let Some(np) = np.filter(|_| s.enabled) else {
                if !last_key.is_empty() {
                    last_key.clear();
                    set(&app, &current, None);
                }
                continue;
            };
            let key = track_key(&np);
            if key == last_key {
                continue;
            }
            last_key = key.clone();
            set(&app, &current, None);
            let dto = match cached(&db, &key) {
                Some(hit) => hit,
                None if s.online_enabled => {
                    let fetched = match &client {
                        Some(c) => fetch(c, &np).await,
                        None => None,
                    };
                    store(&db, &key, fetched.as_ref());
                    fetched.map(|(synced, text)| to_dto(&key, synced, &text))
                }
                None => None,
            };
            if last_key == key {
                set(&app, &current, dto);
            }
        }
    });
    handle
}

fn set(app: &AppHandle, current: &Mutex<Option<LyricsDto>>, dto: Option<LyricsDto>) {
    *current.lock().unwrap_or_else(PoisonError::into_inner) = dto.clone();
    for label in ["island", "main"] {
        let _ = app.emit_to(label, events::LYRICS_CURRENT, &dto);
    }
}

fn to_dto(key: &str, synced: bool, text: &str) -> LyricsDto {
    LyricsDto {
        track_key: key.into(),
        synced,
        lines: if synced { lrc::parse(text) } else { Vec::new() },
        plain: (!synced).then(|| text.to_owned()),
    }
}

/// `Some(dto)` for a hit, `Some(None)`-like miss is represented by returning `None`
/// only when nothing is cached; a cached "no lyrics" yields an empty plain dto.
fn cached(db: &Db, key: &str) -> Option<Option<LyricsDto>> {
    db.call(|c| {
        c.query_row(
            "SELECT source, synced, content, fetched_at FROM lyrics_cache WHERE track_key = ?1",
            [key],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, bool>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, i64>(3)? as u64,
                ))
            },
        )
    })
    .ok()
    .and_then(|(source, synced, content, at)| {
        if source == "none" {
            (now_ms().saturating_sub(at) < CACHE_MISS_TTL_MS).then_some(None)
        } else {
            content.map(|t| Some(to_dto(key, synced, &t)))
        }
    })
}

fn store(db: &Db, key: &str, value: Option<&(bool, String)>) {
    let (source, synced, content) = match value {
        Some((s, t)) => ("lrclib", *s, Some(t.clone())),
        None => ("none", false, None),
    };
    let _ = db.call(|c| {
        c.execute(
            "INSERT OR REPLACE INTO lyrics_cache (track_key, source, synced, content, offset_ms, fetched_at) VALUES (?1, ?2, ?3, ?4, 0, ?5)",
            rusqlite::params![key, source, synced, content, now_ms() as i64],
        )
    });
}

async fn fetch(client: &reqwest::Client, np: &NowPlayingDto) -> Option<(bool, String)> {
    let title = clean_title(&np.title);
    let mut query = vec![("track_name", title.clone())];
    if let Some(artist) = &np.artist {
        query.push(("artist_name", artist.clone()));
    }
    if let Some(d) = np.duration_ms {
        query.push(("duration", (d / 1000).to_string()));
    }
    let resp = client
        .get("https://lrclib.net/api/get")
        .query(&query)
        .send()
        .await
        .ok()?;
    let body: LrclibResponse = if resp.status().is_success() {
        resp.json().await.ok()?
    } else {
        // Fall back to search and accept a result within 2 s of the duration.
        let q = format!("{} {}", title, np.artist.as_deref().unwrap_or(""));
        let list: Vec<serde_json::Value> = client
            .get("https://lrclib.net/api/search")
            .query(&[("q", q.trim())])
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()?;
        let pick = list.into_iter().find(|v| {
            let d = v
                .get("duration")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            np.duration_ms
                .is_none_or(|ms| (d - ms as f64 / 1000.0).abs() <= 2.0)
        })?;
        serde_json::from_value(pick).ok()?
    };
    match (body.synced_lyrics, body.plain_lyrics) {
        (Some(s), _) if !s.trim().is_empty() => Some((true, s)),
        (_, Some(p)) if !p.trim().is_empty() => Some((false, p)),
        _ => None,
    }
}

#[tauri::command]
pub fn lyrics_get_current(lyrics: State<'_, LyricsHandle>) -> Option<LyricsDto> {
    lyrics
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

#[cfg(test)]
mod tests {
    use super::clean_title;

    #[test]
    fn cleans_web_titles() {
        assert_eq!(
            clean_title("Chúng Ta Của Hiện Tại (Official MV)"),
            "Chúng Ta Của Hiện Tại"
        );
        assert_eq!(clean_title("Song Name [Lyrics]"), "Song Name");
        assert_eq!(clean_title("Artist - Song Official Audio"), "Artist - Song");
        assert_eq!(clean_title("Plain"), "Plain");
    }
}
