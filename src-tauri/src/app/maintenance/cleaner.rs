//! Fixed, conservative cleanup recipes. No caller-supplied paths or shell commands.
use crate::platform::maintenance::{LogDirectory, delete_log};
use serde::Serialize;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};
use ts_rs::TS;

const WEEK: u64 = 7 * 24 * 60 * 60;
const MAX_FILES: usize = 100_000;
const MAX_VISITED: u32 = 250_000;

#[derive(Clone, Default, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CleanupProgress {
    pub phase: String,
    pub category: String,
    pub scanned: u32,
    pub total: u32,
    pub processed: u32,
    pub files: u32,
    #[ts(type = "number")]
    pub bytes: u64,
    /// Files/directories that could not be processed, not an estimate of bytes.
    pub skipped: u32,
    pub busy_apps: u32,
    pub limited: bool,
}

#[derive(Clone, Copy)]
enum Policy {
    Temp,
    Cache,
    Logs,
}
struct Recipe {
    path: PathBuf,
    category: &'static str,
    process: Option<&'static str>,
    policy: Policy,
}
pub struct Locations {
    pub local: PathBuf,
    pub roaming: PathBuf,
    pub windows_temp: PathBuf,
    pub logs: PathBuf,
}

fn recipes(loc: &Locations, progress: &mut CleanupProgress) -> Vec<Recipe> {
    let mut list = vec![
        Recipe {
            path: loc.local.join("Temp"),
            category: "temp",
            process: None,
            policy: Policy::Temp,
        },
        Recipe {
            path: loc.windows_temp.clone(),
            category: "temp",
            process: None,
            policy: Policy::Temp,
        },
        Recipe {
            path: loc.logs.clone(),
            category: "logs",
            process: None,
            policy: Policy::Logs,
        },
    ];
    for (base, process) in [
        ("Google/Chrome/User Data", "chrome.exe"),
        ("Microsoft/Edge/User Data", "msedge.exe"),
        ("BraveSoftware/Brave-Browser/User Data", "brave.exe"),
        ("Chromium/User Data", "chrome.exe"),
    ] {
        // Inspect only immediate, standard profiles; never search arbitrary AppData.
        let base = loc.local.join(base);
        for profile in child_directories(&base, progress) {
            let name = profile.file_name().unwrap_or_default().to_string_lossy();
            if name == "Default"
                || name == "Guest Profile"
                || name
                    .strip_prefix("Profile ")
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
            {
                for cache in ["Cache", "Code Cache", "GPUCache"] {
                    list.push(Recipe {
                        path: profile.join(cache),
                        category: "browsers",
                        process: Some(process),
                        policy: Policy::Cache,
                    });
                }
            }
        }
    }
    for profile in child_directories(&loc.local.join("Mozilla/Firefox/Profiles"), progress) {
        list.push(Recipe {
            path: profile.join("cache2"),
            category: "browsers",
            process: Some("firefox.exe"),
            policy: Policy::Cache,
        });
    }
    // Electron documents these cache-only folders separately from Local Storage.
    for (app, process) in [
        ("discord", "discord.exe"),
        ("Slack", "slack.exe"),
        ("Code", "code.exe"),
    ] {
        for cache in ["Cache", "Code Cache", "GPUCache"] {
            list.push(Recipe {
                path: loc.roaming.join(app).join(cache),
                category: "apps",
                process: Some(process),
                policy: Policy::Cache,
            });
        }
    }
    list
}

fn child_directories(path: &Path, progress: &mut CleanupProgress) -> Vec<PathBuf> {
    let guard = match LogDirectory::open(path) {
        Ok(guard) => guard,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return vec![],
        Err(_) => {
            progress.skipped += 1;
            return vec![];
        }
    };
    let Ok(entries) = std::fs::read_dir(guard.path()) else {
        progress.skipped += 1;
        return vec![];
    };
    let mut directories = Vec::new();
    for (index, entry) in entries.enumerate() {
        if index >= 256 {
            progress.limited = true;
            break;
        }
        progress.scanned += 1;
        let Ok(entry) = entry else {
            progress.skipped += 1;
            continue;
        };
        match entry.file_type() {
            Ok(kind) if kind.is_symlink() => progress.skipped += 1,
            Ok(kind) if kind.is_dir() => directories.push(entry.path()),
            Err(_) => progress.skipped += 1,
            _ => {}
        }
    }
    directories
}

