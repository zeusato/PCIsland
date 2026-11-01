//! Where each video was left, so reopening it continues there (docs/modules/
//! video-player.md §3 step 10). Stored in `kv` as one bounded map.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::link::LinkKind;
use crate::core::storage::Db;
use crate::ipc::now_ms;

const KV: &str = "video.progress";
const MAX_ENTRIES: usize = 200;
/// Shorter than this is "just started": begin from the start.
const MIN_RESUME_MS: u64 = 5_000;
/// This close to the end counts as watched.
const END_MARGIN_MS: u64 = 15_000;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub pos_ms: u64,
    pub dur_ms: Option<u64>,
    pub at: u64,
}

/// One key per video, independent of tracking parameters or `t=`.
pub fn key(link: &LinkKind) -> String {
    match link {
        LinkKind::Youtube { id, .. } => format!("yt:{id}"),
        LinkKind::Facebook { url } | LinkKind::Direct { url, .. } | LinkKind::Page { url } => {
            url.split('#').next().unwrap_or(url).to_owned()
        }
    }
}

/// Position to continue from, or `None` to start over (barely started or finished).
pub fn resume_at(e: &Entry) -> Option<u64> {
    if e.pos_ms < MIN_RESUME_MS {
        return None;
    }
    if e.dur_ms.is_some_and(|d| e.pos_ms + END_MARGIN_MS >= d) {
        return None;
    }
    Some(e.pos_ms)
}

/// Applies one report to the map: finished videos are forgotten, the oldest entries go
/// first when the map is full.
pub fn apply(map: &mut HashMap<String, Entry>, key: &str, entry: Entry, ended: bool) {
    if ended || resume_at(&entry).is_none() && entry.pos_ms >= MIN_RESUME_MS {
        map.remove(key);
        return;
    }
    map.insert(key.to_owned(), entry);
    while map.len() > MAX_ENTRIES {
        let Some(oldest) = map.iter().min_by_key(|(_, e)| e.at).map(|(k, _)| k.clone()) else {
            break;
        };
        map.remove(&oldest);
    }
}

fn load_map(db: &Db) -> HashMap<String, Entry> {
    db.kv_get(KV)
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default()
}

pub fn load(db: &Db, key: &str) -> Option<u64> {
    load_map(db).get(key).and_then(resume_at)
}

pub fn save(db: &Db, key: &str, pos_ms: u64, dur_ms: Option<u64>, ended: bool) {
    let mut map = load_map(db);
    apply(
        &mut map,
        key,
        Entry {
            pos_ms,
            dur_ms,
            at: now_ms(),
        },
        ended,
    );
    if let Ok(json) = serde_json::to_string(&map) {
        let _ = db.kv_set(KV, &json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(pos: u64, dur: Option<u64>, at: u64) -> Entry {
        Entry {
            pos_ms: pos,
            dur_ms: dur,
            at,
        }
    }

    #[test]
    fn keys_ignore_start_time_and_fragments() {
        let a = LinkKind::Youtube {
            id: "aqz-KE-bpKQ".into(),
            start_s: Some(30),
        };
        let b = LinkKind::Youtube {
            id: "aqz-KE-bpKQ".into(),
            start_s: None,
        };
        assert_eq!(key(&a), key(&b));
        let p = LinkKind::Page {
            url: "https://e.com/v#top".into(),
        };
        assert_eq!(key(&p), "https://e.com/v");
    }

    #[test]
    fn resumes_only_in_the_middle() {
        assert_eq!(resume_at(&e(197_000, Some(635_000), 1)), Some(197_000));
        assert_eq!(
            resume_at(&e(3_000, Some(635_000), 1)),
            None,
            "barely started"
        );
        assert_eq!(resume_at(&e(625_000, Some(635_000), 1)), None, "finished");
        assert_eq!(
            resume_at(&e(60_000, None, 1)),
            Some(60_000),
            "unknown length"
        );
    }

    #[test]
    fn finished_videos_are_forgotten_and_the_map_is_bounded() {
        let mut map = HashMap::new();
        apply(&mut map, "a", e(100_000, Some(600_000), 1), false);
        assert!(map.contains_key("a"));
        apply(&mut map, "a", e(599_000, Some(600_000), 2), false);
        assert!(!map.contains_key("a"), "watched to the end");
        apply(&mut map, "b", e(100_000, None, 3), false);
        apply(&mut map, "b", e(120_000, None, 4), true);
        assert!(!map.contains_key("b"), "ended event");
        for i in 0..(MAX_ENTRIES as u64 + 5) {
            apply(&mut map, &format!("k{i}"), e(60_000, None, 10 + i), false);
        }
        assert_eq!(map.len(), MAX_ENTRIES);
        assert!(!map.contains_key("k0") && map.contains_key(&format!("k{}", MAX_ENTRIES + 4)));
    }
}
