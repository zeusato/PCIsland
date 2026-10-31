//! Data locations and portable detection (docs/modules/settings-storage.md §5,
//! docs/06-decision-log.md ADR-009).

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Paths {
    /// settings.json, pcisland.db, logs/.
    pub data_dir: PathBuf,
    pub portable: bool,
    /// `%LOCALAPPDATA%\PCisland`: WebView2 data, safe to wipe with `--cleanup`.
    pub local_dir: PathBuf,
    /// True when a portable `data\` exists but is read-only and we fell back.
    pub fell_back: bool,
}

impl Paths {
    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }
    pub fn settings_file(&self) -> PathBuf {
        self.data_dir.join("settings.json")
    }
    pub fn db_file(&self) -> PathBuf {
        self.data_dir.join("pcisland.db")
    }
}

pub fn resolve(args: &[String]) -> Paths {
    let local_dir = known_dir("LOCALAPPDATA").join("PCisland");
    let roaming = known_dir("APPDATA").join("PCisland");

    if let Some(dir) = arg_value(args, "--data-dir") {
        let data_dir = PathBuf::from(dir);
        return Paths {
            data_dir,
            portable: true,
            local_dir,
            fell_back: false,
        };
    }
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    let portable_dir = exe_dir.join("data");
    if portable_dir.is_dir() {
        if is_writable(&portable_dir) {
            return Paths {
                data_dir: portable_dir,
                portable: true,
                local_dir,
                fell_back: false,
            };
        }
        return Paths {
            data_dir: roaming,
            portable: false,
            local_dir,
            fell_back: true,
        };
    }
    Paths {
        data_dir: roaming,
        portable: false,
        local_dir,
        fell_back: false,
    }
}

pub fn arg_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn known_dir(var: &str) -> PathBuf {
    std::env::var_os(var).map_or_else(std::env::temp_dir, PathBuf::from)
}

fn is_writable(dir: &Path) -> bool {
    let probe = dir.join(".write-test");
    let ok = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(probe);
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_flag_wins() {
        let args = vec!["pcisland".into(), "--data-dir".into(), "X:\\custom".into()];
        let p = resolve(&args);
        assert_eq!(p.data_dir, PathBuf::from("X:\\custom"));
        assert!(p.portable);
    }

    #[test]
    fn arg_value_reads_next_token() {
        let args = vec!["a".into(), "--open".into(), "/timer".into()];
        assert_eq!(arg_value(&args, "--open"), Some("/timer"));
        assert_eq!(arg_value(&args, "--missing"), None);
    }
}
