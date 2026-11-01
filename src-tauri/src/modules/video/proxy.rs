//! `vstream://` — forwards the player's media requests to the resolved stream with the
//! headers yt-dlp asked for (docs/modules/video-player.md §5.4).
//!
//! - `http://vstream.localhost/<token>`            → the session's main URL
//! - `http://vstream.localhost/<token>/u?=<url>`   → a URL listed in a playlist of that
//!   session (HLS variants/segments). Only hosts seen in the session are forwarded, so
//!   this is never an open proxy.
//!
//! WebView custom protocols answer with a full body, so media ranges are capped
//! (`CHUNK`): the element asks again for the next range.

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasher, RandomState};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use tauri::Url;
use tauri::http::{Request, Response, StatusCode, header};

use super::resolve::Headers;

const CHUNK: u64 = 4 * 1024 * 1024;
const MAX_BODY: usize = 48 * 1024 * 1024;
const MAX_PLAYLIST: usize = 2 * 1024 * 1024;
const BASE: &str = "http://vstream.localhost";

struct Session {
    url: String,
    headers: Headers,
    hosts: HashSet<String>,
}

pub struct Proxy {
    sessions: Mutex<HashMap<String, Session>>,
    client: reqwest::Client,
}

impl Default for Proxy {
    fn default() -> Self {
        Self {
            sessions: Mutex::default(),
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(60))
                .redirect(reqwest::redirect::Policy::limited(5))
                .build()
                .unwrap_or_default(),
        }
    }
}

fn token() -> String {
    let n = crate::ipc::now_ms();
    format!(
        "{:016x}{:016x}",
        RandomState::new().hash_one(n),
        RandomState::new().hash_one((n, "vstream"))
    )
}

/// `bytes=a-b` / `bytes=a-` → the capped range to ask upstream for.
pub fn capped_range(value: Option<&str>) -> String {
    let parsed = value
        .and_then(|v| v.trim().strip_prefix("bytes="))
        .and_then(|v| v.split_once('-'))
        .and_then(|(a, b)| {
            let start: u64 = a.trim().parse().ok()?;
            let end = b.trim().parse::<u64>().ok();
            Some((start, end))
        });
    let (start, end) = parsed.unwrap_or((0, None));
    let last = start + CHUNK - 1;
    let end = end.map_or(last, |e| e.min(last)).max(start);
    format!("bytes={start}-{end}")
}

pub fn is_playlist(content_type: &str, url: &str) -> bool {
    // YouTube segment URLs contain "index.m3u8" mid-path (spike S2): only trust the
    // content type or a path that really ends in .m3u8.
    content_type.to_ascii_lowercase().contains("mpegurl")
        || Url::parse(url).is_ok_and(|u| u.path().to_ascii_lowercase().ends_with(".m3u8"))
}

/// Rewrites every URI in a playlist to go through this session; returns the hosts seen.
pub fn rewrite_playlist(text: &str, base: &str, token: &str) -> (String, Vec<String>) {
    let mut hosts = Vec::new();
    let mut fix = |raw: &str| -> String {
        match Url::parse(base).and_then(|b| b.join(raw.trim())) {
            Ok(u) if matches!(u.scheme(), "https" | "http") => {
                if let Some(h) = u.host_str() {
                    hosts.push(h.to_ascii_lowercase());
                }
                let mut out = Url::parse(&format!("{BASE}/{token}/u")).expect("fixed base");
                out.query_pairs_mut().append_pair("", u.as_str());
                out.to_string()
            }
            _ => raw.to_owned(),
        }
    };
    let body = text
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                line.to_owned()
            } else if trimmed.starts_with('#') {
                // Attributes such as URI="..." on #EXT-X-MEDIA / #EXT-X-MAP / #EXT-X-KEY.
                let mut out = String::new();
                let mut rest = line;
                while let Some(i) = rest.find("URI=\"") {
                    let (head, tail) = rest.split_at(i + 5);
                    out.push_str(head);
                    let Some(close) = tail.find('"') else {
                        rest = tail;
                        break;
                    };
                    out.push_str(&fix(&tail[..close]));
                    rest = &tail[close..];
                }
                out.push_str(rest);
                out
            } else {
                fix(trimmed)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    (body, hosts)
}