fn eligible(policy: Policy, path: &Path, meta: &std::fs::Metadata, now: SystemTime) -> bool {
    let old = meta
        .modified()
        .ok()
        .and_then(|t| now.duration_since(t).ok())
        .is_some_and(|age| age.as_secs() > WEEK);
    if !old {
        return false;
    }
    match policy {
        Policy::Cache => true,
        Policy::Logs => {
            super::logs::expired_name(&path.file_name().unwrap_or_default().to_string_lossy(), now)
        }
        Policy::Temp => {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            // Recovery, installer payloads, downloads and arbitrary documents stay.
            !["autosave", "recovery", "backup"]
                .iter()
                .any(|s| name.contains(s))
                && !name.starts_with('~')
                && matches!(
                    path.extension()
                        .and_then(|e| e.to_str())
                        .map(str::to_ascii_lowercase)
                        .as_deref(),
                    Some("tmp" | "temp" | "log" | "etl")
                )
        }
    }
}

struct Candidate {
    path: PathBuf,
    recipe: usize,
}

// Explicit traversal state avoids holding filesystem iterators/handles in a global job.
#[allow(clippy::too_many_arguments)]
fn scan_dir(
    recipe: &Recipe,
    index: usize,
    path: &Path,
    depth: usize,
    now: SystemTime,
    candidates: &mut Vec<Candidate>,
    progress: &mut CleanupProgress,
    report: &mut impl FnMut(&CleanupProgress),
) {
    if candidates.len() >= MAX_FILES || progress.scanned >= MAX_VISITED || depth > 32 {
        progress.limited = true;
        return;
    }
    let guard = match LogDirectory::open(path) {
        Ok(g) => g,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(_) => {
            progress.skipped += 1;
            return;
        }
    };
    let Ok(entries) = std::fs::read_dir(guard.path()) else {
        progress.skipped += 1;
        return;
    };
    for entry in entries {
        if candidates.len() >= MAX_FILES || progress.scanned >= MAX_VISITED {
            progress.limited = true;
            break;
        }
        progress.scanned += 1;
        let Ok(entry) = entry else {
            progress.skipped += 1;
            continue;
        };
        let Ok(kind) = entry.file_type() else {
            progress.skipped += 1;
            continue;
        };
        if kind.is_symlink() {
            progress.skipped += 1;
            continue;
        }
        if kind.is_dir() {
            if !matches!(recipe.policy, Policy::Logs) {
                // Do not traverse app recovery subtrees inside the temp location.
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if matches!(recipe.policy, Policy::Temp)
                    && ["recovery", "autosave", "backup"]
                        .iter()
                        .any(|s| name.contains(s))
                {
                    continue;
                }
                scan_dir(
                    recipe,
                    index,
                    &entry.path(),
                    depth + 1,
                    now,
                    candidates,
                    progress,
                    report,
                );
            }
        } else {
            match guard.file(&entry.path(), false).and_then(|f| f.metadata()) {
                Ok(meta) if eligible(recipe.policy, &entry.path(), &meta, now) => {
                    candidates.push(Candidate {
                        path: entry.path(),
                        recipe: index,
                    })
                }
                Ok(_) => {}
                Err(_) => progress.skipped += 1,
            }
        }
        progress.total = candidates.len() as u32;
        report(progress);
    }
}

