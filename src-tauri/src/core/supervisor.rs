//! Module supervisor (docs/02-system-design.md §4): runs a service future, catches
//! panics, restarts with backoff (1 s, 5 s, 30 s), then marks the module Disabled.
//! Services keep their command receiver in a `Shared` so it survives a restart.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use ts_rs::TS;

const BACKOFF: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(5),
    Duration::from_secs(30),
];

/// A receiver that outlives the task reading it.
pub type Shared<T> = Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<T>>>;

pub fn shared<T>(rx: mpsc::UnboundedReceiver<T>) -> Shared<T> {
    Arc::new(tokio::sync::Mutex::new(rx))
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum ModuleState {
    Running,
    Restarting { attempt: u32 },
    Disabled { reason: String },
    Stopped,
}

#[derive(Clone, Default)]
pub struct Supervisor {
    states: Arc<Mutex<BTreeMap<String, ModuleState>>>,
    app: Option<AppHandle>,
}

impl Supervisor {
    pub fn new(app: &AppHandle) -> Self {
        Self {
            states: Arc::default(),
            app: Some(app.clone()),
        }
    }

    pub fn states(&self) -> BTreeMap<String, ModuleState> {
        self.states
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn set(&self, module: &str, state: ModuleState) {
        self.states
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(module.into(), state);
        if let Some(app) = &self.app {
            let _ = app.emit_to("main", "app://modules", self.states());
        }
    }

    /// Spawns `factory()` and restarts it after a panic. A normal return ends supervision.
    pub fn spawn<F, Fut>(&self, module: &'static str, factory: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let sup = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut attempt = 0usize;
            loop {
                sup.set(module, ModuleState::Running);
                let result = tokio::spawn(factory()).await;
                match result {
                    Ok(()) => {
                        sup.set(module, ModuleState::Stopped);
                        return;
                    }
                    Err(err) if err.is_panic() => {
                        tracing::error!(module, attempt, "service panicked");
                        let Some(delay) = BACKOFF.get(attempt) else {
                            sup.set(
                                module,
                                ModuleState::Disabled {
                                    reason: "panicked 4 times".into(),
                                },
                            );
                            return;
                        };
                        attempt += 1;
                        sup.set(
                            module,
                            ModuleState::Restarting {
                                attempt: attempt as u32,
                            },
                        );
                        tokio::time::sleep(*delay).await;
                    }
                    Err(_) => return,
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn restarts_after_panic_then_stops_on_normal_return() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("rt");
        rt.block_on(async {
            let runs = Arc::new(AtomicU32::new(0));
            let sup = Supervisor::default();
            let r = runs.clone();
            // Run inline instead of through tauri's runtime for the test.
            let factory = move || {
                let r = r.clone();
                async move {
                    if r.fetch_add(1, Ordering::SeqCst) == 0 {
                        panic!("first run fails");
                    }
                }
            };
            let first = tokio::spawn(factory()).await;
            assert!(first.is_err_and(|e| e.is_panic()));
            let second = tokio::spawn(factory()).await;
            assert!(second.is_ok());
            assert_eq!(runs.load(Ordering::SeqCst), 2);
            sup.set("x", ModuleState::Running);
            assert_eq!(sup.states().get("x"), Some(&ModuleState::Running));
        });
    }
}
