//! User-initiated maintenance; no automatic disk deletion, process trim or app close.
#![forbid(unsafe_code)]
mod cleaner;
mod logs;
use crate::{
    core::paths::Paths,
    ipc::{IpcError, IpcResult},
    platform::{
        maintenance::{SystemResources, WindowsResources},
        memory::{self, AppCloseResult, MemoryApp, MemoryProcess, MemoryReclaim, MemoryTarget},
    },
};
use serde::Serialize;
use std::{
    collections::HashSet,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tauri::{AppHandle, Manager};
use ts_rs::TS;

#[derive(Default)]
pub struct MaintenanceState {
    busy: Arc<AtomicBool>,
    /// (pid, creation time) a graceful close left running; only these may be forced.
    escalated: Mutex<HashSet<(u32, String)>>,
}
struct Operation(Arc<AtomicBool>);
impl Drop for Operation {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
impl MaintenanceState {
    fn begin(&self) -> IpcResult<Operation> {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| IpcError::new("maintenance.busy", "maintenance in progress"))?;
        Ok(Operation(self.busy.clone()))
    }
    fn remember(&self, targets: &[MemoryTarget]) {
        if let Ok(mut set) = self.escalated.lock() {
            set.clear();
            set.extend(targets.iter().map(|t| (t.pid, t.started.clone())));
        }
    }
    /// Splits targets into those a graceful close left running and a refused count.
    /// One use: the remembered set is cleared either way.
    fn take_escalated(&self, targets: Vec<MemoryTarget>) -> (Vec<MemoryTarget>, u32) {
        let Ok(mut set) = self.escalated.lock() else {
            return (vec![], targets.len() as u32);
        };
        let (allowed, refused): (Vec<_>, Vec<_>) = targets
            .into_iter()
            .partition(|t| set.contains(&(t.pid, t.started.clone())));
        set.clear();
        (allowed, refused.len() as u32)
    }
}
#[derive(Default, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResult {
    #[ts(type = "number")]
    pub bytes: u64,
    pub files: u32,
    pub skipped: u32,
}
#[derive(Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSnapshot {
    #[ts(type = "number")]
    pub app_working_set: u64,
    #[ts(type = "number")]
    pub app_private_bytes: u64,
    #[ts(type = "number")]
    pub sampled_at: u64,
    #[ts(type = "number")]
    pub ram_total: u64,
    #[ts(type = "number")]
    pub ram_available: u64,
    #[ts(type = "number")]
    pub system_cache: u64,
    #[ts(type = "number")]
    pub disk_total: u64,
    #[ts(type = "number")]
    pub disk_free: u64,
}
fn snapshot(system: &impl SystemResources) -> IpcResult<ResourceSnapshot> {
    let s = system.sample().map_err(failed)?;
    Ok(ResourceSnapshot {
        app_working_set: s.working_set,
        app_private_bytes: s.private_bytes,
        sampled_at: crate::ipc::now_ms(),
        ram_total: s.ram_total,
        ram_available: s.ram_available,
        system_cache: s.system_cache,
        disk_total: s.disk_total,
        disk_free: s.disk_free,
    })
}
fn failed(error: impl ToString) -> IpcError {
    IpcError::new("maintenance.failed", error.to_string())
}

#[tauri::command]
pub async fn maintenance_resources() -> IpcResult<ResourceSnapshot> {
    tauri::async_runtime::spawn_blocking(|| snapshot(&WindowsResources))
        .await
        .map_err(failed)?
}
#[tauri::command]
pub async fn maintenance_logs(app: AppHandle, clean: bool) -> IpcResult<CleanupResult> {
    let operation = app.state::<MaintenanceState>().begin()?;
    let root = app.state::<Paths>().logs_dir();
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        logs::run(&root, std::time::SystemTime::now(), clean).map_err(failed)
    })
    .await
    .map_err(failed)?
}
#[tauri::command]
pub async fn maintenance_clean(
    app: AppHandle,
    progress: tauri::ipc::Channel<cleaner::CleanupProgress>,
) -> IpcResult<cleaner::CleanupProgress> {
    let operation = app.state::<MaintenanceState>().begin()?;
    let system = crate::platform::maintenance::system_directory().map_err(failed)?;
    let loc = cleaner::Locations {
        local: app.path().local_data_dir().map_err(failed)?,
        roaming: app.path().data_dir().map_err(failed)?,
        windows_temp: system
            .parent()
            .ok_or_else(|| failed("system directory unavailable"))?
            .join("Temp"),
        program_data: crate::platform::maintenance::program_data_directory().map_err(failed)?,
        logs: app.state::<Paths>().logs_dir(),
    };
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        cleaner::run(
            loc,
            std::time::SystemTime::now(),
            crate::platform::maintenance::running_process_names,
            |p| {
                let _ = progress.send(p);
            },
        )
    })
    .await
    .map_err(failed)
}

