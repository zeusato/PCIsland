use super::{dto::WebQueueItem, sites};
use tauri::Url;
pub const KEY: &str = "web_player.sessions";

pub fn site_url(value: &str, site_id: &str) -> Option<String> {
    if value.len() > 8192 {
        return None;
    }
    let url = Url::parse(value).ok()?;
    (url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && sites::for_host(url.host_str()?).is_some_and(|site| site.id == site_id))
    .then(|| url.to_string())
}

pub fn queue(items: Vec<WebQueueItem>, site_id: &str) -> Vec<WebQueueItem> {
    let mut seen = std::collections::HashSet::new();
    items
        .into_iter()
        .take(200)
        .filter_map(|item| {
            let url = site_url(&item.url, site_id)?;
            if !seen.insert(url.clone()) {
                return None;
            }
            Some(WebQueueItem {
                url,
                title: item.title.chars().take(512).collect(),
                artist: clean_credit(item.artist),
                uploader: clean_credit(item.uploader),
            })
        })
        .take(100)
        .collect()
}

fn clean_credit(value: Option<String>) -> Option<String> {
    value
        .map(|v| {
            v.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(256)
                .collect::<String>()
        })
        .filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    #[test]
    fn old_queues_load_and_search_credits_are_bounded() {
        let old: super::WebQueueItem =
            serde_json::from_str(r#"{"url":"https://zingmp3.vn/bai-hat/a/123.html","title":"A"}"#)
                .unwrap();
        assert!(old.artist.is_none() && old.uploader.is_none());
        let mut item = old;
        item.artist = Some("  Ca sĩ\nA  ".into());
        item.uploader = Some("x".repeat(500));
        let clean = super::queue(vec![item], "zing_mp3");
        assert_eq!(clean[0].artist.as_deref(), Some("Ca sĩ A"));
        assert_eq!(clean[0].uploader.as_ref().unwrap().len(), 256);
    }
    #[test]
    fn resume_and_queue_urls_cannot_escape_the_selected_site() {
        assert!(super::site_url("https://zingmp3.vn/bai-hat/a/123.html", "zing_mp3").is_some());
        for url in [
            "javascript:alert(1)",
            "https://evil.example/",
            "https://user:pass@zingmp3.vn/",
            "http://zingmp3.vn/",
        ] {
            assert!(super::site_url(url, "zing_mp3").is_none());
        }
    }
}
