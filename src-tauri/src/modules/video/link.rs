//! Classifies a pasted link (docs/modules/video-player.md §5.2). Pure; no I/O.

use serde::Serialize;
use tauri::Url;
use ts_rs::TS;

const MAX_LEN: usize = 2048;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum LinkKind {
    /// Resolved with yt-dlp; `start_s` comes from `t=`.
    Youtube {
        id: String,
        #[serde(rename = "startS", skip_serializing_if = "Option::is_none")]
        #[ts(rename = "startS", optional)]
        start_s: Option<u32>,
    },
    Facebook {
        url: String,
    },
    /// A media file or HLS playlist the player opens directly (through the proxy).
    Direct {
        url: String,
        hls: bool,
    },
    /// Any other web page: streams are sniffed from it.
    Page {
        url: String,
    },
}

impl LinkKind {
    /// The page URL handed to yt-dlp or the sniffer.
    pub fn source_url(&self) -> String {
        match self {
            Self::Youtube { id, .. } => format!("https://www.youtube.com/watch?v={id}"),
            Self::Facebook { url } | Self::Direct { url, .. } | Self::Page { url } => url.clone(),
        }
    }

    pub fn start_s(&self) -> Option<u32> {
        match self {
            Self::Youtube { start_s, .. } => *start_s,
            _ => None,
        }
    }
}

fn youtube_id(value: &str) -> Option<String> {
    (value.len() == 11
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
    .then(|| value.to_owned())
}

/// `90`, `90s`, `1m30s`, `1h2m3s` → seconds.
pub fn parse_start(value: &str) -> Option<u32> {
    if let Ok(n) = value.parse::<u32>() {
        return Some(n);
    }
    let mut total = 0u32;
    let mut number = String::new();
    for c in value.chars() {
        if c.is_ascii_digit() {
            number.push(c);
            continue;
        }
        let n: u32 = number.parse().ok()?;
        number.clear();
        total = total.checked_add(match c {
            'h' => n.checked_mul(3600)?,
            'm' => n.checked_mul(60)?,
            's' => n,
            _ => return None,
        })?;
    }
    number.is_empty().then_some(total)
}

fn is_host(host: &str, domain: &str) -> bool {
    host == domain || host.ends_with(&format!(".{domain}"))
}

pub fn classify(raw: &str) -> Option<LinkKind> {
    let raw = raw.trim();
    if raw.is_empty() || raw.len() > MAX_LEN {
        return None;
    }
    let url = Url::parse(raw).ok()?;
    if !matches!(url.scheme(), "https" | "http")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    let path = url.path().to_owned();
    let query = |key: &str| {
        url.query_pairs()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.into_owned())
    };
    let start_s = query("t")
        .or_else(|| query("start"))
        .and_then(|t| parse_start(&t));

    if is_host(&host, "youtube.com") || is_host(&host, "youtube-nocookie.com") {
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let id = match segments.as_slice() {
            ["watch"] => query("v"),
            ["shorts" | "embed" | "live" | "v", id, ..] => Some((*id).to_owned()),
            _ => None,
        }?;
        return Some(LinkKind::Youtube {
            id: youtube_id(&id)?,
            start_s,
        });
    }
    if host == "youtu.be" {
        let id = path.trim_start_matches('/').split('/').next()?;
        return Some(LinkKind::Youtube {
            id: youtube_id(id)?,
            start_s,
        });
    }
    if url.scheme() != "https" {
        // Plain http is only accepted for direct media files on a LAN/NAS.
        let lower = path.to_ascii_lowercase();
        return direct(&lower).map(|hls| LinkKind::Direct {
            url: url.to_string(),
            hls,
        });
    }
    if is_host(&host, "facebook.com") || host == "fb.watch" {
        let lower = path.to_ascii_lowercase();
        let video = host == "fb.watch"
            || lower.contains("/videos/")
            || lower.starts_with("/reel/")
            || lower.starts_with("/watch")
            || lower.starts_with("/share/v/")
            || lower.starts_with("/share/r/");
        return video.then(|| LinkKind::Facebook {
            url: url.to_string(),
        });
    }
    let lower = path.to_ascii_lowercase();
    Some(match direct(&lower) {
        Some(hls) => LinkKind::Direct {
            url: url.to_string(),
            hls,
        },
        None => LinkKind::Page {
            url: url.to_string(),
        },
    })
}