#[derive(Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct RecycleBinInfo {
    #[ts(type = "number")]
    pub bytes: u64,
    #[ts(type = "number")]
    pub items: u64,
}
#[tauri::command]
pub async fn maintenance_recycle_bin() -> IpcResult<RecycleBinInfo> {
    tauri::async_runtime::spawn_blocking(|| {
        let (bytes, items) = crate::platform::maintenance::recycle_bin().map_err(failed)?;
        Ok(RecycleBinInfo { bytes, items })
    })
    .await
    .map_err(failed)?
}
/// Irreversible; the UI shows the size and asks for confirmation first. Returns what
/// actually left the bin (before minus after), not the pre-confirmation estimate.
#[tauri::command]
pub async fn maintenance_empty_recycle_bin(app: AppHandle) -> IpcResult<RecycleBinInfo> {
    use crate::platform::maintenance::{empty_recycle_bin, recycle_bin};
    let operation = app.state::<MaintenanceState>().begin()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        let (bytes, items) = recycle_bin().map_err(failed)?;
        if items > 0 {
            empty_recycle_bin().map_err(failed)?;
        }
        let (after_bytes, after_items) = recycle_bin().map_err(failed)?;
        Ok(RecycleBinInfo {
            bytes: bytes.saturating_sub(after_bytes),
            items: items.saturating_sub(after_items),
        })
    })
    .await
    .map_err(failed)?
}

/// Fixed argument of the elevated helper; nothing else is ever passed to it.
pub const ELEVATED_UPDATE_ARG: &str = "--maintenance-windows-update";
const EXIT_MARKER: u32 = 0x2000_0000;
const EXIT_KIB_MAX: u32 = (1 << 27) - 1;
/// Exit code layout: marker bit 29, freed KiB in bits 2..29 (capped), bit 0 = partial,
/// bit 1 reserved zero. Anything else (crash, panic, early failure) reads as failed.
fn encode_exit(progress: &cleaner::CleanupProgress) -> u32 {
    let kib = (progress.bytes / 1024).min(u64::from(EXIT_KIB_MAX)) as u32;
    EXIT_MARKER | (kib << 2) | u32::from(progress.skipped > 0)
}
fn decode_exit(code: u32) -> Option<(bool, u64)> {
    (code & 0xE000_0000 == EXIT_MARKER && code & 2 == 0)
        .then(|| (code & 1 == 1, u64::from((code >> 2) & EXIT_KIB_MAX) * 1024))
}
/// Runs inside the elevated copy of pcisland.exe, before any app setup.
pub fn run_elevated_helper() -> i32 {
    let Some(windows) = crate::platform::maintenance::system_directory()
        .ok()
        .and_then(|s| s.parent().map(std::path::Path::to_path_buf))
    else {
        return 1;
    };
    encode_exit(&cleaner::run_windows_update(
        &windows,
        std::time::SystemTime::now(),
    )) as i32
}
#[derive(Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ElevatedCleanup {
    /// `done`, `partial`, `cancelled` (UAC declined) or `failed`.
    pub status: String,
    #[ts(type = "number")]
    pub bytes: u64,
}
#[tauri::command]
pub async fn maintenance_windows_update(app: AppHandle) -> IpcResult<ElevatedCleanup> {
    let operation = app.state::<MaintenanceState>().begin()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        let outcome =
            crate::platform::maintenance::run_elevated_self(ELEVATED_UPDATE_ARG, 10 * 60 * 1000);
        let (status, bytes) = match outcome {
            Ok(None) => ("cancelled", 0),
            Ok(Some(code)) => match decode_exit(code) {
                Some((partial, bytes)) => (if partial { "partial" } else { "done" }, bytes),
                None => ("failed", 0),
            },
            Err(error) => {
                tracing::warn!(%error, "elevated update cleanup failed");
                ("failed", 0)
            }
        };
        ElevatedCleanup {
            status: status.into(),
            bytes,
        }
    })
    .await
    .map_err(failed)
}

