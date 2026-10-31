//! ActivityHub: decides what the island shows (docs/modules/activity-hub.md, ADR-002).
//! Modules declare activities; the hub picks primary / secondary / alert and routes
//! quick actions back to the owning module.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot, watch};
use ts_rs::TS;

use super::priority;
use crate::ipc::{events, now_ms};

const ALERT_MIN_MS: u64 = 1_200;
const MAX_QUEUED_ALERTS: usize = 5;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ActivityAction {
    pub id: String,
    pub icon: String,
    pub label_key: String,
    pub primary: bool,
}

impl ActivityAction {
    pub fn new(id: &str, icon: &str, label_key: &str) -> Self {
        Self {
            id: id.into(),
            icon: icon.into(),
            label_key: label_key.into(),
            primary: false,
        }
    }
    pub fn primary(mut self) -> Self {
        self.primary = true;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum Placement {
    Auto,
    /// Privacy indicators: never primary, always visible as secondary.
    SecondaryOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: String,
    pub source: String,
    /// Selects the frontend view, e.g. `timer.running`.
    pub kind: String,
    pub priority: u8,
    /// `Some(ttl)` = transient alert; `None` = persistent.
    pub transient_ms: Option<u32>,
    /// Kind-specific data; the frontend view for `kind` knows its shape.
    #[ts(type = "unknown")]
    pub payload: Value,
    pub actions: Vec<ActivityAction>,
    pub placement: Placement,
    #[ts(type = "number")]
    pub started_at: u64,
}

impl Activity {
    pub fn new(
        source: &str,
        id: impl Into<String>,
        kind: &str,
        priority: u8,
        payload: Value,
    ) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            kind: kind.into(),
            priority,
            transient_ms: None,
            payload,
            actions: Vec::new(),
            placement: Placement::Auto,
            started_at: now_ms(),
        }
    }
    pub fn transient(mut self, ttl_ms: u32) -> Self {
        self.transient_ms = Some(ttl_ms);
        self
    }
    pub fn with_actions(mut self, actions: Vec<ActivityAction>) -> Self {
        self.actions = actions;
        self
    }
    #[allow(dead_code, reason = "used by the mic/camera indicator (F32, P4)")]
    pub fn secondary_only(mut self) -> Self {
        self.placement = Placement::SecondaryOnly;
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct IslandState {
    pub primary: Option<Activity>,
    pub secondary: Option<Activity>,
    pub alert: Option<Activity>,
    /// Extra critical activities waiting behind `alert` (shown as "+N").
    pub critical_waiting: u32,
    pub pinned_id: Option<String>,
    pub focus_active: bool,
    #[ts(type = "number")]
    pub rev: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TopInfo {
    pub priority: u8,
    pub has_alert: bool,
}

/// Filters applied before arbitration.
#[derive(Debug, Clone, Copy, Default)]
pub struct Filters {
    pub focus: bool,
    pub alerts_paused: bool,
}

/// Serializes a module's typed payload struct (ADR-010). Every payload type is
/// exported to TypeScript and mapped to its `kind` in `src/lib/registry/payloads.ts`.
pub fn payload<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// Pure arbitration over persistent activities (docs/modules/activity-hub.md §5).
/// Returns (primary, secondary, critical alert, number of further criticals).
pub fn arbitrate<'a>(
    persistent: &'a [Activity],
    pinned: Option<&str>,
) -> (
    Option<&'a Activity>,
    Option<&'a Activity>,
    Option<&'a Activity>,
    u32,
) {
    let mut criticals: Vec<&Activity> = persistent
        .iter()
        .filter(|a| a.priority >= priority::CRITICAL)
        .collect();
    criticals.sort_by_key(|a| std::cmp::Reverse(a.started_at));
    let critical = criticals.first().copied();
    let waiting = criticals.len().saturating_sub(1) as u32;

    let rest: Vec<&Activity> = persistent
        .iter()
        .filter(|a| a.priority < priority::CRITICAL)
        .collect();
    let rank = |a: &&Activity| (a.priority, a.started_at);
    let primary = pinned
        .and_then(|id| {
            rest.iter()
                .copied()
                .find(|a| a.id == id && a.placement == Placement::Auto)
        })
        .or_else(|| {
            rest.iter()
                .copied()
                .filter(|a| a.placement == Placement::Auto)
                .max_by_key(rank)
        });
    let secondary = rest
        .iter()
        .copied()
        .filter(|a| a.placement == Placement::SecondaryOnly)
        .max_by_key(rank)
        .or_else(|| {
            let source = primary.map(|p| p.source.as_str());
            rest.iter()
                .copied()
                // The idle screen (eyes/clock) is never worth a secondary slot.
                .filter(|a| {
                    a.placement == Placement::Auto
                        && a.priority > priority::IDLE
                        && Some(a.source.as_str()) != source
                })
                .max_by_key(rank)
        });
    (primary, secondary, critical, waiting)
}

/// Whether a transient alert may be shown under the current filters.
pub fn alert_allowed(alert: &Activity, filters: Filters) -> bool {
    if filters.alerts_paused {
        return false;
    }
    !(filters.focus && alert.source != "timer")
}

pub type ActionHandler = Arc<dyn Fn(&str, &str) -> Result<(), String> + Send + Sync>;

enum Cmd {
    Upsert(Activity),
    Remove(String),
    #[allow(dead_code, reason = "module disable from Settings (P3)")]
    RemoveSource(String),
    Alert(Activity),
    Dismiss(String),
    Pin(Option<String>),
    SetFocus(bool),
    PauseAlerts(Option<u64>),
    SetIslandHidden(bool),
    Snapshot(oneshot::Sender<IslandState>),
}

#[derive(Clone)]
pub struct ActivityHub {
    tx: mpsc::UnboundedSender<Cmd>,
    handlers: Arc<Mutex<HashMap<String, ActionHandler>>>,
    top: watch::Receiver<TopInfo>,
}

impl ActivityHub {
    pub fn start(app: &AppHandle, sup: &super::supervisor::Supervisor) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let (top_tx, top) = watch::channel(TopInfo::default());
        let rx = super::supervisor::shared(rx);
        let top_tx = Arc::new(top_tx);
        let app = app.clone();
        // State is rebuilt as modules publish again after a restart.
        sup.spawn("activity", move || {
            service(app.clone(), rx.clone(), top_tx.clone())
        });
        Self {
            tx,
            handlers: Arc::new(Mutex::new(HashMap::new())),
            top,
        }
    }

    pub fn upsert(&self, a: Activity) {
        let _ = self.tx.send(Cmd::Upsert(a));
    }
    pub fn remove(&self, id: &str) {
        let _ = self.tx.send(Cmd::Remove(id.into()));
    }
    #[allow(dead_code, reason = "module disable from Settings (P3)")]
    pub fn remove_source(&self, source: &str) {
        let _ = self.tx.send(Cmd::RemoveSource(source.into()));
    }
    pub fn alert(&self, a: Activity) {
        let _ = self.tx.send(Cmd::Alert(a));
    }
    pub fn dismiss(&self, id: &str) {
        let _ = self.tx.send(Cmd::Dismiss(id.into()));
    }
    pub fn pin(&self, id: Option<String>) {
        let _ = self.tx.send(Cmd::Pin(id));
    }
    pub fn set_focus(&self, on: bool) {
        let _ = self.tx.send(Cmd::SetFocus(on));
    }
    /// `until` = epoch ms, `None` resumes alerts.
    pub fn pause_alerts(&self, until: Option<u64>) {
        let _ = self.tx.send(Cmd::PauseAlerts(until));
    }
    pub fn set_island_hidden(&self, hidden: bool) {
        let _ = self.tx.send(Cmd::SetIslandHidden(hidden));
    }
    pub fn top(&self) -> watch::Receiver<TopInfo> {
        self.top.clone()
    }
    pub async fn snapshot(&self) -> IslandState {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(Cmd::Snapshot(tx));
        rx.await.unwrap_or_default()
    }

    /// Registers the quick-action handler of a module (`source`).
    pub fn register_handler(&self, source: &str, handler: ActionHandler) {
        self.handlers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(source.into(), handler);
    }

    /// Routes `activity_invoke` to the module that owns the activity.
    pub fn invoke(&self, activity_id: &str, action_id: &str) -> Result<(), String> {
        let source = activity_id.split(':').next().unwrap_or_default();
        let handler = self
            .handlers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(source)
            .cloned();
        match handler {
            Some(h) => h(activity_id, action_id),
            None => Err(format!("no handler for `{source}`")),
        }
    }
}

struct State {
    persistent: Vec<Activity>,
    queue: VecDeque<Activity>,
    showing: Option<(Activity, u64)>,
    pinned: Option<String>,
    filters: Filters,
    paused_until: Option<u64>,
    island_hidden: bool,
    last: IslandState,
}

async fn service(
    app: AppHandle,
    rx: super::supervisor::Shared<Cmd>,
    top: Arc<watch::Sender<TopInfo>>,
) {
    let mut s = State {
        persistent: Vec::new(),
        queue: VecDeque::new(),
        showing: None,
        pinned: None,
        filters: Filters::default(),
        paused_until: None,
        island_hidden: false,
        last: IslandState::default(),
    };
    loop {
        // Wake when the shown alert expires (or at least every second for pause expiry).
        let wait = s
            .showing
            .as_ref()
            .map(|(a, since)| {
                let ttl = u64::from(a.transient_ms.unwrap_or(0)).max(ALERT_MIN_MS);
                (since + ttl).saturating_sub(now_ms())
            })
            .unwrap_or(1_000);
        let cmd = tokio::select! {
            cmd = async { rx.lock().await.recv().await } => match cmd { Some(c) => Some(c), None => return },
            () = tokio::time::sleep(Duration::from_millis(wait.max(16))) => None,
        };
        let now = now_ms();
        match cmd {
            Some(Cmd::Upsert(a)) => match s.persistent.iter_mut().find(|x| x.id == a.id) {
                Some(existing) => {
                    let started_at = existing.started_at;
                    *existing = Activity { started_at, ..a };
                }
                None => s.persistent.push(a),
            },
            Some(Cmd::Remove(id)) => {
                s.persistent.retain(|a| a.id != id);
                s.queue.retain(|a| a.id != id);
                if s.showing.as_ref().is_some_and(|(a, _)| a.id == id) {
                    s.showing = None;
                }
            }
            Some(Cmd::RemoveSource(src)) => {
                s.persistent.retain(|a| a.source != src);
                s.queue.retain(|a| a.source != src);
                if s.showing.as_ref().is_some_and(|(a, _)| a.source == src) {
                    s.showing = None;
                }
            }
            Some(Cmd::Alert(a)) => {
                let filters = Filters {
                    alerts_paused: s.paused_until.is_some_and(|t| t > now),
                    ..s.filters
                };
                if !s.island_hidden && alert_allowed(&a, filters) {
                    if let Some((shown, since)) = s.showing.as_mut().filter(|(x, _)| x.id == a.id) {
                        // Same alert again (volume while holding the key): update in place.
                        *shown = a;
                        *since = now;
                    } else if let Some(q) = s.queue.iter_mut().find(|x| x.id == a.id) {
                        *q = a;
                    } else {
                        if s.queue.len() >= MAX_QUEUED_ALERTS {
                            s.queue.pop_front();
                        }
                        s.queue.push_back(a);
                    }
                }
            }
            Some(Cmd::Dismiss(id)) => {
                if s.showing.as_ref().is_some_and(|(a, _)| a.id == id) {
                    s.showing = None;
                }
                s.persistent
                    .retain(|a| !(a.id == id && a.priority >= priority::CRITICAL));
            }
            Some(Cmd::Pin(id)) => s.pinned = id,
            Some(Cmd::SetFocus(on)) => s.filters.focus = on,
            Some(Cmd::PauseAlerts(until)) => s.paused_until = until,
            Some(Cmd::SetIslandHidden(hidden)) => {
                s.island_hidden = hidden;
                if hidden {
                    s.queue.clear();
                    s.showing = None;
                }
            }
            Some(Cmd::Snapshot(reply)) => {
                let _ = reply.send(s.last.clone());
                continue;
            }
            None => {}
        }

        // Expire the shown alert, then promote the next one.
        if let Some((a, since)) = &s.showing {
            let ttl = u64::from(a.transient_ms.unwrap_or(0)).max(ALERT_MIN_MS);
            if now >= since + ttl {
                s.showing = None;
            }
        }
        if s.showing.is_none() {
            s.showing = s.queue.pop_front().map(|a| (a, now));
        }
        if s.pinned
            .as_ref()
            .is_some_and(|id| !s.persistent.iter().any(|a| &a.id == id))
        {
            s.pinned = None;
        }

        let (primary, secondary, critical, waiting) = arbitrate(&s.persistent, s.pinned.as_deref());
        let next = IslandState {
            primary: primary.cloned(),
            secondary: secondary.cloned(),
            alert: critical
                .cloned()
                .or_else(|| s.showing.as_ref().map(|(a, _)| a.clone())),
            critical_waiting: waiting,
            pinned_id: s.pinned.clone(),
            focus_active: s.filters.focus,
            rev: s.last.rev,
        };
        if next != s.last {
            s.last = IslandState {
                rev: s.last.rev + 1,
                ..next
            };
            for label in ["island", "main"] {
                let _ = app.emit_to(label, events::ISLAND_STATE, &s.last);
            }
        }
        let info = TopInfo {
            priority: s
                .persistent
                .iter()
                .map(|a| a.priority)
                .max()
                .unwrap_or(priority::IDLE),
            has_alert: s.last.alert.is_some(),
        };
        top.send_if_modified(|v| std::mem::replace(v, info) != info);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn a(source: &str, id: &str, prio: u8, at: u64) -> Activity {
        Activity {
            started_at: at,
            ..Activity::new(source, format!("{source}:{id}"), "k", prio, json!(null))
        }
    }

    #[test]
    fn empty_has_nothing() {
        assert_eq!(arbitrate(&[], None), (None, None, None, 0));
    }

    #[test]
    fn timer_beats_media_and_media_becomes_secondary() {
        let list = [
            a("media", "np", priority::MEDIA, 10),
            a("timer", "t1", priority::LIVE, 5),
        ];
        let (p, s, _, _) = arbitrate(&list, None);
        assert_eq!(p.map(|x| x.source.as_str()), Some("timer"));
        assert_eq!(s.map(|x| x.source.as_str()), Some("media"));
    }

    #[test]
    fn ties_go_to_the_most_recent() {
        let list = [
            a("timer", "a", priority::LIVE, 1),
            a("timer", "b", priority::LIVE, 2),
        ];
        assert_eq!(
            arbitrate(&list, None).0.map(|x| x.id.as_str()),
            Some("timer:b")
        );
    }

    #[test]
    fn idle_screen_is_never_secondary() {
        let list = [
            a("timer", "sw", priority::LIVE, 1),
            a("indicators", "idle", priority::IDLE, 0),
        ];
        let (p, s, _, _) = arbitrate(&list, None);
        assert_eq!(p.map(|x| x.source.as_str()), Some("timer"));
        assert_eq!(s, None);
    }

    #[test]
    fn secondary_must_come_from_another_module() {
        let list = [
            a("timer", "a", priority::LIVE, 1),
            a("timer", "b", priority::LIVE, 2),
        ];
        assert_eq!(arbitrate(&list, None).1, None);
    }

    #[test]
    fn pin_overrides_priority_and_falls_back_when_missing() {
        let list = [
            a("media", "np", priority::MEDIA, 1),
            a("timer", "t", priority::LIVE, 2),
        ];
        assert_eq!(
            arbitrate(&list, Some("media:np"))
                .0
                .map(|x| x.source.as_str()),
            Some("media")
        );
        assert_eq!(
            arbitrate(&list, Some("gone:x"))
                .0
                .map(|x| x.source.as_str()),
            Some("timer")
        );
    }

    #[test]
    fn critical_becomes_alert_newest_first_and_counts_waiting() {
        let list = [
            a("reminders", "r1", priority::CRITICAL, 1),
            a("reminders", "r2", priority::CRITICAL, 2),
            a("media", "np", priority::MEDIA, 3),
        ];
        let (p, _, alert, waiting) = arbitrate(&list, None);
        assert_eq!(alert.map(|x| x.id.as_str()), Some("reminders:r2"));
        assert_eq!(waiting, 1);
        assert_eq!(p.map(|x| x.source.as_str()), Some("media"));
    }

    #[test]
    fn secondary_only_never_becomes_primary_and_always_takes_secondary() {
        let mic = a("indicators", "mic", priority::LIVE, 9).secondary_only();
        let list = [
            mic,
            a("media", "np", priority::MEDIA, 1),
            a("timer", "t", priority::LIVE, 2),
        ];
        let (p, s, _, _) = arbitrate(&list, None);
        assert_eq!(p.map(|x| x.source.as_str()), Some("timer"));
        assert_eq!(s.map(|x| x.id.as_str()), Some("indicators:mic"));
    }

    #[test]
    fn focus_and_pause_filter_alerts() {
        let vol = a("indicators", "vol", priority::ALERT, 1);
        let tick = a("timer", "phase", priority::ALERT, 1);
        let focus = Filters {
            focus: true,
            alerts_paused: false,
        };
        assert!(!alert_allowed(&vol, focus));
        assert!(alert_allowed(&tick, focus));
        assert!(!alert_allowed(
            &tick,
            Filters {
                focus: false,
                alerts_paused: true
            }
        ));
        assert!(alert_allowed(&vol, Filters::default()));
    }
}