fn reply(status: StatusCode, content_type: &str, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::CACHE_CONTROL, "no-store")
        .body(body)
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

impl Proxy {
    /// Registers a stream; returns the URL the player loads.
    pub fn register(&self, url: &str, headers: &Headers) -> String {
        let token = token();
        let host = Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_ascii_lowercase));
        self.sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                token.clone(),
                Session {
                    url: url.to_owned(),
                    headers: headers.clone(),
                    hosts: host.into_iter().collect(),
                },
            );
        format!("{BASE}/{token}")
    }

    /// Forgets every session (the video window closed or a new video started).
    pub fn clear(&self) {
        self.sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    fn target(&self, request: &Request<Vec<u8>>) -> Option<(String, String, Headers)> {
        let uri = request.uri();
        let mut parts = uri.path().trim_start_matches('/').splitn(2, '/');
        let token = parts.next()?.to_owned();
        let sessions = self.sessions.lock().unwrap_or_else(PoisonError::into_inner);
        let session = sessions.get(&token)?;
        let url = match parts.next() {
            None | Some("") => session.url.clone(),
            Some("u") => {
                let query = uri.query()?;
                let full = Url::parse(&format!("{BASE}/?{query}")).ok()?;
                let target = full
                    .query_pairs()
                    .find(|(k, _)| k.is_empty())?
                    .1
                    .into_owned();
                let parsed = Url::parse(&target).ok()?;
                let host = parsed.host_str()?.to_ascii_lowercase();
                if !matches!(parsed.scheme(), "https" | "http") || !session.hosts.contains(&host) {
                    return None;
                }
                target
            }
            Some(_) => return None,
        };
        Some((token, url, session.headers.clone()))
    }

    fn allow_hosts(&self, token: &str, hosts: Vec<String>) {
        if let Some(s) = self
            .sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_mut(token)
        {
            s.hosts.extend(hosts);
        }
    }

    pub async fn handle(&self, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
        let Some((token, url, headers)) = self.target(&request) else {
            return reply(StatusCode::NOT_FOUND, "text/plain", Vec::new());
        };
        let range = request
            .headers()
            .get(header::RANGE)
            .and_then(|v| v.to_str().ok());
        let mut upstream = self.client.get(&url);
        for (k, v) in &headers {
            upstream = upstream.header(k.as_str(), v.as_str());
        }
        if range.is_some() {
            upstream = upstream.header(header::RANGE, capped_range(range));
        }
        let resp = match upstream.send().await {
            Ok(r) => r,
            Err(error) => {
                tracing::debug!(module = "video", %error, "stream request failed");
                return reply(StatusCode::BAD_GATEWAY, "text/plain", Vec::new());
            }
        };
        let status = resp.status();
        let content_type = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_owned();
        let final_url = resp.url().to_string();
        if !status.is_success() {
            // 403/410 = the signed URL expired; the page asks for a fresh one.
            return reply(
                StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
                "text/plain",
                Vec::new(),
            );
        }
        if is_playlist(&content_type, &final_url) {
            let Ok(text) = read_limited(resp, MAX_PLAYLIST).await else {
                return reply(StatusCode::BAD_GATEWAY, "text/plain", Vec::new());
            };
            let (body, hosts) =
                rewrite_playlist(&String::from_utf8_lossy(&text), &final_url, &token);
            self.allow_hosts(&token, hosts);
            return reply(
                StatusCode::OK,
                "application/vnd.apple.mpegurl",
                body.into_bytes(),
            );
        }
        let content_range = resp
            .headers()
            .get(header::CONTENT_RANGE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let Ok(body) = read_limited(resp, MAX_BODY).await else {
            return reply(StatusCode::BAD_GATEWAY, "text/plain", Vec::new());
        };
        let mut builder = Response::builder()
            .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
            .header(header::CONTENT_TYPE, content_type)
            .header(header::ACCEPT_RANGES, "bytes")
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .header(header::CONTENT_LENGTH, body.len());
        if let Some(cr) = content_range {
            builder = builder.header(header::CONTENT_RANGE, cr);
        }
        builder
            .body(body)
            .unwrap_or_else(|_| Response::new(Vec::new()))
    }
}

async fn read_limited(mut resp: reqwest::Response, max: usize) -> Result<Vec<u8>, ()> {
    let mut out = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|_| ())? {
        if out.len() + chunk.len() > max {
            return Err(());
        }
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_are_capped_to_one_chunk() {
        assert_eq!(
            capped_range(Some("bytes=0-")),
            format!("bytes=0-{}", CHUNK - 1)
        );
        assert_eq!(capped_range(Some("bytes=100-199")), "bytes=100-199");
        assert_eq!(
            capped_range(Some("bytes=10-999999999")),
            format!("bytes=10-{}", 10 + CHUNK - 1)
        );
        assert_eq!(capped_range(None), format!("bytes=0-{}", CHUNK - 1));
    }

    #[test]
    fn playlist_detection_ignores_m3u8_in_the_middle_of_a_path() {
        let seg = "https://rr1.googlevideo.com/videoplayback/playlist/index.m3u8/file/seg.ts";
        assert!(!is_playlist("video/mp2t", seg));
        assert!(is_playlist("application/vnd.apple.mpegurl", seg));
        assert!(is_playlist("", "https://cdn.example/live/master.m3u8?x=1"));
    }

    #[test]
    fn rewrites_segments_and_uri_attributes_and_collects_hosts() {
        let text = "#EXTM3U\n#EXT-X-MEDIA:TYPE=AUDIO,URI=\"a/audio.m3u8\",NAME=\"x\"\n#EXT-X-MAP:URI=\"https://cdn2.example/init.mp4\"\nseg1.ts\nhttps://cdn3.example/seg2.ts\n";
        let (out, hosts) = rewrite_playlist(text, "https://cdn1.example/hls/master.m3u8", "tok");
        assert!(out.starts_with("#EXTM3U\n"));
        assert!(out.contains("URI=\"http://vstream.localhost/tok/u?=https%3A%2F%2Fcdn1.example%2Fhls%2Fa%2Faudio.m3u8\""));
        assert!(out.contains(
            "http://vstream.localhost/tok/u?=https%3A%2F%2Fcdn1.example%2Fhls%2Fseg1.ts"
        ));
        assert!(out.contains("https%3A%2F%2Fcdn3.example%2Fseg2.ts"));
        assert_eq!(
            hosts,
            vec![
                "cdn1.example",
                "cdn2.example",
                "cdn1.example",
                "cdn3.example"
            ]
        );
    }

    #[test]
    fn only_known_hosts_are_forwarded() {
        let proxy = Proxy::default();
        let url = proxy.register("https://cdn1.example/master.m3u8", &vec![]);
        let token = url.rsplit('/').next().unwrap().to_owned();
        let req = |path: &str| {
            Request::builder()
                .uri(format!("{BASE}{path}"))
                .body(Vec::new())
                .unwrap()
        };
        assert!(proxy.target(&req(&format!("/{token}"))).is_some());
        let evil = format!("/{token}/u?=https%3A%2F%2Fevil.example%2Fx");
        assert!(proxy.target(&req(&evil)).is_none());
        proxy.allow_hosts(&token, vec!["evil.example".into()]);
        assert!(proxy.target(&req(&evil)).is_some());
        assert!(proxy.target(&req("/unknown")).is_none());
        proxy.clear();
        assert!(proxy.target(&req(&format!("/{token}"))).is_none());
    }
}
