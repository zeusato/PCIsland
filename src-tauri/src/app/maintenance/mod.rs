//! User-initiated maintenance; no automatic disk deletion or system-wide process trim.
#![forbid(unsafe_code)]
mod cleaner;
mod logs;
use crate::{
    core::paths::Paths,
    ipc::{IpcError, IpcResult},
    platform::{
        maintenance::{SystemResources, WindowsResources},
        memory::{self, MemoryProcess, MemoryReclaim, MemoryTarget},
    },
};
use serde::Serialize;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::{AppHandle, Manager};
use ts_rs::TS;

#[derive(Default)]
pub struct MaintenanceState {
    busy: Arc<AtomicBool>,
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
