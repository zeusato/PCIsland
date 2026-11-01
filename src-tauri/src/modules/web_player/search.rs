//! Isolated, silent search view. Uses the site's own search and authenticated profile.
use std::sync::{Mutex, PoisonError};
use tauri::{AppHandle, Url, Webview, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::oneshot;

use super::{WebPlayerHandle, dto::WebQueueItem, session};
use crate::ipc::{IpcError, IpcResult};

struct Pending {
    id: String,
    site_id: String,
    url: String,
    reply: Option<oneshot::Sender<Vec<WebQueueItem>>>,
}

#[derive(Default)]
pub struct SearchState {
    pending: Mutex<Option<Pending>>,
    results: Mutex<Option<(String, Vec<WebQueueItem>)>>,
}

fn target(site: &str, query: &str) -> IpcResult<Url> {
    let query = query.trim();
    if query.is_empty() || query.chars().count() > 160 {
        return Err(IpcError::new(
            "web_player.search_query",
            "query must contain 1–160 characters",
        ));
    }
    let mut url = Url::parse(match site {
        "zing_mp3" => "https://zingmp3.vn/tim-kiem/bai-hat",
        // Live QA encountered SoundCloud's anti-bot block. Do not automate its
        // search until an approved integration has been validated.
        "spotify" => "https://open.spotify.com/search/",
        _ => {
            return Err(IpcError::new(
                "web_player.search_unsupported",
                "search not available for this site",
            ));
        }
    })
    .expect("fixed search URL");
    if site == "spotify" {
        url.path_segments_mut()
            .expect("https URL")
            .pop_if_empty()
            .push(query)
            .push("tracks");
    } else {
        url.query_pairs_mut().append_pair("q", query);
    }
    Ok(url)
}

pub async fn run(
    app: &AppHandle,
    handle: &WebPlayerHandle,
    site_id: &str,
    query: &str,
) -> IpcResult<Vec<WebQueueItem>> {
    let url = target(site_id, query)?;
    let id = crate::core::id::new_id();
    let label = format!("web-search-{id}");
    let (reply, receive) = oneshot::channel();
    {
        let mut pending = handle
            .search
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if pending.is_some() {
            return Err(IpcError::new(
                "web_player.search_busy",
                "search in progress",
            ));
        }
        *pending = Some(Pending {
            id: id.clone(),
            site_id: site_id.into(),
            url: url.to_string(),
            reply: Some(reply),
        });
    }
    let script = format!(
        "{}\ndocument.addEventListener('DOMContentLoaded',()=>window.__pcislandSearch({}));",
        include_str!("search.js"),
        serde_json::to_string(&id).expect("request ID")
    );
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
        .title("PCisland Search")
        .visible(false)
        .inner_size(1100.0, 760.0)
        .data_directory(super::web_data_dir())
        .additional_browser_args(super::BROWSER_ARGS)
        .initialization_script(script)
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .build();
    let result = match window {
        Ok(window) => {
            let result = tokio::time::timeout(std::time::Duration::from_secs(22), receive).await;
            let _ = window.destroy();
            match result {
                Ok(Ok(items)) if !items.is_empty() => Ok(items),
                _ => Err(IpcError::new(
                    "web_player.search_empty",
                    "no results received; sign in or open the site to check",
                )),
            }
        }
        Err(error) => Err(error.into()),
    };
    *handle
        .search
        .pending
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = None;
    if let Ok(items) = &result {
        *handle
            .search
            .results
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some((site_id.into(), items.clone()));
    }
    result
}

pub fn report(
    handle: &WebPlayerHandle,
    webview: &Webview,
    id: &str,
    items: Vec<WebQueueItem>,
) -> IpcResult<()> {
    let mut pending = handle
        .search
        .pending
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let Some(request) = pending.as_ref() else {
        return Ok(());
    };
    if request.id != id
        || webview.label() != format!("web-search-{id}")
        || webview.url()?.as_str() != request.url
    {
        return Err(IpcError::new(
            "web_player.search_sender",
            "unexpected search sender",
        ));
    }
    let items = session::queue(items, &request.site_id);
    if let Some(reply) = pending.as_mut().and_then(|request| request.reply.take()) {
        let _ = reply.send(items);
    }
    Ok(())
}

pub fn play(app: &AppHandle, handle: &WebPlayerHandle, site_id: &str, url: &str) -> IpcResult<()> {
    let cached = handle
        .search
        .results
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let item = cached
        .as_ref()
        .filter(|(site, _)| site == site_id)
        .and_then(|(_, items)| items.iter().find(|item| item.url == url))
        .cloned()
        .ok_or_else(|| {
            IpcError::new(
                "web_player.search_result",
                "result no longer available; search again",
            )
        })?;
    drop(cached);
    let saved = super::dto::WebSession {
        site_id: site_id.into(),
        page_url: item.url.clone(),
        track_url: Some(item.url.clone()),
        title: item.title,
        position_ms: 0,
        queue: vec![],
    };
    *handle
        .restoring_title
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(saved.title.clone());
    let generation = handle
        .resume_generation
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        + 1;
    super::open_target(
        app,
        handle,
        Some(site_id),
        Some(&item.url),
        false,
        Some(&saved),
    )?;
    super::watch_resume(app, handle, generation);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn search_query_is_data_not_url_syntax() {
        let query = "Bằng Kiều / ? & #";
        let zing = super::target("zing_mp3", query).unwrap();
        assert_eq!(
            zing.query_pairs().find(|(key, _)| key == "q").unwrap().1,
            query
        );
        let spotify = super::target("spotify", query).unwrap();
        assert!(spotify.path().contains("%2F"));
        assert!(spotify.query().is_none() && spotify.fragment().is_none());
        assert!(super::target("youtube", query).is_err());
        assert!(super::target("soundcloud", query).is_err());
        assert!(super::target("spotify", " ").is_err());
    }
}
