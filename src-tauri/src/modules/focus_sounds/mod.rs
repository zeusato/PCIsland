//! Focus music downloaded on request from a separate release repository
//! (docs/modules/focus-sounds.md, ADR-023). Plays through local_player via the EventBus.
pub mod commands;
pub mod manifest;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::watch;
use ts_rs::TS;

use crate::core::events::{AppEvent, EventBus, MediaIntent, PomodoroPhase};
use crate::core::settings::Settings;
use manifest::{Manifest, PackName, SoundPack, SoundRole};

const INSTALLED: &str = "installed.json";

/// Packs downloaded completely, as the manifest described them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Installed {
    pub release: String,
    pub packs: Vec<SoundPack>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPack {
    pub id: String,
    pub role: SoundRole,
    pub name: PackName,
    pub tracks: u32,
    #[ts(type = "number")]
    pub bytes: u64,
    /// Every file still exists with its expected size.
    pub complete: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct FocusSoundsStatus {
    pub release: Option<String>,
    pub packs: Vec<InstalledPack>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum PackState {
    NotInstalled,
    Installed,
    UpdateAvailable,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPack {
    pub id: String,
    pub role: SoundRole,
    pub name: PackName,
    pub tracks: u32,
    #[ts(type = "number")]
    pub bytes: u64,
    pub state: PackState,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct FocusCatalog {
    pub release: String,
    pub license: String,
    pub packs: Vec<CatalogPack>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct FocusDownloadProgress {
    pub pack: String,
    pub file: String,
    #[ts(type = "number")]
    pub done_bytes: u64,
    #[ts(type = "number")]
    pub total_bytes: u64,
    pub done_files: u32,
    pub total_files: u32,
}

#[derive(Clone)]
pub struct FocusSoundsHandle(Arc<Inner>);

pub struct Inner {
    pub dir: PathBuf,
    busy: AtomicBool,
    cancel: AtomicBool,
    catalog: Mutex<Option<Manifest>>,
    lock: Mutex<()>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn read_installed(dir: &Path) -> Installed {
    std::fs::read(dir.join(INSTALLED))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn write_installed(dir: &Path, installed: &Installed) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join("installed.json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(installed)?)?;
    std::fs::rename(tmp, dir.join(INSTALLED))
}

fn track_path(dir: &Path, pack: &str, file: &str) -> PathBuf {
    dir.join(pack).join(file)
}

fn present(dir: &Path, pack: &str, track: &manifest::SoundTrack) -> bool {
    std::fs::metadata(track_path(dir, pack, &track.file)).is_ok_and(|m| m.len() == track.size)
}

pub fn status(dir: &Path) -> FocusSoundsStatus {
    let installed = read_installed(dir);
    FocusSoundsStatus {
        release: (!installed.release.is_empty()).then(|| installed.release.clone()),
        packs: installed
            .packs
            .iter()
            .map(|p| InstalledPack {
                id: p.id.clone(),
                role: p.role,
                name: p.name.clone(),
                tracks: p.tracks.len() as u32,
                bytes: p.tracks.iter().map(|t| t.size).sum(),
                complete: p.tracks.iter().all(|t| present(dir, &p.id, t)),
            })
            .collect(),
    }
}

/// A pack is up to date when every file has the same hash and is still on disk.
pub fn catalog(manifest: &Manifest, installed: &Installed, dir: &Path) -> FocusCatalog {
    FocusCatalog {
        release: manifest.release.clone(),
        license: manifest.license.clone(),
        packs: manifest
            .packs
            .iter()
            .map(|p| {
                let local = installed.packs.iter().find(|l| l.id == p.id);
                let state = match local {
                    None => PackState::NotInstalled,
                    Some(l)
                        if l.tracks == p.tracks
                            && p.tracks.iter().all(|t| present(dir, &p.id, t)) =>
                    {
                        PackState::Installed
                    }
                    Some(_) => PackState::UpdateAvailable,
                };
                CatalogPack {
                    id: p.id.clone(),
                    role: p.role,
                    name: p.name.clone(),
                    tracks: p.tracks.len() as u32,
                    bytes: p.tracks.iter().map(|t| t.size).sum(),
                    state,
                }
            })
            .collect(),
    }
}

/// Files to play for a Pomodoro phase from the packs the user selected.
pub fn files_for(
    dir: &Path,
    installed: &Installed,
    selected: &[String],
    role: SoundRole,
) -> Vec<PathBuf> {
    installed
        .packs
        .iter()
        .filter(|p| p.role == role && selected.contains(&p.id))
        .flat_map(|p| {
            p.tracks
                .iter()
                .filter(|t| present(dir, &p.id, t))
                .map(|t| track_path(dir, &p.id, &t.file))
        })
        .collect()
}

/// Streams one download, refusing anything longer than expected, and checks the hash.
pub struct Verifier {
    hasher: Sha256,
    written: u64,
    expected_size: u64,
    expected_sha: String,
}

impl Verifier {
    pub fn new(expected_size: u64, expected_sha: &str) -> Self {
        Self {
            hasher: Sha256::new(),
            written: 0,
            expected_size,
            expected_sha: expected_sha.to_ascii_lowercase(),
        }
    }
    pub fn push(&mut self, chunk: &[u8]) -> Result<(), String> {
        self.written += chunk.len() as u64;
        if self.written > self.expected_size {
            return Err("file larger than the manifest says".into());
        }
        self.hasher.update(chunk);
        Ok(())
    }
    pub fn finish(self) -> Result<(), String> {
        if self.written != self.expected_size {
            return Err("file size does not match the manifest".into());
        }
        let actual: String = self
            .hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if actual != self.expected_sha {
            return Err("checksum mismatch; the download was discarded".into());
        }
        Ok(())
    }
}

impl FocusSoundsHandle {
    pub fn dir(&self) -> &Path {
        &self.0.dir
    }

    pub async fn fetch_manifest(&self) -> Result<Manifest, String> {
        let client = client()?;
        let mut resp = client
            .get(manifest::MANIFEST_URL)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| format!("manifest download failed: {e}"))?;
        let mut body = Vec::new();
        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| format!("manifest download failed: {e}"))?
        {
            if body.len() + chunk.len() > manifest::MAX_MANIFEST {
                return Err("manifest too large".into());
            }
            body.extend_from_slice(&chunk);
        }
        let m = manifest::parse(&body)?;
        *lock(&self.0.catalog) = Some(m.clone());
        Ok(m)
    }

    pub fn cancel(&self) {
        self.0.cancel.store(true, Ordering::Relaxed);
    }

    pub async fn download(
        &self,
        packs: &[String],
        progress: impl Fn(FocusDownloadProgress),
    ) -> Result<FocusSoundsStatus, String> {
        if self.0.busy.swap(true, Ordering::AcqRel) {
            return Err("a download is already running".into());
        }
        self.0.cancel.store(false, Ordering::Relaxed);
        let result = self.download_inner(packs, progress).await;
        self.0.busy.store(false, Ordering::Release);
        result.map(|()| status(&self.0.dir))
    }

    async fn download_inner(
        &self,
        ids: &[String],
        progress: impl Fn(FocusDownloadProgress),
    ) -> Result<(), String> {
        let cached = lock(&self.0.catalog).clone();
        let manifest = match cached {
            Some(m) => m,
            None => self.fetch_manifest().await?,
        };
        let dir = self.0.dir.clone();
        let wanted: Vec<&SoundPack> = manifest
            .packs
            .iter()
            .filter(|p| ids.contains(&p.id))
            .collect();
        if wanted.is_empty() {
            return Err("no such pack".into());
        }
        let todo: Vec<(&SoundPack, &manifest::SoundTrack)> = wanted
            .iter()
            .flat_map(|p| {
                p.tracks
                    .iter()
                    .filter(|t| !present(&dir, &p.id, t))
                    .map(move |t| (*p, t))
            })
            .collect();
        let mut report = FocusDownloadProgress {
            pack: String::new(),
            file: String::new(),
            done_bytes: 0,
            total_bytes: todo.iter().map(|(_, t)| t.size).sum(),
            done_files: 0,
            total_files: todo.len() as u32,
        };
        progress(report.clone());
        let client = client()?;
        for (pack, track) in todo {
            report.pack.clone_from(&pack.id);
            report.file.clone_from(&track.title);
            let target = track_path(&dir, &pack.id, &track.file);
            std::fs::create_dir_all(target.parent().expect("pack dir"))
                .map_err(|e| e.to_string())?;
            let part = target.with_extension("mp3.part");
            let outcome = self
                .fetch_file(
                    &client,
                    &manifest::file_url(&manifest.release, &track.file),
                    &part,
                    track,
                    &mut report,
                    &progress,
                )
                .await;
            if let Err(error) = outcome {
                let _ = std::fs::remove_file(&part);
                return Err(error);
            }
            std::fs::rename(&part, &target).map_err(|e| e.to_string())?;
            report.done_files += 1;
            progress(report.clone());
        }
        // Record finished packs and drop files a newer release no longer lists.
        let _guard = lock(&self.0.lock);
        let mut installed = read_installed(&dir);
        for pack in wanted {
            if pack.tracks.iter().all(|t| present(&dir, &pack.id, t)) {
                prune(&dir.join(&pack.id), pack);
                installed.packs.retain(|p| p.id != pack.id);
                installed.packs.push(pack.clone());
            }
        }
        installed.release.clone_from(&manifest.release);
        write_installed(&dir, &installed).map_err(|e| e.to_string())
    }

    async fn fetch_file(
        &self,
        client: &reqwest::Client,
        url: &str,
        part: &Path,
        track: &manifest::SoundTrack,
        report: &mut FocusDownloadProgress,
        progress: &impl Fn(FocusDownloadProgress),
    ) -> Result<(), String> {
        use std::io::Write;
        let mut resp = client
            .get(url)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|e| format!("{}: {e}", track.file))?;
        let mut out =
            std::io::BufWriter::new(std::fs::File::create(part).map_err(|e| e.to_string())?);
        let mut verifier = Verifier::new(track.size, &track.sha256);
        let mut since_report = 0usize;
        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| format!("{}: {e}", track.file))?
        {
            if self.0.cancel.load(Ordering::Relaxed) {
                return Err("cancelled".into());
            }
            verifier.push(&chunk)?;
            out.write_all(&chunk).map_err(|e| e.to_string())?;
            report.done_bytes += chunk.len() as u64;
            since_report += chunk.len();
            if since_report >= 512 * 1024 {
                since_report = 0;
                progress(report.clone());
            }
        }
        out.flush().map_err(|e| e.to_string())?;
        drop(out);
        verifier
            .finish()
            .map_err(|e| format!("{}: {e}", track.file))
    }

    pub fn remove(&self, ids: &[String]) -> Result<FocusSoundsStatus, String> {
        if self.0.busy.load(Ordering::Acquire) {
            return Err("a download is running".into());
        }
        let _guard = lock(&self.0.lock);
        let dir = &self.0.dir;
        let mut installed = read_installed(dir);
        for id in ids.iter().filter(|id| manifest::safe_name(id)) {
            let pack_dir = dir.join(id);
            // Only files this module wrote: known MP3 names and partial downloads.
            if let Ok(entries) = std::fs::read_dir(&pack_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if manifest::safe_name(name.trim_end_matches(".part"))
                        && (name.ends_with(".mp3") || name.ends_with(".mp3.part"))
                    {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
            let _ = std::fs::remove_dir(&pack_dir);
            installed.packs.retain(|p| &p.id != id);
        }
        if installed.packs.is_empty() {
            installed.release.clear();
        }
        write_installed(dir, &installed).map_err(|e| e.to_string())?;
        Ok(status(dir))
    }
}

/// Deletes MP3s in a pack folder that the current manifest no longer lists.
fn prune(pack_dir: &Path, pack: &SoundPack) {
    let Ok(entries) = std::fs::read_dir(pack_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".mp3") && !pack.tracks.iter().any(|t| t.file == name) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("PCisland/", env!("CARGO_PKG_VERSION")))
        .https_only(true)
        .connect_timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

pub fn setup(
    data_dir: &Path,
    bus: EventBus,
    settings: watch::Receiver<Settings>,
) -> FocusSoundsHandle {
    let handle = FocusSoundsHandle(Arc::new(Inner {
        dir: data_dir.join("focus-sounds"),
        busy: AtomicBool::new(false),
        cancel: AtomicBool::new(false),
        catalog: Mutex::new(None),
        lock: Mutex::new(()),
    }));
    let dir = handle.0.dir.clone();
    let mut events = bus.subscribe();
    tauri::async_runtime::spawn(async move {
        // Pause on stop only what this module started.
        let mut started = false;
        while let Ok(event) = events.recv().await {
            let AppEvent::PomodoroPhase(phase) = event else {
                continue;
            };
            let selected = settings.borrow().pomodoro.focus_sound_packs.clone();
            let role = match phase {
                PomodoroPhase::Work => SoundRole::Focus,
                PomodoroPhase::Break => SoundRole::Break,
                PomodoroPhase::Stopped => {
                    if std::mem::take(&mut started) {
                        bus.send(AppEvent::MediaIntent(MediaIntent::Pause));
                    }
                    continue;
                }
            };
            if selected.is_empty() {
                continue;
            }
            let mut paths = files_for(&dir, &read_installed(&dir), &selected, role);
            if !paths.is_empty() {
                // Shuffle keeps the first file; start each phase somewhere else.
                let start = (crate::ipc::now_ms() as usize) % paths.len();
                paths.rotate_left(start);
                started = true;
                bus.send(AppEvent::MediaIntent(MediaIntent::PlayFiles {
                    paths,
                    shuffle: true,
                }));
            }
        }
    });
    handle
}

#[cfg(test)]
mod tests {
    use super::*;
    const SHA_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn temp() -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("pcisland-focus-{}", crate::core::id::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn verifier_checks_size_and_hash() {
        let mut ok = Verifier::new(3, SHA_ABC);
        ok.push(b"ab").unwrap();
        ok.push(b"c").unwrap();
        assert!(ok.finish().is_ok());
        let mut long = Verifier::new(2, SHA_ABC);
        assert!(long.push(b"abc").is_err());
        let mut wrong = Verifier::new(3, SHA_ABC);
        wrong.push(b"abd").unwrap();
        assert!(wrong.finish().is_err());
    }

    #[test]
    fn catalog_states_and_phase_files_follow_disk_and_selection() {
        let dir = temp();
        let m = manifest::parse(manifest::sample("focus-sounds-v1", SHA_ABC).as_bytes()).unwrap();
        let mut installed = Installed {
            release: m.release.clone(),
            packs: vec![m.packs[0].clone()],
        };
        std::fs::create_dir_all(dir.join("lofi")).unwrap();
        std::fs::write(dir.join("lofi/lofi-01-a.mp3"), b"abc").unwrap();
        let c = catalog(&m, &installed, &dir);
        assert_eq!(
            (c.packs[0].state, c.packs[1].state),
            (PackState::Installed, PackState::NotInstalled)
        );
        let selected = vec!["lofi".to_string(), "rest".to_string()];
        assert_eq!(
            files_for(&dir, &installed, &selected, SoundRole::Focus),
            [dir.join("lofi/lofi-01-a.mp3")]
        );
        assert!(files_for(&dir, &installed, &selected, SoundRole::Break).is_empty());
        assert!(files_for(&dir, &installed, &[], SoundRole::Focus).is_empty());
        // A newer release with a changed hash needs an update.
        installed.packs[0].tracks[0].sha256 = "0".repeat(64);
        assert_eq!(
            catalog(&m, &installed, &dir).packs[0].state,
            PackState::UpdateAvailable
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn remove_deletes_only_this_modules_files() {
        let dir = temp();
        let m = manifest::parse(manifest::sample("focus-sounds-v1", SHA_ABC).as_bytes()).unwrap();
        write_installed(
            &dir,
            &Installed {
                release: m.release.clone(),
                packs: vec![m.packs[0].clone()],
            },
        )
        .unwrap();
        std::fs::create_dir_all(dir.join("lofi")).unwrap();
        std::fs::write(dir.join("lofi/lofi-01-a.mp3"), b"abc").unwrap();
        std::fs::write(dir.join("lofi/notes.txt"), b"keep").unwrap();
        let handle = FocusSoundsHandle(Arc::new(Inner {
            dir: dir.clone(),
            busy: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            catalog: Mutex::new(None),
            lock: Mutex::new(()),
        }));
        let after = handle.remove(&["lofi".into(), "../x".into()]).unwrap();
        assert!(after.packs.is_empty());
        assert!(!dir.join("lofi/lofi-01-a.mp3").exists());
        assert!(dir.join("lofi/notes.txt").exists());
        let _ = std::fs::remove_dir_all(dir);
    }
}
