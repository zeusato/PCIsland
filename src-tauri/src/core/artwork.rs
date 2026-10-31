//! In-memory LRU of artwork bytes served through the `artwork` URI scheme
//! (docs/02-system-design.md §11). Only hashes cross IPC, never bytes.

use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, Mutex};

use tauri::http::{Request, Response, StatusCode, header};

const CAPACITY: usize = 32;

#[derive(Default)]
pub struct ArtworkCache {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    map: HashMap<String, Arc<Vec<u8>>>,
    order: VecDeque<String>,
}

impl ArtworkCache {
    /// Stores `bytes` and returns the URL the frontend should use.
    pub fn put(&self, bytes: Arc<Vec<u8>>) -> String {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let key = format!("{:016x}", hasher.finish());
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !inner.map.contains_key(&key) {
            inner.map.insert(key.clone(), bytes);
            inner.order.push_back(key.clone());
            while inner.order.len() > CAPACITY {
                if let Some(old) = inner.order.pop_front() {
                    inner.map.remove(&old);
                }
            }
        }
        url(&key)
    }

    pub fn get(&self, key: &str) -> Option<Arc<Vec<u8>>> {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        inner.map.get(key).cloned()
    }

    /// Handler for the `artwork` URI scheme.
    pub fn respond(&self, request: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
        let key = request.uri().path().trim_start_matches('/');
        match self.get(key) {
            Some(bytes) => Response::builder()
                .header(header::CONTENT_TYPE, sniff_mime(&bytes))
                .header(header::CACHE_CONTROL, "max-age=31536000, immutable")
                .body(Cow::Owned(bytes.as_ref().clone()))
                .unwrap_or_else(|_| not_found()),
            None => not_found(),
        }
    }
}

fn not_found() -> Response<Cow<'static, [u8]>> {
    let mut res = Response::new(Cow::Borrowed(&[][..]));
    *res.status_mut() = StatusCode::NOT_FOUND;
    res
}

pub fn url(key: &str) -> String {
    format!("http://artwork.localhost/{key}")
}

pub(crate) fn sniff_mime(bytes: &[u8]) -> &'static str {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => "image/png",
        [0xFF, 0xD8, 0xFF, ..] => "image/jpeg",
        [b'G', b'I', b'F', ..] => "image/gif",
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => "image/webp",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_bytes_give_same_url_and_lru_evicts_oldest() {
        let cache = ArtworkCache::default();
        let first = cache.put(Arc::new(vec![0]));
        assert_eq!(first, cache.put(Arc::new(vec![0])));
        for i in 1..=CAPACITY as u8 {
            cache.put(Arc::new(vec![i]));
        }
        let key = first.rsplit('/').next().unwrap_or_default();
        assert!(cache.get(key).is_none());
    }

    #[test]
    fn sniffs_png_and_jpeg() {
        assert_eq!(sniff_mime(&[0x89, b'P', b'N', b'G']), "image/png");
        assert_eq!(sniff_mime(&[0xFF, 0xD8, 0xFF, 0xE0]), "image/jpeg");
    }
}
