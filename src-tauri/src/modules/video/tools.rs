//! yt-dlp as an on-demand helper (docs/modules/video-player.md §5.3, ADR-018).
//! Downloaded only after the user agrees, verified against the release's SHA2-256SUMS,
//! stored under `%LOCALAPPDATA%\PCisland\tools` (removed by `--cleanup`).

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ts_rs::TS;

use crate::core::storage::Db;
use crate::ipc::now_ms;

const KV: &str = "video.tools";
const RELEASE_API: &str = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";
const DOWNLOAD: &str = "https://github.com/yt-dlp/yt-dlp/releases/download";
const MAX_EXE: usize = 64 * 1024 * 1024;
const CHECK_EVERY_MS: u64 = 24 * 60 * 60 * 1000;
pub const RESOLVE_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_JSON: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Record {
    version: String,
    sha256: String,
    checked_at: u64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ToolsStatus {
    pub installed: bool,
    pub version: Option<String>,
}

pub fn exe(dir: &Path) -> PathBuf {
    dir.join("yt-dlp.exe")
}

fn record(db: &Db) -> Option<Record> {
    db.kv_get(KV).and_then(|v| serde_json::from_str(&v).ok())
}

pub fn status(db: &Db, dir: &Path) -> ToolsStatus {
    let installed = exe(dir).is_file();
    ToolsStatus {
        installed,
        version: installed.then(|| record(db).map(|r| r.version)).flatten(),
    }
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("PCisland/", env!("CARGO_PKG_VERSION")))
        .https_only(true)
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())
}

async fn latest_tag(client: &reqwest::Client) -> Result<String, String> {
    #[derive(Deserialize)]
    struct Release {
        tag_name: String,
    }
    let release: Release = client
        .get(RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| format!("release lookup failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("release lookup failed: {e}"))?;
    // Tags look like 2026.10.26; anything else is refused before it reaches a URL.
    if release.tag_name.is_empty()
        || !release
            .tag_name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
    {
        return Err("unexpected release tag".into());
    }
    Ok(release.tag_name)
}

async fn bytes(client: &reqwest::Client, url: &str, max: usize) -> Result<Vec<u8>, String> {
    let mut resp = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| format!("download failed: {e}"))?;
    if resp.content_length().is_some_and(|n| n > max as u64) {
        return Err("download too large".into());
    }
    let mut out = Vec::new();
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("download failed: {e}"))?
    {
        if out.len() + chunk.len() > max {
            return Err("download too large".into());
        }
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}

/// The expected hash of `yt-dlp.exe` from a `SHA2-256SUMS` file.
pub fn expected_sha(sums: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?.trim_start_matches('*');
        (name == "yt-dlp.exe" && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| hash.to_ascii_lowercase())
    })
}

pub fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Downloads `tag` (or the latest release) and swaps it in atomically.
pub async fn install(db: &Db, dir: &Path) -> Result<ToolsStatus, String> {
    let client = client()?;
    let tag = latest_tag(&client).await?;
    let sums = bytes(
        &client,
        &format!("{DOWNLOAD}/{tag}/SHA2-256SUMS"),
        64 * 1024,
    )
    .await?;
    let expected = expected_sha(&String::from_utf8_lossy(&sums)).ok_or("checksum missing")?;
    let exe_bytes = bytes(&client, &format!("{DOWNLOAD}/{tag}/yt-dlp.exe"), MAX_EXE).await?;
    let actual = sha256_hex(&exe_bytes);
    if actual != expected {
        return Err("checksum mismatch; the download was discarded".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tmp = dir.join("yt-dlp.exe.download");
    std::fs::write(&tmp, &exe_bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, exe(dir)).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })?;
    let rec = Record {
        version: tag,
        sha256: actual,
        checked_at: now_ms(),
    };
    let _ = db.kv_set(KV, &serde_json::to_string(&rec).unwrap_or_default());
    tracing::info!(module = "video", version = %rec.version, "yt-dlp installed");
    Ok(status(db, dir))
}

/// Checks once a day for a newer release and installs it. Errors are logged only.
pub async fn update_if_due(db: &Db, dir: &Path, force: bool) -> bool {
    let Some(rec) = record(db) else { return false };
    if !exe(dir).is_file() || (!force && now_ms().saturating_sub(rec.checked_at) < CHECK_EVERY_MS) {
        return false;
    }
    let Ok(client) = client() else { return false };
    let latest = match latest_tag(&client).await {
        Ok(tag) => tag,
        Err(error) => {
            tracing::warn!(module = "video", %error, "yt-dlp update check failed");
            return false;
        }
    };
    if latest == rec.version {
        let _ = db.kv_set(
            KV,
            &serde_json::to_string(&Record {
                checked_at: now_ms(),
                ..rec
            })
            .unwrap_or_default(),
        );
        return false;
    }
    match install(db, dir).await {
        Ok(_) => true,
        Err(error) => {
            tracing::warn!(module = "video", %error, "yt-dlp update failed");
            false
        }
    }
}

/// `yt-dlp --dump-single-json` for one URL. Blocking; run with `spawn_blocking`.
pub fn dump_json(dir: &Path, url: &str) -> Result<String, String> {
    let exe = exe(dir);
    if !exe.is_file() {
        return Err("tools_missing".into());
    }
    let out = crate::platform::process::run(
        &exe,
        &[
            "--dump-single-json",
            "--no-playlist",
            "--no-warnings",
            "--no-progress",
            "--",
            url,
        ],
        RESOLVE_TIMEOUT,
        MAX_JSON,
    )?;
    if !out.success {
        let reason = out
            .stderr
            .lines()
            .rev()
            .find(|l| l.contains("ERROR"))
            .unwrap_or("yt-dlp failed");
        return Err(reason.chars().take(300).collect());
    }
    String::from_utf8(out.stdout).map_err(|_| "invalid output".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_exe_hash_in_sums() {
        let h = "f5389d3698ecde4ef06f4e6b1ce0294fe9ac1396228c8990b8d3416d6d177a7d";
        let sums = format!(
            "{}  yt-dlp\n{h}  yt-dlp.exe\n{}  yt-dlp_x86.exe\n",
            "a".repeat(64),
            "b".repeat(64)
        );
        assert_eq!(expected_sha(&sums).as_deref(), Some(h));
        assert_eq!(
            expected_sha(&format!("{}  yt-dlp.exe", "z".repeat(64))),
            None
        );
        assert_eq!(expected_sha("nothing here"), None);
    }

    #[test]
    fn hashes_like_sha256sum() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