/// Uses two passes: discover candidates, then revalidate age, containment and locks
/// before each handle-based deletion. No recursive remove_dir_all, even for caches.
pub fn run(
    loc: Locations,
    now: SystemTime,
    mut processes: impl FnMut() -> Result<HashSet<String>, String>,
    mut send: impl FnMut(CleanupProgress),
) -> CleanupProgress {
    let mut progress = CleanupProgress {
        phase: "scan".into(),
        category: "temp".into(),
        ..Default::default()
    };
    send(progress.clone());
    let recipes = recipes(&loc, &mut progress);
    let running = processes();
    let mut busy = HashSet::new();
    let mut candidates = Vec::new();
    let mut tick = Instant::now();
    for (index, recipe) in recipes.iter().enumerate() {
        progress.category = recipe.category.into();
        if recipe
            .process
            .is_some_and(|p| running.as_ref().map_or(true, |r| r.contains(p)))
        {
            if recipe.path.exists() {
                busy.insert(recipe.process.unwrap());
            }
            continue;
        }
        send(progress.clone());
        scan_dir(
            recipe,
            index,
            &recipe.path,
            0,
            now,
            &mut candidates,
            &mut progress,
            &mut |p| {
                if tick.elapsed() >= Duration::from_millis(100) {
                    send(p.clone());
                    tick = Instant::now();
                }
            },
        );
    }
    progress.phase = "delete".into();
    progress.total = candidates.len() as u32;
    progress.busy_apps = busy.len() as u32;
    send(progress.clone());
    let mut running = processes();
    tick = Instant::now();
    let mut last_check = Instant::now();
    for candidate in candidates {
        if last_check.elapsed() >= Duration::from_millis(250) {
            running = processes();
            last_check = Instant::now();
        }
        let recipe = &recipes[candidate.recipe];
        progress.category = recipe.category.into();
        let blocked = recipe
            .process
            .is_some_and(|p| running.as_ref().map_or(true, |r| r.contains(p)));
        if blocked {
            busy.insert(recipe.process.unwrap());
        }
        let deleted = if blocked {
            None
        } else {
            delete_candidate(&candidate.path, recipe, now)
        };
        match deleted {
            Some(size) => {
                progress.files += 1;
                progress.bytes += size;
            }
            None => progress.skipped += 1,
        }
        progress.processed += 1;
        progress.busy_apps = busy.len() as u32;
        if tick.elapsed() >= Duration::from_millis(100) {
            send(progress.clone());
            tick = Instant::now();
        }
    }
    progress.phase = "done".into();
    send(progress.clone());
    progress
}

