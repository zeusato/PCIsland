//! Typed settings with atomic, debounced persistence (docs/modules/settings-storage.md §5).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tauri::{AppHandle, Emitter};
use tokio::sync::watch;
use ts_rs::TS;

pub const SCHEMA_VERSION: u32 = 1;
const SAVE_DEBOUNCE: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub schema_version: u32,
    pub app: AppSettings,
    pub island: IslandSettings,
    pub timer: TimerSettings,
    pub pomodoro: PomodoroSettings,
    pub reminders: RemindersSettings,
    pub indicators: IndicatorsSettings,
    pub media: MediaSettings,
    pub local_player: LocalPlayerSettings,
    pub lyrics: LyricsSettings,
    pub notes: NotesSettings,
    pub shelf: ShelfSettings,
    pub ui: UiSettings,
    /// Keys written by a newer version are kept so a downgrade does not lose them.
    #[serde(flatten)]
    #[ts(skip)]
    pub unknown: Map<String, Value>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            app: AppSettings::default(),
            island: IslandSettings::default(),
            timer: TimerSettings::default(),
            pomodoro: PomodoroSettings::default(),
            reminders: RemindersSettings::default(),
            indicators: IndicatorsSettings::default(),
            media: MediaSettings::default(),
            local_player: LocalPlayerSettings::default(),
            lyrics: LyricsSettings::default(),
            notes: NotesSettings::default(),
            shelf: ShelfSettings::default(),
            ui: UiSettings::default(),
            unknown: Map::new(),
        }
    }
}

