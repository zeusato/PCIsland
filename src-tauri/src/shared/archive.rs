//! Streaming ZIP exports shared by shelf and meetings.
#![forbid(unsafe_code)]
use serde::Serialize;
use std::{
    collections::HashSet,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use ts_rs::TS;
#[derive(Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ShelfProgress {
    pub completed: u32,
    pub total: u32,
    pub name: String,
}

/// Reject reparse points, including Windows junctions (WalkDir alone isn't sufficient).
fn regular(path: &Path) -> std::io::Result<std::fs::Metadata> {
    let meta = std::fs::symlink_metadata(path)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err(std::io::Error::other(
                "Linked folders/files are not exported",
            ));
        }
    }
    if meta.file_type().is_symlink() {
        return Err(std::io::Error::other("Symlinks are not exported"));
    }
    Ok(meta)
}

pub fn zip_files(
    paths: &[PathBuf],
    destination: &Path,
    cancel: &AtomicBool,
    progress: impl Fn(ShelfProgress),
) -> Result<(), String> {
    let mut entries = Vec::new();
    let mut names = HashSet::new();
    for root in paths {
        regular(root).map_err(|e| format!("{}: {e}", root.display()))?;
        if root.is_dir() {
            let source = root.canonicalize().map_err(|e| e.to_string())?;
            let parent = destination
                .parent()
                .ok_or("Destination needs a folder")?
                .canonicalize()
                .map_err(|e| e.to_string())?;
            if parent.starts_with(source) {
                return Err("Export must be outside selected folders".into());
            }
        }
        let base = root
            .file_name()
            .ok_or("Cannot archive a filesystem root")?
            .to_string_lossy()
            .into_owned();
        let mut name = base.clone();
        let mut suffix = 2;
        while !names.insert(name.to_lowercase()) {
            name = format!("{suffix}-{base}");
            suffix += 1;
        }
        for entry in walkdir::WalkDir::new(root).follow_links(false) {
            if cancel.load(Ordering::Relaxed) {
                return Err("Cancelled".into());
            }
            let entry = entry.map_err(|e| e.to_string())?;
            regular(entry.path()).map_err(|e| format!("{}: {e}", entry.path().display()))?;
            if entry.path() == destination {
                return Err("Export must be outside selected folders".into());
            }
            let rel = entry.path().strip_prefix(root).map_err(|e| e.to_string())?;
            let archive_name = if rel.as_os_str().is_empty() {
                name.clone()
            } else {
                Path::new(&name)
                    .join(rel)
                    .to_string_lossy()
                    .replace('\\', "/")
            };
            entries.push((
                entry.path().to_owned(),
                archive_name,
                entry.file_type().is_dir(),
            ));
        }
    }
    let temp = destination.with_extension(format!("{}.partial", crate::core::id::new_id()));
    let result = (|| {
        let file = File::options()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let mut buffer = [0_u8; 65536];
        for (index, (path, name, dir)) in entries.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                return Err("Cancelled".into());
            }
            regular(path).map_err(|e| e.to_string())?;
            if *dir {
                archive
                    .add_directory(name, options)
                    .map_err(|e| e.to_string())?;
            } else {
                archive
                    .start_file(name, options)
                    .map_err(|e| e.to_string())?;
                let mut input = File::open(path).map_err(|e| e.to_string())?;
                loop {
                    if cancel.load(Ordering::Relaxed) {
                        return Err("Cancelled".into());
                    }
                    let count = input.read(&mut buffer).map_err(|e| e.to_string())?;
                    if count == 0 {
                        break;
                    }
                    archive
                        .write_all(&buffer[..count])
                        .map_err(|e| e.to_string())?;
                }
            }
            progress(ShelfProgress {
                completed: (index + 1) as u32,
                total: entries.len() as u32,
                name: name.clone(),
            });
        }
        archive
            .finish()
            .map_err(|e| e.to_string())?
            .sync_all()
            .map_err(|e| e.to_string())?;
        // Atomic publication, also supported by FAT/exFAT portable drives.
        crate::platform::files::publish(&temp, destination).map_err(|e| e.to_string())?;
        Ok(())
    })();
    let _ = std::fs::remove_file(temp);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn archive_duplicate_names_and_never_overwrite() {
        let root = std::env::temp_dir().join(crate::core::id::new_id());
        std::fs::create_dir_all(root.join("a")).unwrap();
        std::fs::create_dir_all(root.join("b")).unwrap();
        std::fs::write(root.join("a/test.txt"), b"first").unwrap();
        std::fs::write(root.join("b/test.txt"), b"second").unwrap();
        let out = root.join("all.zip");
        let paths = vec![root.join("a/test.txt"), root.join("b/test.txt")];
        zip_files(&paths, &out, &AtomicBool::new(false), |_| {}).unwrap();
        let mut zip = zip::ZipArchive::new(File::open(&out).unwrap()).unwrap();
        assert_eq!(zip.len(), 2);
        let mut text = String::new();
        zip.by_name("2-test.txt")
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        assert_eq!(text, "second");
        drop(zip);
        assert!(zip_files(&paths, &out, &AtomicBool::new(false), |_| {}).is_err());
        assert!(
            zip_files(
                &paths,
                &root.join("cancel.zip"),
                &AtomicBool::new(true),
                |_| {}
            )
            .is_err()
        );
        assert!(!root.join("cancel.zip").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