#[tauri::command]
pub async fn maintenance_apps(app: AppHandle) -> IpcResult<Vec<MemoryApp>> {
    let operation = app.state::<MaintenanceState>().begin()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        memory::apps().map_err(failed)
    })
    .await
    .map_err(failed)?
}
/// Step one: ask the app to close (WM_CLOSE) and wait a few seconds. Survivors are
/// remembered so step two can force them, and only them.
#[tauri::command]
pub async fn maintenance_close_app(
    app: AppHandle,
    targets: Vec<MemoryTarget>,
    allow_audio: bool,
) -> IpcResult<AppCloseResult> {
    let operation = app.state::<MaintenanceState>().begin()?;
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        memory::close(&targets, allow_audio, Duration::from_secs(5)).map_err(failed)
    })
    .await
    .map_err(failed)??;
    if matches!(result.status.as_str(), "pending" | "no_window") {
        app.state::<MaintenanceState>().remember(&result.remaining);
    }
    Ok(result)
}
/// Step two, only after the user confirms again: terminate what step one left running.
#[tauri::command]
pub async fn maintenance_force_close(
    app: AppHandle,
    targets: Vec<MemoryTarget>,
) -> IpcResult<AppCloseResult> {
    let state = app.state::<MaintenanceState>();
    let operation = state.begin()?;
    let (allowed, refused) = state.take_escalated(targets);
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        let mut result = if allowed.is_empty() {
            AppCloseResult {
                status: "skipped".into(),
                ..Default::default()
            }
        } else {
            memory::terminate(&allowed, Duration::from_secs(3)).map_err(failed)?
        };
        result.skipped += refused;
        Ok(result)
    })
    .await
    .map_err(failed)?
}

#[tauri::command]
pub async fn maintenance_processes(app: AppHandle) -> IpcResult<Vec<MemoryProcess>> {
    let operation = app.state::<MaintenanceState>().begin()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        memory::scan().map_err(failed)
    })
    .await
    .map_err(failed)?
}
#[tauri::command]
pub async fn maintenance_reclaim(
    app: AppHandle,
    targets: Vec<MemoryTarget>,
) -> IpcResult<MemoryReclaim> {
    let operation = app.state::<MaintenanceState>().begin()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        memory::reclaim(&targets).map_err(failed)
    })
    .await
    .map_err(failed)?
}
#[tauri::command]
pub async fn maintenance_open(tool: String) -> IpcResult<()> {
    tauri::async_runtime::spawn_blocking(move || WindowsResources.open(&tool).map_err(failed))
        .await
        .map_err(failed)?
}
#[tauri::command]
pub async fn maintenance_measure(
    targets: Vec<MemoryTarget>,
) -> IpcResult<memory::MemoryMeasurement> {
    tauri::async_runtime::spawn_blocking(move || memory::measure(&targets).map_err(failed))
        .await
        .map_err(failed)?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_overlapping_maintenance_and_releases_on_drop() {
        let state = MaintenanceState::default();
        let first = state.begin().unwrap();
        assert!(state.begin().is_err());
        drop(first);
        assert!(state.begin().is_ok());
    }
    #[test]
    fn force_close_is_limited_to_targets_a_graceful_close_left_running() {
        let state = MaintenanceState::default();
        let target = |pid: u32| MemoryTarget {
            pid,
            started: format!("t{pid}"),
        };
        assert_eq!(state.take_escalated(vec![target(1)]).1, 1);
        state.remember(&[target(1), target(2)]);
        let (allowed, refused) = state.take_escalated(vec![
            target(1),
            MemoryTarget {
                pid: 2,
                started: "reused".into(),
            },
            target(3),
        ]);
        assert_eq!((allowed.len(), allowed[0].pid, refused), (1, 1, 2));
        assert!(
            state.take_escalated(vec![target(1)]).0.is_empty(),
            "one use"
        );
    }
    #[test]
    fn elevated_exit_code_round_trips_and_rejects_crashes() {
        let progress = cleaner::CleanupProgress {
            bytes: 5 * 1024 * 1024,
            skipped: 2,
            ..Default::default()
        };
        assert_eq!(
            decode_exit(encode_exit(&progress)),
            Some((true, 5 * 1024 * 1024))
        );
        let clean = cleaner::CleanupProgress::default();
        assert_eq!(decode_exit(encode_exit(&clean)), Some((false, 0)));
        for crash in [0, 1, 101, 0xC000_0005, 0x2000_0002] {
            assert_eq!(decode_exit(crash), None, "{crash:#x}");
        }
        let huge = cleaner::CleanupProgress {
            bytes: u64::MAX,
            ..Default::default()
        };
        assert!(decode_exit(encode_exit(&huge)).is_some());
    }
    #[test]
    fn resource_adapter_reports_available_separately_from_cache() {
        struct Fake;
        impl SystemResources for Fake {
            fn sample(&self) -> Result<crate::platform::maintenance::Resources, String> {
                Ok(crate::platform::maintenance::Resources {
                    ram_total: 100,
                    ram_available: 40,
                    disk_total: 1000,
                    disk_free: 200,
                    working_set: 1,
                    private_bytes: 2,
                    system_cache: 15,
                })
            }
            fn open(&self, _: &str) -> Result<(), String> {
                Err("not launched".into())
            }
        }
        let s = snapshot(&Fake).unwrap();
        assert_eq!(
            (s.ram_available, s.system_cache, s.disk_free),
            (40, 15, 200)
        );
    }
}
