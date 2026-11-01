//! yt-dlp JSON → what the player plays (docs/modules/video-player.md §5.3). Pure; tested
//! with trimmed real output. Order (spike S2): HLS → video+audio pair → single stream.

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct Info {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub uploader: Option<String>,
    #[serde(default)]
    pub duration: Option<f64>,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub is_live: Option<bool>,
    #[serde(default)]
    pub formats: Vec<Format>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Format {
    #[serde(default)]
    pub format_id: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub manifest_url: Option<String>,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub ext: Option<String>,
    #[serde(default)]
    pub vcodec: Option<String>,
    #[serde(default)]
    pub acodec: Option<String>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub tbr: Option<f64>,
    #[serde(default)]
    pub has_drm: Option<serde_json::Value>,
    #[serde(default)]
    pub http_headers: Option<std::collections::BTreeMap<String, String>>,
}

pub type Headers = Vec<(String, String)>;

#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    pub url: String,
    pub headers: Headers,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Plan {
    /// Master playlist; hls.js picks the level (capped in the page).
    Hls { manifest: Stream },
    /// Separate video and audio files, synced in the page.
    Pair {
        video: Stream,
        audio: Stream,
        height: u32,
    },
    /// One file with both tracks.
    Single { stream: Stream },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub title: String,
    pub uploader: Option<String>,
    pub duration_s: Option<f64>,
    pub thumbnail: Option<String>,
    pub is_live: bool,
    pub plan: Plan,
    /// Heights the quality menu offers (descending).
    pub qualities: Vec<u32>,
}

fn none(codec: Option<&str>) -> bool {
    codec.is_none_or(|c| c == "none" || c.is_empty())
}

fn video_rank(codec: &str) -> Option<u8> {
    // Hardware decoding is most common for H.264, then VP9, then AV1.
    if codec.starts_with("avc1") {
        Some(3)
    } else if codec.starts_with("vp09") || codec.starts_with("vp9") {
        Some(2)
    } else if codec.starts_with("av01") {
        Some(1)
    } else {
        None
    }
}

fn audio_ok(codec: &str) -> bool {
    codec.starts_with("mp4a") || codec.starts_with("opus")
}

fn usable(f: &Format) -> bool {
    let drm = f
        .has_drm
        .as_ref()
        .is_some_and(|v| v.as_bool().unwrap_or(true));
    !drm && !f.format_id.ends_with("-drc")
}

fn stream(f: &Format, url: &str) -> Option<Stream> {
    let parsed = tauri::Url::parse(url).ok()?;
    matches!(parsed.scheme(), "https" | "http").then(|| Stream {
        url: url.to_owned(),
        headers: f
            .http_headers
            .as_ref()
            .map(|h| h.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default(),
    })
}

fn is_hls(f: &Format) -> bool {
    f.protocol.as_deref().is_some_and(|p| p.starts_with("m3u8"))
}

fn is_plain(f: &Format) -> bool {
    matches!(f.protocol.as_deref(), Some("https" | "http") | None)
}

/// Picks the plan. `want` is a height the user chose; otherwise the best ≤ `max_height`.
pub fn plan(info: &Info, max_height: u32, want: Option<u32>) -> Option<Resolved> {
    let formats: Vec<&Format> = info.formats.iter().filter(|f| usable(f)).collect();
    let cap = |h: Option<u32>| h.is_none_or(|h| h <= max_height);
    let heights_of = |list: &mut Vec<u32>, h: Option<u32>| {
        if let Some(h) = h.filter(|h| *h <= max_height) {
            if !list.contains(&h) {
                list.push(h);
            }
        }
    };

    // Video-only candidates and the best audio per container family.
    let videos: Vec<&&Format> = formats
        .iter()
        .filter(|f| is_plain(f) && none(f.acodec.as_deref()) && f.url.is_some())
        .filter(|f| f.vcodec.as_deref().and_then(video_rank).is_some())
        .collect();
    let best_audio = |mp4: bool| {
        formats
            .iter()
            .filter(|f| is_plain(f) && none(f.vcodec.as_deref()) && f.url.is_some())
            .filter(|f| f.acodec.as_deref().is_some_and(audio_ok))
            .filter(|f| (f.ext.as_deref() == Some("m4a") || f.ext.as_deref() == Some("mp4")) == mp4)
            .max_by(|a, b| a.tbr.unwrap_or(0.0).total_cmp(&b.tbr.unwrap_or(0.0)))
            .copied()
    };

    let mut qualities = Vec::new();
    let manifest = formats
        .iter()
        .filter(|f| is_hls(f))
        .find_map(|f| f.manifest_url.as_deref().map(|m| (*f, m)));
    let chosen = if let Some((f, url)) = manifest.filter(|_| want.is_none()) {
        for f in formats
            .iter()
            .filter(|f| is_hls(f) && !none(f.vcodec.as_deref()))
        {
            heights_of(&mut qualities, f.height);
        }
        stream(f, url).map(|manifest| Plan::Hls { manifest })
    } else {
        None
    };
    let chosen = chosen.or_else(|| {
        for v in &videos {
            heights_of(&mut qualities, v.height);
        }
        let mut pairable: Vec<&&&Format> = videos
            .iter()
            .filter(|v| cap(v.height) && want.is_none_or(|w| v.height == Some(w)))
            .collect();
        pairable.sort_by(|a, b| {
            let key = |f: &Format| {
                (
                    f.height.unwrap_or(0),
                    f.vcodec.as_deref().and_then(video_rank).unwrap_or(0),
                )
            };
            key(b).cmp(&key(a))
        });
        pairable.into_iter().find_map(|v| {
            let mp4 = v.ext.as_deref() == Some("mp4");
            let a = best_audio(mp4)?;
            Some(Plan::Pair {
                video: stream(v, v.url.as_deref()?)?,
                audio: stream(a, a.url.as_deref()?)?,
                height: v.height.unwrap_or(0),
            })
        })
    });
    let chosen = chosen.or_else(|| {
        let single = formats
            .iter()
            .filter(|f| is_plain(f) && f.url.is_some())
            .filter(|f| !none(f.vcodec.as_deref()) && !none(f.acodec.as_deref()))
            .filter(|f| cap(f.height))
            .max_by_key(|f| f.height.unwrap_or(0))?;
        heights_of(&mut qualities, single.height);
        Some(Plan::Single {
            stream: stream(single, single.url.as_deref()?)?,
        })
    })?;
    qualities.sort_unstable_by(|a, b| b.cmp(a));
    Some(Resolved {
        title: info
            .title
            .clone()
            .unwrap_or_default()
            .chars()
            .take(512)
            .collect(),
        uploader: info.uploader.clone().map(|u| u.chars().take(256).collect()),
        duration_s: info.duration.filter(|d| d.is_finite() && *d > 0.0),
        thumbnail: info.thumbnail.clone(),
        is_live: info.is_live.unwrap_or(false),
        plan: chosen,
        qualities,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Trimmed from real `yt-dlp --dump-single-json` output (spike S2, 2026-11-01).
    const YOUTUBE: &str = r#"{
      "title": "Big Buck Bunny 60fps 4K", "uploader": "Blender", "duration": 635, "is_live": false,
      "thumbnail": "https://i.ytimg.com/vi/x/maxresdefault.jpg",
      "formats": [
        {"format_id":"140-drc","protocol":"https","ext":"m4a","vcodec":"none","acodec":"mp4a.40.2","tbr":129,"url":"https://g.googlevideo.com/a140drc"},
        {"format_id":"140","protocol":"https","ext":"m4a","vcodec":"none","acodec":"mp4a.40.2","tbr":129,"url":"https://g.googlevideo.com/a140","http_headers":{"User-Agent":"UA"}},
        {"format_id":"251","protocol":"https","ext":"webm","vcodec":"none","acodec":"opus","tbr":129,"url":"https://g.googlevideo.com/a251"},
        {"format_id":"299","protocol":"https","ext":"mp4","vcodec":"avc1.64002A","acodec":"none","height":1080,"tbr":3248,"url":"https://g.googlevideo.com/v299","http_headers":{"User-Agent":"UA"}},
        {"format_id":"303","protocol":"https","ext":"webm","vcodec":"vp9","acodec":"none","height":1080,"tbr":2127,"url":"https://g.googlevideo.com/v303"},
        {"format_id":"308","protocol":"https","ext":"webm","vcodec":"vp9","acodec":"none","height":1440,"tbr":5968,"url":"https://g.googlevideo.com/v308"},
        {"format_id":"298","protocol":"https","ext":"mp4","vcodec":"avc1.4D4020","acodec":"none","height":720,"tbr":1898,"url":"https://g.googlevideo.com/v298"},
        {"format_id":"312","protocol":"m3u8_native","ext":"mp4","vcodec":"avc1.64002A","acodec":"none","height":1080,"url":"https://m.googlevideo.com/312.m3u8","manifest_url":"https://manifest.googlevideo.com/master.m3u8"},
        {"format_id":"628","protocol":"m3u8_native","ext":"mp4","vcodec":"vp09.00.51.08","acodec":"none","height":2160,"url":"https://m.googlevideo.com/628.m3u8","manifest_url":"https://manifest.googlevideo.com/master.m3u8"},
        {"format_id":"sb0","protocol":"mhtml","ext":"mhtml","vcodec":"none","acodec":"none","url":"https://i.ytimg.com/sb"}
      ]}"#;

    fn info(json: &str) -> Info {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn youtube_prefers_hls_and_caps_qualities() {
        let r = plan(&info(YOUTUBE), 1080, None).unwrap();
        assert!(matches!(&r.plan, Plan::Hls { manifest } if manifest.url.ends_with("master.m3u8")));
        assert_eq!(r.qualities, vec![1080]);
        assert_eq!(r.title, "Big Buck Bunny 60fps 4K");
        assert_eq!(r.duration_s, Some(635.0));
    }

    #[test]
    fn a_chosen_height_uses_the_pair_with_h264_and_aac() {
        let r = plan(&info(YOUTUBE), 1440, Some(1080)).unwrap();
        match r.plan {
            Plan::Pair {
                video,
                audio,
                height,
            } => {
                assert_eq!(height, 1080);
                assert!(
                    video.url.ends_with("v299"),
                    "avc1 preferred over vp9 at the same height"
                );
                assert!(audio.url.ends_with("a140"), "-drc variants are skipped");
                assert_eq!(video.headers, vec![("User-Agent".into(), "UA".into())]);
            }
            other => panic!("expected pair, got {other:?}"),
        }
        assert_eq!(r.qualities, vec![1440, 1080, 720]);
    }

    #[test]
    fn webm_video_pairs_with_webm_audio() {
        let r = plan(&info(YOUTUBE), 1440, Some(1440)).unwrap();
        assert!(
            matches!(r.plan, Plan::Pair { ref audio, height: 1440, .. } if audio.url.ends_with("a251"))
        );
    }

    #[test]
    fn facebook_style_progressive_and_drm() {
        let fb = r#"{"title":"Clip","formats":[
          {"format_id":"sd","protocol":"https","ext":"mp4","vcodec":"h264","acodec":"aac","height":360,"url":"https://video.fbcdn.net/sd.mp4"},
          {"format_id":"hd","protocol":"https","ext":"mp4","vcodec":"h264","acodec":"aac","height":720,"url":"https://video.fbcdn.net/hd.mp4"}]}"#;
        let r = plan(&info(fb), 1080, None).unwrap();
        assert!(matches!(&r.plan, Plan::Single { stream } if stream.url.ends_with("hd.mp4")));
        assert_eq!(r.qualities, vec![720]);
        let drm = r#"{"formats":[{"format_id":"x","protocol":"https","vcodec":"avc1","acodec":"mp4a","url":"https://a/b","has_drm":true}]}"#;
        assert!(plan(&info(drm), 1080, None).is_none());
    }

    #[test]
    fn rejects_non_http_urls() {
        let bad = r#"{"formats":[{"format_id":"x","protocol":"https","vcodec":"avc1","acodec":"mp4a","url":"file:///C:/x.mp4"}]}"#;
        assert!(plan(&info(bad), 1080, None).is_none());
    }
}
