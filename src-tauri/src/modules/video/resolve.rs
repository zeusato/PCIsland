//! yt-dlp JSON → what the player plays (docs/modules/video-player.md §5.3). Pure; tested
//! with trimmed real output. Order (spike S2): HLS → video+audio pair → single stream.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

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
    pub language: Option<String>,
    pub language_preference: Option<i32>,
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
        languages: BTreeMap<String, Stream>,
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
    let matches_height = |h: Option<u32>| match want {
        Some(height) => h == Some(height),
        None => h.is_none_or(|h| h <= max_height),
    };

    // Video-only candidates and the best audio per container family.
    let videos: Vec<&&Format> = formats
        .iter()
        .filter(|f| is_plain(f) && none(f.acodec.as_deref()) && f.url.is_some())
        .filter(|f| f.vcodec.as_deref().and_then(video_rank).is_some())
        .collect();
    let mut audio: Vec<_> = formats
        .iter()
        .copied()
        .filter(|f| {
            is_plain(f) && none(f.vcodec.as_deref()) && f.acodec.as_deref().is_some_and(audio_ok)
        })
        .filter_map(|f| Some((f, stream(f, f.url.as_deref()?)?)))
        .collect();
    let mp4_audio = |f: &Format| matches!(f.ext.as_deref(), Some("m4a" | "mp4"));
    let language = |f: &Format| {
        f.language
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase()
    };
    let pair_languages: BTreeSet<_> = audio
        .iter()
        .map(|(f, _)| language(f))
        .filter(|tag| !tag.is_empty())
        .collect();

    let manifest = formats
        .iter()
        .filter(|f| is_hls(f))
        .find_map(|f| f.manifest_url.as_deref().map(|m| (*f, m)));
    let in_manifest =
        |f: &Format| is_hls(f) && f.manifest_url.as_deref() == manifest.map(|(_, url)| url);
    let hls_heights: BTreeSet<_> = formats
        .iter()
        .filter(|f| in_manifest(f) && !none(f.vcodec.as_deref()))
        .filter_map(|f| f.height)
        .collect();
    let hls = || {
        let (f, url) = manifest.filter(|_| want.is_none_or(|h| hls_heights.contains(&h)))?;
        stream(f, url).map(|manifest| Plan::Hls { manifest })
    };
    // A master can contain only its default dub even when separate files expose more.
    let manifest_languages: BTreeSet<_> = formats
        .iter()
        .filter(|f| in_manifest(f))
        .map(|f| language(f))
        .collect();
    let prefer_pair = pair_languages.len() > 1
        && !pair_languages.is_subset(&manifest_languages)
        && !info.is_live.unwrap_or(false);
    let chosen = if prefer_pair || want.is_some() {
        None
    } else {
        hls()
    };
    let chosen = chosen
        .or_else(|| {
            let mut pairable: Vec<_> = videos.iter().filter(|v| matches_height(v.height)).collect();
            pairable.sort_by_key(|v| {
                std::cmp::Reverse((v.height, v.vcodec.as_deref().and_then(video_rank)))
            });
            pairable.into_iter().find_map(|v| {
                let mp4 = v.ext.as_deref() == Some("mp4");
                audio.sort_by(|(a, _), (b, _)| {
                    let rank =
                        |f: &Format| (f.language_preference.unwrap_or(0), mp4_audio(f) == mp4);
                    rank(a)
                        .cmp(&rank(b))
                        .then_with(|| a.tbr.unwrap_or(0.0).total_cmp(&b.tbr.unwrap_or(0.0)))
                });
                let (_, default) = audio.last()?;
                // Separate elements decode independently. Prefer a matching container on ties.
                let languages = audio
                    .iter()
                    .map(|(f, s)| (language(f), s.clone()))
                    .collect();
                Some(Plan::Pair {
                    video: stream(v, v.url.as_deref()?)?,
                    audio: default.clone(),
                    languages,
                    height: v.height.unwrap_or(0),
                })
            })
        })
        .or_else(hls);
    let mut qualities = hls_heights;
    if !audio.is_empty() {
        qualities.extend(videos.iter().filter_map(|v| v.height));
    }
    let chosen = chosen.or_else(|| {
        let single = formats
            .iter()
            .filter(|f| is_plain(f) && f.url.is_some())
            .filter(|f| !none(f.vcodec.as_deref()) && !none(f.acodec.as_deref()))
            .filter(|f| matches_height(f.height))
            .max_by_key(|f| f.height.unwrap_or(0))?;
        qualities.extend(single.height);
        Some(Plan::Single {
            stream: stream(single, single.url.as_deref()?)?,
        })
    })?;
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
        qualities: qualities.into_iter().rev().collect(),
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
    fn youtube_auto_prefers_hls_and_offers_all_hls_and_pair_heights() {
        let r = plan(&info(YOUTUBE), 1080, None).unwrap();
        assert!(matches!(&r.plan, Plan::Hls { manifest } if manifest.url.ends_with("master.m3u8")));
        assert_eq!(r.qualities, vec![2160, 1440, 1080, 720]);
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
                ..
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
        assert_eq!(r.qualities, vec![2160, 1440, 1080, 720]);
    }

    #[test]
    fn webm_video_pairs_with_webm_audio() {
        let r = plan(&info(YOUTUBE), 1080, Some(1440)).unwrap();
        assert!(
            matches!(r.plan, Plan::Pair { ref audio, height: 1440, .. } if audio.url.ends_with("a251"))
        );
    }

    #[test]
    fn manual_4k_uses_hls_when_no_4k_pair_exists() {
        let r = plan(&info(YOUTUBE), 1080, Some(2160)).unwrap();
        assert!(matches!(r.plan, Plan::Hls { .. }));
        assert!(plan(&info(YOUTUBE), 1080, Some(4320)).is_none());
    }

    #[test]
    fn dubbed_video_exposes_each_playable_language_at_both_qualities() {
        let mut info = info(YOUTUBE);
        let mut dub = info.formats[1].clone();
        info.formats[1].language = Some("en".into());
        info.formats[1].language_preference = Some(10);
        info.formats[2].language = Some("en".into());
        dub.language = Some("vi-VN".into());
        dub.tbr = Some(48.0);
        dub.url = Some("https://audio.test/vi".into());
        info.formats.push(dub);
        for height in [None, Some(720)] {
            let resolved = plan(&info, 1080, height).unwrap();
            let Plan::Pair {
                audio, languages, ..
            } = resolved.plan
            else {
                panic!("expected both language streams");
            };
            assert_eq!(audio, languages["en"]);
            assert_eq!(languages.len(), 2);
            assert_eq!(languages["vi-vn"].url, "https://audio.test/vi");
        }
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
