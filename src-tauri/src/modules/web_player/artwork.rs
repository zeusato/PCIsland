//! Downloads remote covers into the app's artwork scheme without relaxing the UI CSP.

use std::sync::Arc;
use std::time::Duration;

use crate::core::artwork::ArtworkCache;
use crate::modules::media::dto::NowPlayingDto;

const MAX_BYTES: usize = 2 * 1024 * 1024;

/// Keeps the latest metadata while a cover is loading. A generation rejects late
/// results after changing tracks, navigating, closing, or reopening the player.
#[derive(Default)]
pub struct PendingArtwork {
    generation: u64,
    remote: Option<String>,
    local: Option<String>,
    latest: Option<NowPlayingDto>,
}

impl PendingArtwork {
    pub fn update(&mut self, mut next: Option<NowPlayingDto>) -> Option<(u64, String)> {
        let remote = next.as_ref().and_then(|n| n.artwork_url.clone());
        let request = if remote != self.remote {
            self.generation += 1;
            self.remote = remote.clone();
            self.local = None;
            remote.map(|url| (self.generation, url))
        } else {
            None
        };
        if let Some(np) = &mut next {
            np.artwork_url = self.local.clone();
        }
        self.latest = next;
        request
    }

    pub fn snapshot(&self) -> Option<NowPlayingDto> {
        self.latest.clone()
    }

    pub fn complete(&mut self, generation: u64, local: String) -> Option<NowPlayingDto> {
        if generation != self.generation {
            return None;
        }
        let np = self.latest.as_mut()?;
        np.artwork_url = Some(local.clone());
        self.local = Some(local);
        Some(np.clone())
    }
}

pub fn remote_url(value: &str) -> Option<String> {
    let url = tauri::Url::parse(value).ok()?;
    (value.len() <= 8192
        && url.scheme() == "https"
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none())
    .then(|| url.to_string())
}

fn append(bytes: &mut Vec<u8>, chunk: &[u8]) -> Result<(), &'static str> {
    if chunk.len() > MAX_BYTES.saturating_sub(bytes.len()) {
        return Err("cover exceeds 2 MB");
    }
    bytes.extend_from_slice(chunk);
    Ok(())
}

pub async fn download(cache: &ArtworkCache, url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .https_only(true)
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .map_err(|_| "cannot create artwork client")?;
    let mut response = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|_| "artwork request failed")?;
    if response
        .content_length()
        .is_some_and(|n| n > MAX_BYTES as u64)
    {
        return Err("cover exceeds 2 MB".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "cannot read artwork")? {
        append(&mut bytes, &chunk)?;
    }
    if !crate::core::artwork::sniff_mime(&bytes).starts_with("image/") {
        return Err("unsupported artwork content".into());
    }
    Ok(cache.put(Arc::new(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::media::dto::{MediaCaps, PlaybackStatus};

    fn track(title: &str, url: &str) -> NowPlayingDto {
        NowPlayingDto {
            source_id: "web".into(),
            app_name: "Test".into(),
            title: title.into(),
            artist: None,
            album: None,
            artwork_url: Some(url.into()),
            status: PlaybackStatus::Playing,
            position_ms: Some(0),
            position_at: 1,
            duration_ms: Some(1000),
            rate: 1.0,
            caps: MediaCaps::default(),
            is_ad: false,
        }
    }

    #[test]
    fn remote_cover_is_never_published_and_download_is_deduplicated() {
        let mut state = PendingArtwork::default();
        let np = track("A", "https://cdn.example/a.jpg");
        let (generation, _) = state.update(Some(np.clone())).expect("request");
        assert!(state.snapshot().expect("metadata").artwork_url.is_none());
        assert!(state.update(Some(np.clone())).is_none());
        state.complete(generation, "http://artwork.localhost/cover".into());
        state.update(Some(np));
        assert_eq!(
            state.snapshot().expect("metadata").artwork_url.as_deref(),
            Some("http://artwork.localhost/cover")
        );
    }

    #[test]
    fn late_cover_cannot_replace_new_track_or_resurrect_closed_player() {
        let mut state = PendingArtwork::default();
        let (a, _) = state
            .update(Some(track("A", "https://cdn.example/a")))
            .expect("A");
        let (b, _) = state
            .update(Some(track("B", "https://cdn.example/b")))
            .expect("B");
        assert!(state.complete(a, "old".into()).is_none());
        let mut paused = track("B", "https://cdn.example/b");
        paused.status = PlaybackStatus::Paused;
        paused.position_ms = Some(500);
        state.update(Some(paused));
        let ready = state.complete(b, "new".into()).expect("B ready");
        assert_eq!(ready.status, PlaybackStatus::Paused);
        assert_eq!(ready.position_ms, Some(500));
        state.update(None);
        assert!(state.complete(b, "late".into()).is_none());
        assert!(state.snapshot().is_none());
    }

    #[test]
    fn validates_urls_without_truncating_signed_queries_and_bounds_streams() {
        let signed = format!("https://cdn.example/cover?signature={}", "x".repeat(600));
        assert_eq!(remote_url(&signed), Some(signed));
        for bad in [
            "http://cdn.example/a",
            "file:///a",
            "https://user:pass@cdn.example/a",
        ] {
            assert!(remote_url(bad).is_none());
        }
        let mut bytes = vec![0; MAX_BYTES - 1];
        assert!(append(&mut bytes, &[1]).is_ok());
        assert!(append(&mut bytes, &[2]).is_err());
        assert_eq!(bytes.len(), MAX_BYTES);
    }
}
