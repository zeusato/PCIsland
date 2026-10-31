//! Update check against GitHub Releases (F45, docs/modules/app-lifecycle.md §5).
//! Notifies only; never downloads or installs.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use ts_rs::TS;

use crate::core::activity::{Activity, ActivityAction, ActivityHub, payload};
use crate::core::priority;
use crate::core::settings::Settings;
use crate::core::storage::Db;
use crate::ipc::now_ms;

const RELEASES_API: &str = "https://api.github.com/repos/zeusato/PCIsland/releases/latest";
const RELEASES_PAGE_PREFIX: &str = "https://github.com/zeusato/PCIsland/";
const LAST_CHECK_KEY: &str = "app.last_update_check";
const SKIPPED_KEY: &str = "app.skipped_version";
const DAY_MS: u64 = 86_400_000;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// `a > b` for dotted numeric versions ("v1.2.10" > "1.2.9"); suffixes are ignored.
pub fn is_newer(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.trim_start_matches('v')
            .split(['.', '-', '+'])
            .take(3)
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    parse(a) > parse(b)
}

pub async fn check(force: bool, db: &Db) -> Option<UpdateInfo> {
    let now = now_ms();
    let last: u64 = db
        .kv_get(LAST_CHECK_KEY)
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if !force && now.saturating_sub(last) < DAY_MS {
        return None;
    }
    let _ = db.kv_set(LAST_CHECK_KEY, &now.to_string());
    let client = reqwest::Client::builder()
        .user_agent(concat!("PCisland/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;
    let release: Release = client
        .get(RELEASES_API)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;
    let version = release.tag_name.trim_start_matches('v').to_owned();
    let skipped = db.kv_get(SKIPPED_KEY);
    let valid_url = release.html_url.starts_with(RELEASES_PAGE_PREFIX);
    (!release.draft
        && !release.prerelease
        && valid_url
        && is_newer(&version, env!("CARGO_PKG_VERSION"))
        && (force || skipped.as_deref() != Some(version.as_str())))
    .then_some(UpdateInfo {
        version,
        url: release.html_url,
    })
}

pub fn spawn(db: Db, hub: ActivityHub, settings: watch::Receiver<Settings>) {
    let for_actions = db.clone();
    let hub_for_actions = hub.clone();
    let latest: Arc<std::sync::Mutex<Option<UpdateInfo>>> = Arc::default();
    let latest_for_actions = latest.clone();
    hub.register_handler(
        "app",
        Arc::new(move |_, action| {
            let info = latest_for_actions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone();
            let Some(info) = info else {
                return Err("no update".into());
            };
            match action {
                "open" => open_url(&info.url),
                "skip" => {
                    let _ = for_actions.kv_set(SKIPPED_KEY, &info.version);
                }
                other => return Err(format!("unknown app action `{other}`")),
            }
            hub_for_actions.dismiss("app:update");
            Ok(())
        }),
    );
    tauri::async_runtime::spawn(async move {
        // First check a minute after start, then hourly (rate-limited to once a day).
        tokio::time::sleep(Duration::from_secs(60)).await;
        loop {
            if settings.borrow().app.check_updates {
                if let Some(info) = check(false, &db).await {
                    tracing::info!(version = %info.version, "update available");
                    *latest
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(info.clone());
                    hub.alert(
                        Activity::new(
                            "app",
                            "app:update",
                            "app.update",
                            priority::ALERT,
                            payload(&info),
                        )
                        .transient(8_000)
                        .with_actions(vec![
                            ActivityAction::new("open", "download", "app.update.open").primary(),
                            ActivityAction::new("skip", "x", "app.update.skip"),
                        ]),
                    );
                }
            }
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    });
}

/// Opens a GitHub URL in the default browser.
pub fn open_url(url: &str) {
    if url.starts_with(RELEASES_PAGE_PREFIX) {
        let _ = std::process::Command::new("explorer").arg(url).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn version_comparison() {
        assert!(is_newer("0.1.0", "0.0.1"));
        assert!(is_newer("v1.2.10", "1.2.9"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("0.9.9", "1.0.0"));
        assert!(is_newer("1.0.1-beta", "1.0.0"));
    }
}
