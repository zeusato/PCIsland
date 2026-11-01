use super::CleanupResult;
use crate::platform::maintenance::{LogDirectory, delete_log};
use std::{
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
const RETENTION: u64 = 7 * 24 * 60 * 60;

pub(super) fn expired_name(name: &str, now: SystemTime) -> bool {
    let timestamp = if let Some(date) = name
        .strip_prefix("pcisland.")
        .and_then(|s| s.strip_suffix(".log"))
    {
        if date.len() != 10 {
            return false;
        }
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .ok()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|d| d.and_utc().timestamp())
            .and_then(|s| u64::try_from(s).ok())
    } else {
        name.strip_prefix("crash-")
            .and_then(|s| s.strip_suffix(".log"))
            .and_then(|s| s.parse::<u64>().ok())
            .map(|n| n / 1000)
    };
    timestamp
        .and_then(|t| UNIX_EPOCH.checked_add(Duration::from_secs(t)))
        .and_then(|t| now.duration_since(t).ok())
        .is_some_and(|age| age.as_secs() > RETENTION)
}

/// Only dated PCisland logs older than seven days by both name and modification time.
pub fn run(root: &Path, now: SystemTime, deleting: bool) -> std::io::Result<CleanupResult> {
    let mut result = CleanupResult::default();
    let dir = match LogDirectory::open(root) {
        Ok(dir) => dir,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(result),
        Err(e) => return Err(e),
    };
    for entry in std::fs::read_dir(dir.path())? {
        let Ok(entry) = entry else {
            result.skipped += 1;
            continue;
        };
        if !expired_name(&entry.file_name().to_string_lossy(), now) {
            continue;
        }
        let Ok(file) = dir.file(&entry.path(), deleting) else {
            result.skipped += 1;
            continue;
        };
        let meta = file.metadata()?;
        if !meta
            .modified()
            .ok()
            .and_then(|t| now.duration_since(t).ok())
            .is_some_and(|age| age.as_secs() > RETENTION)
        {
            continue;
        }
        let bytes = meta.len();
        if deleting && delete_log(file).is_err() {
            result.skipped += 1;
            continue;
        }
        result.bytes += bytes;
        result.files += 1;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn directory_guard_rejects_outside_paths_and_blocks_replacement() {
        let root =
            std::env::temp_dir().join(format!("pcisland-clean-{}", crate::core::id::new_id()));
        std::fs::create_dir_all(&root).unwrap();
        let guard = LogDirectory::open(&root).unwrap();
        assert!(guard.file(&root.join("..\\outside.log"), true).is_err());
        assert!(std::fs::rename(&root, root.with_extension("moved")).is_err());
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
        assert!(LogDirectory::open(Path::new("relative/logs")).is_err());
    }
    #[test]
    fn accepts_only_expired_owned_names() {
        let now = UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        for name in [
            "settings.json",
            "pcisland.db",
            "notes.log",
            "pcisland.2099-01-01.log",
            "crash-no.log",
            "pcisland.2020-01-01.log.bak",
        ] {
            assert!(!expired_name(name, now));
        }
        assert!(expired_name("pcisland.2020-01-01.log", now));
        assert!(expired_name("crash-1600000000000.log", now));
    }
    #[test]
    fn scan_and_clean_preserve_current_unknown_and_nested_files() {
        let root =
            std::env::temp_dir().join(format!("pcisland-clean-{}", crate::core::id::new_id()));
        std::fs::create_dir_all(root.join("nested")).unwrap();
        for name in [
            "pcisland.2020-01-01.log",
            "settings.json",
            "nested/pcisland.2020-01-01.log",
            "pcisland.2099-01-01.log",
        ] {
            std::fs::write(root.join(name), b"abc").unwrap();
        }
        // Inject time; never mutate real log timestamps or touch the user's data.
        let now = SystemTime::now() + Duration::from_secs(RETENTION + 60);
        let scan = run(&root, now, false).unwrap();
        assert_eq!((scan.files, scan.bytes), (1, 3));
        let clean = run(&root, now, true).unwrap();
        assert_eq!((clean.files, clean.bytes), (1, 3));
        assert!(root.join("settings.json").exists());
        assert!(root.join("nested/pcisland.2020-01-01.log").exists());
        assert_eq!(run(&root, now, true).unwrap().bytes, 0);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn locked_file_is_skipped_and_new_mtime_is_preserved() {
        let root =
            std::env::temp_dir().join(format!("pcisland-clean-{}", crate::core::id::new_id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("pcisland.2020-01-01.log");
        std::fs::write(&path, b"abc").unwrap();
        assert_eq!(run(&root, SystemTime::now(), true).unwrap().bytes, 0);
        use std::os::windows::fs::OpenOptionsExt;
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .unwrap();
        let result = run(
            &root,
            SystemTime::now() + Duration::from_secs(RETENTION + 60),
            true,
        )
        .unwrap();
        assert_eq!((result.bytes, result.skipped), (0, 1));
        drop(lock);
        std::fs::remove_dir_all(root).unwrap();
    }
}
