//! Fixed cleanup recipes (ADR-022). No caller-supplied paths or shell commands.
use crate::platform::maintenance::{LogDirectory, delete_log};
use serde::Serialize;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};
use ts_rs::TS;

const DAY: u64 = 24 * 60 * 60;
const WEEK: u64 = 7 * DAY;
const MAX_FILES: usize = 100_000;
const MAX_VISITED: u32 = 250_000;
/// Display order of the result groups.
const CATEGORIES: [&str; 9] = [
    "temp", "crash", "shaders", "windows", "browsers", "apps", "dev", "logs", "update",
];
/// Names never deleted from temp roots; app recovery data often lives there.
const PRESERVED: [&str; 3] = ["autosave", "recovery", "backup"];

#[derive(Clone, Default, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CleanupGroup {
    pub category: String,
    /// Candidate content found by the scan; the deletion pass revalidates each file.
    #[ts(type = "number")]
    pub estimated: u64,
    pub candidates: u32,
    #[ts(type = "number")]
    pub bytes: u64,
    pub files: u32,
}

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
    /// Empty subdirectories removed inside temp/cache roots (never a root itself).
    pub folders: u32,
    /// Files/directories that could not be processed, not an estimate of bytes.
    pub skipped: u32,
    pub busy_apps: u32,
    pub limited: bool,
    pub groups: Vec<CleanupGroup>,
}
impl CleanupProgress {
    fn group(&mut self, category: &str) -> &mut CleanupGroup {
        let index = match self.groups.iter().position(|g| g.category == category) {
            Some(index) => index,
            None => {
                self.groups.push(CleanupGroup {
                    category: category.into(),
                    ..Default::default()
                });
                self.groups
                    .sort_by_key(|g| CATEGORIES.iter().position(|c| *c == g.category));
                self.groups
                    .iter()
                    .position(|g| g.category == category)
                    .unwrap_or_default()
            }
        };
        &mut self.groups[index]
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Policy {
    /// Any file type, except `~` names and autosave/recovery/backup names or folders.
    Temp,
    /// Every file in a folder that only holds regenerable cache (`desktop.ini` kept).
    Cache,
    /// Only `thumbcache_*.db` / `iconcache_*.db` directly inside the folder.
    Thumbnails,
    /// Dated PCisland logs, by name and mtime.
    Logs,
}
struct Recipe {
    path: PathBuf,
    category: &'static str,
    /// Skip the whole recipe while this process runs (Chromium/Electron/Firefox caches).
    process: Option<&'static str>,
    policy: Policy,
    min_age: u64,
    /// Machine-wide root: only entries this account may delete count; access denial is
    /// expected for a standard user and not reported as skipped.
    shared: bool,
}
impl Recipe {
    fn new(path: PathBuf, category: &'static str, policy: Policy, min_age: u64) -> Self {
        Self {
            path,
            category,
            process: None,
            policy,
            min_age,
            shared: false,
        }
    }
    fn app(mut self, process: &'static str) -> Self {
        self.process = Some(process);
        self
    }
    fn shared(mut self) -> Self {
        self.shared = true;
        self
    }
    fn recursive(&self) -> bool {
        matches!(self.policy, Policy::Temp | Policy::Cache)
    }
}
pub struct Locations {
    pub local: PathBuf,
    pub roaming: PathBuf,
    pub windows_temp: PathBuf,
    pub program_data: PathBuf,
    pub logs: PathBuf,
}

const CHROMIUM_CACHES: [&str; 3] = ["Cache", "Code Cache", "GPUCache"];

fn recipes(loc: &Locations, progress: &mut CleanupProgress) -> Vec<Recipe> {
    let local = |p: &str| loc.local.join(p);
    let mut list = vec![
        Recipe::new(local("Temp"), "temp", Policy::Temp, DAY),
        Recipe::new(loc.windows_temp.clone(), "temp", Policy::Temp, DAY).shared(),
        Recipe::new(local("CrashDumps"), "crash", Policy::Cache, DAY),
    ];
    for queue in ["ReportArchive", "ReportQueue", "Temp"] {
        list.push(Recipe::new(
            local("Microsoft/Windows/WER").join(queue),
            "crash",
            Policy::Cache,
            DAY,
        ));
        list.push(
            Recipe::new(
                loc.program_data.join("Microsoft/Windows/WER").join(queue),
                "crash",
                Policy::Cache,
                DAY,
            )
            .shared(),
        );
    }
    // GPU drivers rebuild these; files a running game holds stay locked and skipped.
    for shader in [
        "D3DSCache",
        "NVIDIA/DXCache",
        "NVIDIA/GLCache",
        "AMD/DxCache",
        "AMD/DxcCache",
        "AMD/GLCache",
    ] {
        list.push(Recipe::new(local(shader), "shaders", Policy::Cache, DAY));
    }
    list.push(Recipe::new(
        local("Microsoft/Windows/Explorer"),
        "windows",
        Policy::Thumbnails,
        0,
    ));
    list.push(Recipe::new(
        local("Microsoft/Windows/INetCache"),
        "windows",
        Policy::Cache,
        DAY,
    ));
    for (base, process) in [
        ("Google/Chrome/User Data", "chrome.exe"),
        ("Microsoft/Edge/User Data", "msedge.exe"),
        ("BraveSoftware/Brave-Browser/User Data", "brave.exe"),
        ("Chromium/User Data", "chrome.exe"),
    ] {
        // Inspect only immediate, standard profiles; never search arbitrary AppData.
        for profile in child_directories(&local(base), progress) {
            let name = profile.file_name().unwrap_or_default().to_string_lossy();
            if name == "Default"
                || name == "Guest Profile"
                || name
                    .strip_prefix("Profile ")
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
            {
                for cache in CHROMIUM_CACHES {
                    list.push(
                        Recipe::new(profile.join(cache), "browsers", Policy::Cache, DAY)
                            .app(process),
                    );
                }
            }
        }
    }
    for profile in child_directories(&local("Mozilla/Firefox/Profiles"), progress) {
        list.push(
            Recipe::new(profile.join("cache2"), "browsers", Policy::Cache, DAY).app("firefox.exe"),
        );
    }
    // Electron documents these cache-only folders separately from Local Storage.
    for (base, process) in [
        (loc.roaming.join("discord"), "discord.exe"),
        (loc.roaming.join("Slack"), "slack.exe"),
        (loc.roaming.join("Code"), "code.exe"),
        (loc.roaming.join("Microsoft/Teams"), "teams.exe"),
        (
            local("Packages/MSTeams_8wekyb3d8bbwe/LocalCache/Microsoft/MSTeams/EBWebView/Default"),
            "ms-teams.exe",
        ),
    ] {
        for cache in CHROMIUM_CACHES {
            list.push(Recipe::new(base.join(cache), "apps", Policy::Cache, DAY).app(process));
        }
    }
    // Spotify's streaming/offline cache; offline downloads re-download on demand.
    for base in [
        local("Spotify"),
        local("Packages/SpotifyAB.SpotifyMusic_zpdnekdrzrea0/LocalCache/Spotify"),
    ] {
        for cache in ["Data", "Storage"] {
            list.push(Recipe::new(base.join(cache), "apps", Policy::Cache, DAY).app("spotify.exe"));
        }
    }
    // Package caches are content-addressed; a missing entry is a cache miss.
    for dev in ["npm-cache", "pip/cache", "Yarn/Cache"] {
        list.push(Recipe::new(local(dev), "dev", Policy::Cache, WEEK));
    }
    list.push(Recipe::new(loc.logs.clone(), "logs", Policy::Logs, WEEK));
    list
}

/// Windows Update download payloads and Delivery Optimization cache. Only used by the
/// elevated helper; services keep in-use files locked, which are skipped.
fn update_recipes(windows: &Path) -> Vec<Recipe> {
    [
        "SoftwareDistribution/Download",
        "SoftwareDistribution/DeliveryOptimization",
        "ServiceProfiles/NetworkService/AppData/Local/Microsoft/Windows/DeliveryOptimization/Cache",
    ]
    .into_iter()
    .map(|p| Recipe::new(windows.join(p), "update", Policy::Cache, DAY))
    .collect()
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

fn old_enough(modified: Option<SystemTime>, now: SystemTime, min_age: u64) -> bool {
    modified
        .and_then(|t| now.duration_since(t).ok())
        .is_some_and(|age| age.as_secs() >= min_age)
}

fn preserved_name(name: &str) -> bool {
    let name = name.to_lowercase();
    name.starts_with('~') || PRESERVED.iter().any(|s| name.contains(s))
}

fn eligible(recipe: &Recipe, path: &Path, meta: &std::fs::Metadata, now: SystemTime) -> bool {
    if !old_enough(meta.modified().ok(), now, recipe.min_age) {
        return false;
    }
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    match recipe.policy {
        Policy::Cache => !name.eq_ignore_ascii_case("desktop.ini"),
        Policy::Logs => super::logs::expired_name(&name, now),
        Policy::Temp => !preserved_name(&name),
        Policy::Thumbnails => {
            let name = name.to_lowercase();
            (name.starts_with("thumbcache_") || name.starts_with("iconcache_"))
                && name.ends_with(".db")
        }
    }
}

struct Candidate {
    path: PathBuf,
    recipe: usize,
}

struct Scan<'a, F: FnMut(&CleanupProgress)> {
    now: SystemTime,
    files: Vec<Candidate>,
    /// Post-order, so children are tried before their parent.
    folders: Vec<Candidate>,
    progress: &'a mut CleanupProgress,
    report: F,
}
impl<F: FnMut(&CleanupProgress)> Scan<'_, F> {
    fn skip(&mut self, recipe: &Recipe) {
        if !recipe.shared {
            self.progress.skipped += 1;
        }
    }
    fn dir(&mut self, recipe: &Recipe, index: usize, path: &Path, depth: usize) {
        if self.files.len() >= MAX_FILES || self.progress.scanned >= MAX_VISITED || depth > 32 {
            self.progress.limited = true;
            return;
        }
        let guard = match LogDirectory::open(path) {
            Ok(g) => g,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
            Err(_) => return self.skip(recipe),
        };
        let Ok(entries) = std::fs::read_dir(guard.path()) else {
            return self.skip(recipe);
        };
        for entry in entries {
            if self.files.len() >= MAX_FILES || self.progress.scanned >= MAX_VISITED {
                self.progress.limited = true;
                break;
            }
            self.progress.scanned += 1;
            let Ok(entry) = entry else {
                self.skip(recipe);
                continue;
            };
            let Ok(kind) = entry.file_type() else {
                self.skip(recipe);
                continue;
            };
            if kind.is_symlink() {
                self.skip(recipe);
                continue;
            }
            let path = entry.path();
            if kind.is_dir() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                // Do not traverse recovery subtrees or Outlook's opened attachments.
                if !recipe.recursive()
                    || (recipe.policy == Policy::Temp && preserved_name(&name))
                    || name == "content.outlook"
                {
                    continue;
                }
                let modified = entry.metadata().ok().and_then(|m| m.modified().ok());
                self.dir(recipe, index, &path, depth + 1);
                if old_enough(modified, self.now, recipe.min_age) {
                    self.folders.push(Candidate {
                        path,
                        recipe: index,
                    });
                }
            } else {
                // Shared roots: probe delete access so foreign files are not counted.
                match guard.file(&path, recipe.shared).and_then(|f| f.metadata()) {
                    Ok(meta) if eligible(recipe, &path, &meta, self.now) => {
                        let group = self.progress.group(recipe.category);
                        group.estimated += meta.len();
                        group.candidates += 1;
                        self.files.push(Candidate {
                            path,
                            recipe: index,
                        });
                    }
                    Ok(_) => {}
                    Err(_) => self.skip(recipe),
                }
            }
            self.progress.total = self.files.len() as u32;
            (self.report)(self.progress);
        }
    }
}

