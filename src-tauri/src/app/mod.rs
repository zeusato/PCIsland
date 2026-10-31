//! App lifecycle: logging, CLI flags, autostart, shortcuts, tray
//! (docs/modules/app-lifecycle.md).

pub mod commands;
pub mod shortcuts;
pub mod tray;

use std::path::Path;

use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

use crate::core::paths::Paths;

/// Daily-rotated log files kept for 7 days. The guard must live as long as the app.
pub fn init_logging(
    paths: &Paths,
    debug: bool,
) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(if debug { "debug" } else { "info" }));
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("pcisland")
        .filename_suffix("log")
        .max_log_files(7)
        .build(paths.logs_dir())
        .ok();
    let (file_layer, guard) = match appender {
        Some(a) => {
            let (writer, guard) = tracing_appender::non_blocking(a);
            (
                Some(fmt::layer().with_ansi(false).with_writer(writer)),
                Some(guard),
            )
        }
        None => (None, None),
    };
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer())
        .with(file_layer)
        .try_init();
    guard
}

/// Writes a crash file next to the logs (docs/02-system-design.md §14).
pub fn install_panic_hook(logs_dir: &Path) {
    let dir = logs_dir.to_path_buf();
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let text = format!(
            "PCisland {} panicked: {info}\n{}",
            env!("CARGO_PKG_VERSION"),
            std::backtrace::Backtrace::force_capture()
        );
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(
            dir.join(format!("crash-{}.log", crate::ipc::now_ms())),
            &text,
        );
        tracing::error!("{text}");
        default(info);
    }));
}

/// `--cleanup`: remove machine-local data and the autostart entry (ADR-009).
pub fn cleanup(paths: &Paths) {
    let _ = std::fs::remove_dir_all(&paths.local_dir);
    let _ = std::process::Command::new("reg")
        .args([
            "delete",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "PCisland",
            "/f",
        ])
        .output();
}

/// Keeps the Run key in sync with the setting and the current exe path, so a moved
/// portable folder keeps starting with Windows (docs/modules/app-lifecycle.md §5).
pub fn sync_autostart(app: &AppHandle, enabled: bool) {
    let launcher = app.autolaunch();
    let result = if enabled {
        // enable() rewrites the entry with the current exe path.
        launcher.enable()
    } else if launcher.is_enabled().unwrap_or(false) {
        launcher.disable()
    } else {
        Ok(())
    };
    if let Err(err) = result {
        tracing::warn!(module = "app", error = %err, "autostart sync failed");
    }
}