macro_rules! section {
    ($name:ident { $($field:ident : $ty:ty = $default:expr),* $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
        #[ts(export)]
        #[serde(rename_all = "camelCase", default)]
        pub struct $name { $(pub $field: $ty),* }
        impl Default for $name {
            fn default() -> Self { Self { $($field: $default),* } }
        }
    };
}

section!(Shortcuts {
    toggle_peek: String = "Ctrl+Alt+Space".into(),
    open_main: String = "Ctrl+Alt+I".into(),
    toggle_hidden: String = "Ctrl+Alt+H".into(),
    quick_note: String = "Ctrl+Alt+N".into(),
});
section!(AppSettings {
    language: String = "vi".into(),
    autostart: bool = true,
    sound_volume: f32 = 0.6,
    check_updates: bool = true,
    shortcuts: Shortcuts = Shortcuts::default(),
});
section!(IslandSettings {
    idle_display: String = "eyes".into(),
    hover_delay_ms: u32 = 250,
    collapse_delay_ms: u32 = 400,
    hide_in_fullscreen: bool = true,
    minimal_when_maximized: bool = true,
    alert_over_fullscreen: String = "critical".into(),
    reduce_motion: String = "system".into(),
});
section!(TimerSettings {
    presets_min: Vec<u32> = vec![1, 3, 5, 10, 15, 25, 45, 60],
    ring_max_sec: u32 = 60,
});
section!(PomodoroSettings {
    work_min: u32 = 25,
    short_break_min: u32 = 5,
    long_break_min: u32 = 15,
    long_every: u32 = 4,
    auto_start_breaks: bool = true,
    auto_start_work: bool = false,
    focus_with_pomodoro: bool = true,
});
section!(RemindersSettings {
    show_upcoming_min: u32 = 15,
    snooze_options_min: Vec<u32> = vec![5, 10, 30],
    default_time_when_date_only: String = "09:00".into(),
});
section!(IndicatorsSettings {
    volume_alert: bool = true,
    power_alert: bool = true,
    battery_low_thresholds: Vec<u8> = vec![20, 10],
});
section!(MediaSettings {
    external_enabled: bool = true,
    alert_on_track_change: bool = true,
});
section!(LocalPlayerSettings {
    volume: f32 = 0.8,
    resume_on_start: String = "paused".into(),
});
section!(LyricsSettings {
    enabled: bool = true,
    online_enabled: bool = true,
});
section!(NotesSettings {
    trash_days: u32 = 30,
    show_pinned_when_idle: bool = true,
});
section!(ShelfSettings {
    expire_hours: u32 = 24,
    max_items: u32 = 30,
});
section!(UiSettings {
    accent: String = "orange".into(),
    time_format: String = "24h".into(),
    glow_intensity: f32 = 1.0,
});

impl Settings {
    /// Clamps values into their valid ranges (docs/modules/settings-storage.md §5).
    pub fn validate(&mut self) {
        let i = &mut self.island;
        i.hover_delay_ms = i.hover_delay_ms.clamp(100, 800);
        i.collapse_delay_ms = i.collapse_delay_ms.clamp(150, 1500);
        if !["eyes", "clock", "pill", "hidden"].contains(&i.idle_display.as_str()) {
            i.idle_display = "eyes".into();
        }
        self.app.sound_volume = self.app.sound_volume.clamp(0.0, 1.0);
        self.local_player.volume = self.local_player.volume.clamp(0.0, 1.0);
        let p = &mut self.pomodoro;
        p.work_min = p.work_min.clamp(1, 180);
        p.short_break_min = p.short_break_min.clamp(1, 60);
        p.long_break_min = p.long_break_min.clamp(1, 120);
        p.long_every = p.long_every.clamp(1, 12);
        self.timer.ring_max_sec = self.timer.ring_max_sec.clamp(5, 600);
        self.ui.glow_intensity = self.ui.glow_intensity.clamp(0.0, 1.0);
        if !["vi", "en"].contains(&self.app.language.as_str()) {
            self.app.language = "vi".into();
        }
    }
}

/// Upgrades an older settings document in place. Add one arm per schema bump.
fn migrate(doc: &mut Value) {
    let version = doc
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if version < 1 {
        // v0 → v1: no structural change; defaults fill missing fields.
    }
    if let Some(obj) = doc.as_object_mut() {
        obj.insert("schemaVersion".into(), Value::from(SCHEMA_VERSION));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadOutcome {
    Fresh,
    Loaded,
    RecoveredFromBackup,
    ResetToDefaults,
}

pub fn load(path: &Path) -> (Settings, LoadOutcome) {
    let backup = backup_path(path);
    if !path.exists() {
        return (Settings::default(), LoadOutcome::Fresh);
    }
    if let Some(s) = read(path) {
        let _ = std::fs::copy(path, &backup);
        return (s, LoadOutcome::Loaded);
    }
    if let Some(s) = read(&backup) {
        return (s, LoadOutcome::RecoveredFromBackup);
    }
    (Settings::default(), LoadOutcome::ResetToDefaults)
}

fn read(path: &Path) -> Option<Settings> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut doc: Value = serde_json::from_str(&text).ok()?;
    migrate(&mut doc);
    let mut settings: Settings = serde_json::from_value(doc).ok()?;
    settings.validate();
    Some(settings)
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
}

/// Write to a temp file, then rename over the original (atomic on NTFS).
pub fn save(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(settings).map_err(std::io::Error::other)?;
    std::fs::write(&tmp, text)?;
    std::fs::rename(tmp, path)
}

/// Applies a JSON merge-patch to one section and returns the validated result.
pub fn apply_patch(current: &Settings, section: &str, patch: &Value) -> Result<Settings, String> {
    let mut doc = serde_json::to_value(current).map_err(|e| e.to_string())?;
    let target = doc
        .get_mut(section)
        .ok_or_else(|| format!("unknown settings section `{section}`"))?;
    merge(target, patch);
    let mut next: Settings = serde_json::from_value(doc).map_err(|e| e.to_string())?;
    next.validate();
    Ok(next)
}

fn merge(target: &mut Value, patch: &Value) {
    match (target, patch) {
        (Value::Object(t), Value::Object(p)) => {
            for (k, v) in p {
                merge(t.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (t, p) => *t = p.clone(),
    }
}

/// Shared settings handle: in-memory source of truth + debounced writer.
#[derive(Clone)]
pub struct SettingsHandle {
    tx: Arc<watch::Sender<Settings>>,
}

impl SettingsHandle {
    pub fn start(app: &AppHandle, path: PathBuf, initial: Settings) -> Self {
        let (tx, mut rx) = watch::channel(initial);
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            while rx.changed().await.is_ok() {
                tokio::time::sleep(SAVE_DEBOUNCE).await;
                let snapshot = rx.borrow_and_update().clone();
                for label in ["island", "main"] {
                    let _ = app.emit_to(label, crate::ipc::events::SETTINGS_CHANGED, &snapshot);
                }
                if let Err(err) = save(&path, &snapshot) {
                    tracing::error!(module = "settings", error = %err, "failed to save settings");
                }
            }
        });
        Self { tx: Arc::new(tx) }
    }

    pub fn get(&self) -> Settings {
        self.tx.borrow().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<Settings> {
        self.tx.subscribe()
    }

    pub fn update_section(&self, section: &str, patch: &Value) -> Result<Settings, String> {
        let next = apply_patch(&self.get(), section, patch)?;
        self.tx.send_replace(next.clone());
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_merges_and_clamps() {
        let s = Settings::default();
        let next = apply_patch(
            &s,
            "island",
            &serde_json::json!({ "hoverDelayMs": 5000, "idleDisplay": "clock" }),
        )
        .expect("patch");
        assert_eq!(next.island.hover_delay_ms, 800);
        assert_eq!(next.island.idle_display, "clock");
        assert_eq!(next.island.collapse_delay_ms, 400);
    }

    #[test]
    fn unknown_section_is_rejected() {
        assert!(apply_patch(&Settings::default(), "nope", &serde_json::json!({})).is_err());
    }

    #[test]
    fn unknown_keys_survive_a_roundtrip() {
        let doc = serde_json::json!({ "schemaVersion": 1, "futureModule": { "x": 1 } });
        let s: Settings = serde_json::from_value(doc).expect("parse");
        let back = serde_json::to_value(&s).expect("serialize");
        assert_eq!(back["futureModule"]["x"], 1);
    }

    #[test]
    fn corrupt_file_recovers_from_backup() {
        let dir = std::env::temp_dir().join(format!("pcisland-test-{}", crate::core::id::new_id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("settings.json");
        let mut good = Settings::default();
        good.ui.accent = "pink".into();
        save(&path, &good).expect("save");
        let (_, outcome) = load(&path); // creates the .bak
        assert_eq!(outcome, LoadOutcome::Loaded);
        std::fs::write(&path, "{ not json").expect("corrupt");
        let (loaded, outcome) = load(&path);
        assert_eq!(outcome, LoadOutcome::RecoveredFromBackup);
        assert_eq!(loaded.ui.accent, "pink");
        let _ = std::fs::remove_dir_all(dir);
    }
}