/// Uses two passes: discover candidates, then revalidate age, containment and locks
/// before each handle-based deletion. No recursive remove_dir_all, even for caches.
pub fn run(
    loc: Locations,
    now: SystemTime,
    processes: impl FnMut() -> Result<HashSet<String>, String>,
    send: impl FnMut(CleanupProgress),
) -> CleanupProgress {
    let mut progress = CleanupProgress::default();
    let recipes = recipes(&loc, &mut progress);
    execute(recipes, progress, now, processes, send)
}

/// Entry point of the elevated helper process; never reachable from the webview.
pub fn run_windows_update(windows: &Path, now: SystemTime) -> CleanupProgress {
    execute(
        update_recipes(windows),
        CleanupProgress::default(),
        now,
        || Ok(HashSet::new()),
        |_| {},
    )
}

fn execute(
    recipes: Vec<Recipe>,
    mut progress: CleanupProgress,
    now: SystemTime,
    mut processes: impl FnMut() -> Result<HashSet<String>, String>,
    mut send: impl FnMut(CleanupProgress),
) -> CleanupProgress {
    progress.phase = "scan".into();
    progress.category = recipes.first().map_or("temp", |r| r.category).into();
    send(progress.clone());
    let running = processes();
    let mut busy = HashSet::new();
    let mut tick = Instant::now();
    let mut scan = Scan {
        now,
        files: Vec::new(),
        folders: Vec::new(),
        progress: &mut progress,
        report: |p: &CleanupProgress| {
            if tick.elapsed() >= Duration::from_millis(100) {
                send(p.clone());
                tick = Instant::now();
            }
        },
    };
    for (index, recipe) in recipes.iter().enumerate() {
        scan.progress.category = recipe.category.into();
        if recipe
            .process
            .is_some_and(|p| running.as_ref().map_or(true, |r| r.contains(p)))
        {
            if recipe.path.exists() {
                busy.insert(recipe.process.unwrap_or_default());
            }
            continue;
        }
        (scan.report)(scan.progress);
        scan.dir(recipe, index, &recipe.path, 0);
    }
    let Scan { files, folders, .. } = scan;
    progress.phase = "delete".into();
    progress.total = files.len() as u32;
    progress.busy_apps = busy.len() as u32;
    send(progress.clone());
    let mut running = processes();
    let mut tick = Instant::now();
    let mut last_check = Instant::now();
    for candidate in files {
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
            busy.insert(recipe.process.unwrap_or_default());
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
                let group = progress.group(recipe.category);
                group.files += 1;
                group.bytes += size;
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
    // A folder that still has entries is refused by Windows; that is expected, not a skip.
    for folder in folders {
        let recipe = &recipes[folder.recipe];
        let blocked = recipe
            .process
            .is_some_and(|p| running.as_ref().map_or(true, |r| r.contains(p)));
        if !blocked && delete_empty_folder(&folder.path, recipe) {
            progress.folders += 1;
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
    if !eligible(recipe, path, &meta, now) {
        return None;
    }
    let size = meta.len();
    delete_log(file).ok()?;
    Some(size)
}

fn delete_empty_folder(path: &Path, recipe: &Recipe) -> bool {
    let Ok(root) = LogDirectory::open(&recipe.path) else {
        return false;
    };
    // Strictly inside the root: the recipe root itself is never removed.
    if !path.starts_with(root.path()) || path == root.path() {
        return false;
    }
    let Some(parent) = path.parent().and_then(|p| LogDirectory::open(p).ok()) else {
        return false;
    };
    parent.directory(path).and_then(delete_log).is_ok()
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
                program_data: self.0.join("programdata"),
                logs: self.0.join("data/logs"),
            }
        }
        fn aged(&self, name: &str, age: u64) -> PathBuf {
            let path = self.0.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, b"abc").unwrap();
            std::fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_times(
                    std::fs::FileTimes::new()
                        .set_modified(SystemTime::now() - Duration::from_secs(age)),
                )
                .unwrap();
            path
        }
        fn file(&self, name: &str, old: bool) -> PathBuf {
            self.aged(name, if old { WEEK + 60 } else { 0 })
        }
        fn run(&self, running: &[&str]) -> CleanupProgress {
            let running: HashSet<String> = running.iter().map(|s| s.to_string()).collect();
            run(
                self.locations(),
                SystemTime::now(),
                || Ok(running.clone()),
                |_| {},
            )
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn group<'a>(p: &'a CleanupProgress, category: &str) -> &'a CleanupGroup {
        p.groups.iter().find(|g| g.category == category).unwrap()
    }
    #[test]
    fn one_click_preserves_user_data_recent_files_and_active_app_caches() {
        let f = Fixture::new();
        let removed = [
            "local/Temp/stale.tmp",
            "local/Temp/document.docx",
            "local/Temp/installer.msi",
            "windows/Temp/nested/old.log",
            "local/Google/Chrome/User Data/Default/Cache/Cache_Data/old",
            "roaming/Slack/Code Cache/js/old",
            "data/logs/pcisland.2020-01-01.log",
        ];
        for path in removed {
            f.file(path, true);
        }
        let kept = [
            "local/Temp/recovery.tmp",
            "local/Temp/~lock.docx",
            "local/Temp/autosave/x.tmp",
            "local/Google/Chrome/User Data/Default/Cookies",
            "local/Google/Chrome/User Data/Default/History",
            "local/Google/Chrome/User Data/Default/Login Data",
            "local/Google/Chrome/User Data/Default/Local Storage/value",
            "local/Google/Chrome/User Data/Default/IndexedDB/value",
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
        assert_eq!((result.files, result.bytes, result.busy_apps), (7, 21, 1));
        assert_eq!((result.processed, result.total), (7, 7));
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
        assert_eq!(group(&result, "temp").files, 4);
        assert_eq!(
            f.run(&["msedge.exe"]).bytes,
            0,
            "repeat run must not report bytes again"
        );
    }
    #[test]
    fn new_sources_follow_their_age_and_name_rules() {
        let f = Fixture::new();
        let day = DAY + 60;
        let removed = [
            "local/CrashDumps/app.exe.123.dmp",
            "local/Microsoft/Windows/WER/ReportArchive/AppCrash_x/Report.wer",
            "programdata/Microsoft/Windows/WER/ReportQueue/AppHang_y/Report.wer",
            "local/D3DSCache/abc/data.idx",
            "local/NVIDIA/DXCache/x.bin",
            "local/AMD/DxcCache/y.bin",
            "local/Microsoft/Windows/Explorer/thumbcache_256.db",
            "local/Microsoft/Windows/Explorer/iconcache_32.db",
            "local/Microsoft/Windows/INetCache/IE/ABC/page.htm",
            "local/Temp/yesterday.dat",
        ];
        for path in removed {
            f.aged(path, day);
        }
        let dev = f.aged(
            "local/npm-cache/_cacache/content-v2/sha512/ab/cd",
            WEEK + 60,
        );
        let kept = [
            // Dev caches need a week, temp a day.
            "local/npm-cache/_cacache/index-v5/aa/bb",
            "local/Temp/today.dat",
            // Not a thumbnail database, and not recursed into.
            "local/Microsoft/Windows/Explorer/ExplorerStartupLog.etl",
            "local/Microsoft/Windows/Explorer/sub/thumbcache_1.db",
            "local/Microsoft/Windows/INetCache/desktop.ini",
            "local/Microsoft/Windows/INetCache/Content.Outlook/X/attachment.docx",
            // Outside every recipe.
            "local/Microsoft/Windows/WER/other.txt",
        ];
        f.aged(kept[0], day);
        f.aged(kept[1], 60);
        for path in &kept[2..] {
            f.aged(path, WEEK + 60);
        }
        let result = f.run(&[]);
        for path in removed {
            assert!(!f.0.join(path).exists(), "{path}");
        }
        assert!(!dev.exists());
        for path in kept {
            assert!(f.0.join(path).exists(), "{path}");
        }
        assert_eq!(result.files as usize, removed.len() + 1);
        assert_eq!(group(&result, "crash").files, 3);
        assert_eq!(group(&result, "shaders").files, 3);
        assert_eq!(group(&result, "windows").files, 3);
        assert_eq!(group(&result, "dev").files, 1);
        let order: Vec<_> = result.groups.iter().map(|g| g.category.as_str()).collect();
        assert_eq!(order, ["temp", "crash", "shaders", "windows", "dev"]);
        assert!(
            result
                .groups
                .iter()
                .all(|g| g.estimated == g.bytes && g.candidates == g.files)
        );
    }
    #[test]
    fn app_caches_are_skipped_while_the_app_runs() {
        let f = Fixture::new();
        let spotify = f.file("local/Spotify/Data/ab/cdef.file", true);
        let teams = f.file(
            "local/Packages/MSTeams_8wekyb3d8bbwe/LocalCache/Microsoft/MSTeams/EBWebView/Default/Cache/Cache_Data/f_1",
            true,
        );
        let classic = f.file("roaming/Microsoft/Teams/GPUCache/data_0", true);
        let teams_login = f.file(
            "local/Packages/MSTeams_8wekyb3d8bbwe/LocalCache/Microsoft/MSTeams/EBWebView/Default/Cookies",
            true,
        );
        let result = f.run(&["spotify.exe", "ms-teams.exe"]);
        assert_eq!(result.busy_apps, 2);
        assert!(spotify.exists() && teams.exists() && teams_login.exists());
        assert!(!classic.exists());
        let result = f.run(&[]);
        assert_eq!(result.files, 2);
        assert!(!spotify.exists() && !teams.exists() && teams_login.exists());
    }
    #[test]
    fn removes_only_old_empty_subfolders_and_never_roots() {
        let f = Fixture::new();
        let file = f.aged("local/Temp/build/obj/x.o", DAY + 60);
        let holder = f.aged("local/Temp/keep/new.txt", 60);
        let set_old = |dir: &str| {
            use std::os::windows::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .write(true)
                .custom_flags(0x0200_0000)
                .open(f.0.join(dir))
                .unwrap()
                .set_times(
                    std::fs::FileTimes::new()
                        .set_modified(SystemTime::now() - Duration::from_secs(DAY + 60)),
                )
                .unwrap();
        };
        // Folder times change when the fixture writes into them; age them afterwards.
        for dir in [
            "local/Temp/build/obj",
            "local/Temp/build",
            "local/Temp/keep",
        ] {
            set_old(dir);
        }
        std::fs::create_dir_all(f.0.join("local/Temp/fresh-empty")).unwrap();
        std::fs::create_dir_all(f.0.join("local/D3DSCache")).unwrap();
        let result = f.run(&[]);
        assert!(!file.exists());
        assert!(!f.0.join("local/Temp/build").exists());
        assert_eq!(result.folders, 2);
        assert!(
            holder.exists(),
            "folder with a recent file keeps its content"
        );
        assert_eq!(
            result.skipped, 0,
            "non-empty folders are not reported as skipped"
        );
        assert!(f.0.join("local/Temp/fresh-empty").exists());
        assert!(f.0.join("local/Temp").exists() && f.0.join("local/D3DSCache").exists());
    }
    #[test]
    fn skips_locked_files_changed_files_and_browsers_started_after_scan() {
        use std::os::windows::fs::OpenOptionsExt;
        let f = Fixture::new();
        let locked = f.file("local/Temp/locked.tmp", true);
        let changed = f.file("local/Temp/changed.tmp", true);
        let thumbs = f.file("local/Microsoft/Windows/Explorer/thumbcache_96.db", true);
        let browser = f.file("local/Google/Chrome/User Data/Default/Cache/old", true);
        let _lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&locked)
            .unwrap();
        let _thumbs_lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&thumbs)
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
        assert!(result.skipped >= 4);
        assert_eq!(result.busy_apps, 1);
        assert!(locked.exists() && changed.exists() && browser.exists() && thumbs.exists());
    }
    #[test]
    fn refuses_outside_paths_and_skips_cache_when_process_scan_fails() {
        let f = Fixture::new();
        let outside = f.file("outside/valuable.tmp", true);
        f.file("local/Temp/old.tmp", true);
        let recipe = Recipe::new(f.0.join("local/Temp"), "temp", Policy::Temp, DAY);
        let canonical = outside.canonicalize().unwrap();
        assert!(delete_candidate(&canonical, &recipe, SystemTime::now()).is_none());
        let root = f.0.join("local/Temp").canonicalize().unwrap();
        assert!(!delete_empty_folder(&root, &recipe));
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
        let result = f.run(&[]);
        assert_eq!(result.skipped, 1);
        assert_eq!(result.files, 0);
        assert!(unexpected.exists());
    }
    #[test]
    fn windows_update_recipe_cleans_payloads_only() {
        let f = Fixture::new();
        let payload = f.aged(
            "windows/SoftwareDistribution/Download/abc123/windows10.0-kb.cab",
            DAY + 60,
        );
        let fresh = f.aged("windows/SoftwareDistribution/Download/new/part.cab", 60);
        let datastore = f.aged("windows/SoftwareDistribution/DataStore/DataStore.edb", WEEK);
        let delivery = f.aged(
            "windows/ServiceProfiles/NetworkService/AppData/Local/Microsoft/Windows/DeliveryOptimization/Cache/x/y",
            DAY + 60,
        );
        let result = run_windows_update(&f.0.join("windows"), SystemTime::now());
        assert_eq!((result.files, result.bytes), (2, 6));
        assert!(!payload.exists() && !delivery.exists());
        assert!(fresh.exists() && datastore.exists());
        assert_eq!(group(&result, "update").files, 2);
    }
}