fn delete_candidate(path: &Path, recipe: &Recipe, now: SystemTime) -> Option<u64> {
    let root = LogDirectory::open(&recipe.path).ok()?;
    if !path.starts_with(root.path()) {
        return None;
    }
    let parent = LogDirectory::open(path.parent()?).ok()?;
    let file = parent.file(path, true).ok()?;
    let meta = file.metadata().ok()?;
    if !eligible(recipe.policy, path, &meta, now) {
        return None;
    }
    let size = meta.len();
    delete_log(file).ok()?;
    Some(size)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            Self(
                std::env::temp_dir()
                    .join(format!("pcisland-cleaner-{}", crate::core::id::new_id())),
            )
        }
        fn locations(&self) -> Locations {
            Locations {
                local: self.0.join("local"),
                roaming: self.0.join("roaming"),
                windows_temp: self.0.join("windows/Temp"),
                logs: self.0.join("data/logs"),
            }
        }
        fn file(&self, name: &str, old: bool) -> PathBuf {
            let path = self.0.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, b"abc").unwrap();
            if old {
                std::fs::File::options()
                    .write(true)
                    .open(&path)
                    .unwrap()
                    .set_times(
                        std::fs::FileTimes::new()
                            .set_modified(SystemTime::now() - Duration::from_secs(WEEK + 60)),
                    )
                    .unwrap();
            }
            path
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn one_click_preserves_user_data_recent_files_and_active_app_caches() {
        let f = Fixture::new();
        let removed = [
            "local/Temp/stale.tmp",
            "windows/Temp/nested/old.log",
            "local/Google/Chrome/User Data/Default/Cache/Cache_Data/old",
            "roaming/Slack/Code Cache/js/old",
            "data/logs/pcisland.2020-01-01.log",
        ];
        for path in removed {
            f.file(path, true);
        }
        let kept = [
            "local/Temp/document.docx",
            "local/Temp/installer.msi",
            "local/Temp/recovery.tmp",
            "local/Temp/autosave/x.tmp",
            "local/Google/Chrome/User Data/Default/Cookies",
            "local/Google/Chrome/User Data/Default/History",
            "local/Google/Chrome/User Data/Default/Login Data",
            "local/Google/Chrome/User Data/Default/Local Storage/value",
            "local/Microsoft/Edge/User Data/Default/Cache/old",
            "data/logs/notes.log",
        ];
        for path in kept {
            f.file(path, true);
        }
        let recent = f.file("local/Temp/recent.tmp", false);
        let mut phases = Vec::new();
        let result = run(
            f.locations(),
            SystemTime::now(),
            || Ok(HashSet::from(["msedge.exe".into()])),
            |p| phases.push(p.phase),
        );
        assert_eq!((result.files, result.bytes, result.busy_apps), (5, 15, 1));
        assert_eq!((result.processed, result.total), (5, 5));
        assert_eq!(phases.first().unwrap(), "scan");
        assert!(phases.iter().any(|p| p == "delete"));
        assert_eq!(phases.last().unwrap(), "done");
        for path in removed {
            assert!(!f.0.join(path).exists(), "{path}");
        }
        for path in kept {
            assert!(f.0.join(path).exists(), "{path}");
        }
        assert!(recent.exists());
        assert_eq!(
            run(
                f.locations(),
                SystemTime::now(),
                || Ok(HashSet::from(["msedge.exe".into()])),
                |_| {}
            )
            .bytes,
            0
        );
    }
    #[test]
    fn skips_locked_files_changed_files_and_browsers_started_after_scan() {
        use std::os::windows::fs::OpenOptionsExt;
        let f = Fixture::new();
        let locked = f.file("local/Temp/locked.tmp", true);
        let changed = f.file("local/Temp/changed.tmp", true);
        let browser = f.file("local/Google/Chrome/User Data/Default/Cache/old", true);
        let _lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&locked)
            .unwrap();
        let mut calls = 0;
        let result = run(
            f.locations(),
            SystemTime::now(),
            || {
                calls += 1;
                Ok(if calls > 1 {
                    HashSet::from(["chrome.exe".into()])
                } else {
                    HashSet::new()
                })
            },
            |p| {
                if p.phase == "delete" {
                    std::fs::write(&changed, b"recent write").unwrap();
                }
            },
        );
        assert_eq!((result.files, result.bytes), (0, 0));
        assert!(result.skipped >= 3);
        assert_eq!(result.busy_apps, 1);
        assert!(locked.exists() && changed.exists() && browser.exists());
    }
    #[test]
    fn refuses_outside_paths_and_skips_cache_when_process_scan_fails() {
        let f = Fixture::new();
        let outside = f.file("outside/valuable.tmp", true);
        f.file("local/Temp/old.tmp", true);
        let recipe = Recipe {
            path: f.0.join("local/Temp"),
            category: "temp",
            process: None,
            policy: Policy::Temp,
        };
        let canonical = outside.canonicalize().unwrap();
        assert!(delete_candidate(&canonical, &recipe, SystemTime::now()).is_none());
        let cache = f.file(
            "local/Mozilla/Firefox/Profiles/example.default/cache2/entries/old",
            true,
        );
        let result = run(
            f.locations(),
            SystemTime::now(),
            || Err("unavailable".into()),
            |_| {},
        );
        assert_eq!((result.files, result.bytes, result.busy_apps), (1, 3, 1));
        assert!(cache.exists() && outside.exists());
    }
    #[test]
    fn rejected_profile_roots_are_reported_and_never_deleted() {
        let f = Fixture::new();
        let unexpected = f.file("local/Google/Chrome/User Data", true);
        let result = run(
            f.locations(),
            SystemTime::now(),
            || Ok(HashSet::new()),
            |_| {},
        );
        assert_eq!(result.skipped, 1);
        assert_eq!(result.files, 0);
        assert!(unexpected.exists());
    }
}