/// `Some(true)` for HLS, `Some(false)` for a media file, `None` otherwise.
fn direct(lower_path: &str) -> Option<bool> {
    if lower_path.ends_with(".m3u8") {
        return Some(true);
    }
    [
        ".mp4", ".webm", ".m4v", ".mov", ".mkv", ".mp3", ".m4a", ".ogg", ".oga", ".wav", ".aac",
        ".flac",
    ]
    .iter()
    .any(|ext| lower_path.ends_with(ext))
    .then_some(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yt(id: &str, start: Option<u32>) -> Option<LinkKind> {
        Some(LinkKind::Youtube {
            id: id.into(),
            start_s: start,
        })
    }

    #[test]
    fn recognises_every_youtube_form() {
        let id = "aqz-KE-bpKQ";
        assert_eq!(
            classify(&format!("https://www.youtube.com/watch?v={id}")),
            yt(id, None)
        );
        assert_eq!(
            classify(&format!("https://youtube.com/watch?v={id}&t=90")),
            yt(id, Some(90))
        );
        assert_eq!(
            classify(&format!("https://youtu.be/{id}?t=1m30s")),
            yt(id, Some(90))
        );
        assert_eq!(
            classify(&format!("https://m.youtube.com/shorts/{id}")),
            yt(id, None)
        );
        assert_eq!(
            classify(&format!("https://www.youtube.com/embed/{id}?start=5")),
            yt(id, Some(5))
        );
        assert_eq!(
            classify(&format!("https://www.youtube.com/live/{id}")),
            yt(id, None)
        );
        assert_eq!(
            classify(&format!("  https://music.youtube.com/watch?v={id}  ")),
            yt(id, None)
        );
    }

    #[test]
    fn rejects_bad_youtube_ids_and_unsafe_urls() {
        assert_eq!(classify("https://www.youtube.com/watch?v=short"), None);
        assert_eq!(
            classify("https://www.youtube.com/watch?v=aqz-KE-bpK!"),
            None
        );
        assert_eq!(classify("https://www.youtube.com/feed/trending"), None);
        assert_eq!(classify("javascript:alert(1)"), None);
        assert_eq!(classify("file:///C:/a.mp4"), None);
        assert_eq!(classify("https://user:pw@example.com/a.mp4"), None);
        assert_eq!(classify("http://example.com/page"), None);
        assert_eq!(
            classify(&format!("https://e.com/{}", "a".repeat(3000))),
            None
        );
    }

    #[test]
    fn recognises_facebook_videos_only() {
        for url in [
            "https://www.facebook.com/someone/videos/123456/",
            "https://fb.watch/abcDEF/",
            "https://www.facebook.com/reel/987654",
            "https://www.facebook.com/watch/?v=42",
            "https://www.facebook.com/share/v/1ab2/",
        ] {
            assert!(
                matches!(classify(url), Some(LinkKind::Facebook { .. })),
                "{url}"
            );
        }
        assert_eq!(classify("https://www.facebook.com/someone"), None);
    }

    #[test]
    fn direct_files_and_pages() {
        assert_eq!(
            classify("https://cdn.example/v/clip.MP4?sig=1"),
            Some(LinkKind::Direct {
                url: "https://cdn.example/v/clip.MP4?sig=1".into(),
                hls: false
            })
        );
        assert!(matches!(
            classify("https://e.com/live/index.m3u8"),
            Some(LinkKind::Direct { hls: true, .. })
        ));
        assert!(matches!(
            classify("http://192.168.1.5/movie.mkv"),
            Some(LinkKind::Direct { hls: false, .. })
        ));
        assert!(matches!(
            classify("https://news.example/article/1"),
            Some(LinkKind::Page { .. })
        ));
    }

    #[test]
    fn parses_start_times() {
        assert_eq!(parse_start("90"), Some(90));
        assert_eq!(parse_start("90s"), Some(90));
        assert_eq!(parse_start("1m30s"), Some(90));
        assert_eq!(parse_start("1h2m3s"), Some(3723));
        assert_eq!(parse_start("1x"), None);
        assert_eq!(parse_start("1m30"), None);
    }
}
