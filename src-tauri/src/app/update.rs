//! Update check against GitHub Releases (F45, docs/modules/app-lifecycle.md §5).
//! Downloads verified packages and installs them after the app exits.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use ts_rs::TS;

use super::update_package::{self, Asset, Package};
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
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Clone, Default)]
pub struct Updater {
    check_lock: Arc<tokio::sync::Mutex<()>>,
    pending: Arc<std::sync::Mutex<Option<(UpdateInfo, Package)>>>,
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

impl Updater {
    pub async fn check(&self, force: bool, db: &Db) -> Result<Option<UpdateInfo>, String> {
        let _guard = self.check_lock.lock().await;
        if let Some((info, _)) = &*self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
        {
            return Ok(Some(info.clone()));
        }
        let now = now_ms();
        let last: u64 = db
            .kv_get(LAST_CHECK_KEY)
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        if !force && now.saturating_sub(last) < DAY_MS {
            return Ok(None);
        }
        let client = reqwest::Client::builder()
            .user_agent(concat!("PCisland/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(180))
            .build()
            .map_err(|e| e.to_string())?;
        let release: Release = client
            .get(RELEASES_API)
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        let version = release.tag_name.trim_start_matches('v').to_owned();
        let skipped = db.kv_get(SKIPPED_KEY);
        let valid_url = release.html_url.starts_with(RELEASES_PAGE_PREFIX);
        let available = !release.draft
            && !release.prerelease
            && valid_url
            && release.tag_name == format!("v{version}")
            && version.split('.').count() == 3
            && version
                .split('.')
                .all(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
            && is_newer(&version, env!("CARGO_PKG_VERSION"))
            && (force || skipped.as_deref() != Some(version.as_str()));
        let result = if available {
            let package = update_package::download(&client, &version, &release.assets).await?;
            let info = UpdateInfo {
                version,
                url: release.html_url,
            };
            *self
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some((info.clone(), package));
            Some(info)
        } else {
            None
        };
        // Retry next launch if the process is killed before an available update is applied.
        if result.is_none() {
            let _ = db.kv_set(LAST_CHECK_KEY, &now.to_string());
        }
        Ok(result)
    }

    pub fn apply(&self, restart: bool) -> Result<bool, String> {
        let pending = self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some((_, package)) = pending {
            package.apply(restart)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

pub fn spawn(
    app: tauri::AppHandle,
    updater: Updater,
    db: Db,
    hub: ActivityHub,
    settings: watch::Receiver<Settings>,
) {
    let hub_for_actions = hub.clone();
    let for_actions = updater.clone();
    hub.register_handler(
        "app",
        Arc::new(move |_, action| {
            match action {
                "install" if for_actions.apply(true)? => app.exit(0),
                "later" => (),
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
                match updater.check(false, &db).await {
                    Ok(Some(info)) => {
                        tracing::info!(version = %info.version, "update staged");
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
                                ActivityAction::new("install", "download", "app.update.install")
                                    .primary(),
                                ActivityAction::new("later", "x", "app.update.onExit"),
                            ]),
                        );
                        break;
                    }
                    Ok(None) => (),
                    Err(error) => tracing::warn!(%error, "update check or download failed"),
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
