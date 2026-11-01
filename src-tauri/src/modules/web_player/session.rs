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
            })
        })
        .take(100)
        .collect()
}

#[cfg(test)]
mod tests {
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
