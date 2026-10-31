//! Reminders: scheduling, snooze, missed handling, Vietnamese quick input
//! (docs/modules/reminders.md).
#![forbid(unsafe_code)]

pub mod commands;

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Datelike, Local, NaiveDateTime, TimeZone, Weekday};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, watch};
use ts_rs::TS;

use crate::core::activity::{Activity, ActivityAction, ActivityHub};
use crate::core::audio::{AudioEngine, SoundId};
use crate::core::events::{AppEvent, EventBus};
use crate::core::id::new_id;
use crate::core::priority;
use crate::core::settings::Settings;
use crate::core::storage::{Db, StorageError};
use crate::core::supervisor::{Shared, Supervisor, shared};
use crate::ipc::{events, now_ms};
pub use crate::shared::nlp_vi::Repeat;

/// A reminder that is more than this late on startup/wake counts as missed.
const MISSED_GRACE_MS: u64 = 2 * 60_000;
const RING_MS: u64 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum ReminderStatus {
    Pending,
    Ringing,
    Missed,
    Done,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: String,
    pub title: String,
    pub note: Option<String>,
    #[ts(type = "number")]
    pub due_at: u64,
    pub repeat: Repeat,
    pub lead_min: u32,
    #[ts(type = "number | null")]
    pub snoozed_until: Option<u64>,
    pub status: ReminderStatus,
    #[ts(type = "number | null")]
    pub completed_at: Option<u64>,
}

impl Reminder {
    /// When the reminder rings next.
    pub fn fire_at(&self) -> u64 {
        self.snoozed_until.unwrap_or(self.due_at)
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ReminderInput {
    pub title: String,
    pub note: Option<String>,
    #[ts(type = "number")]
    pub due_at: u64,
    pub repeat: Repeat,
    #[serde(default)]
    pub lead_min: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ParsedPreview {
    pub title: String,
    #[ts(type = "number | null")]
    pub due_at: Option<u64>,
    pub repeat: Repeat,
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct RemindersSnapshot {
    pub upcoming: Vec<Reminder>,
    pub ringing: Vec<Reminder>,
    pub missed_count: u32,
    #[ts(type = "number")]
    pub rev: u64,
}

/// Next occurrence strictly after `after`, keeping the local time of day of `due`.
pub fn next_occurrence(
    repeat: &Repeat,
    due: NaiveDateTime,
    after: NaiveDateTime,
) -> Option<NaiveDateTime> {
    use chrono::Duration as D;
    let mut t = due;
    let step = |t: NaiveDateTime| -> Option<NaiveDateTime> {
        Some(match repeat {
            Repeat::None => return None,
            Repeat::Every { minutes } => t + D::minutes(i64::from(*minutes)),
            Repeat::Daily => t + D::days(1),
            Repeat::Weekdays => {
                let mut n = t + D::days(1);
                while matches!(n.weekday(), Weekday::Sat | Weekday::Sun) {
                    n += D::days(1);
                }
                n
            }
            Repeat::Weekly { days } => {
                let mut n = t + D::days(1);
                for _ in 0..7 {
                    if days.contains(&(n.weekday().num_days_from_monday() as u8)) {
                        break;
                    }
                    n += D::days(1);
                }
                n
            }
            Repeat::Monthly { day } => {
                let (mut y, mut m) = (t.year(), t.month() + 1);
                if m > 12 {
                    y += 1;
                    m = 1;
                }
                // Day 31 in a 30-day month falls back to the last day.
                let d = (1..=u32::from(*day))
                    .rev()
                    .find_map(|d| chrono::NaiveDate::from_ymd_opt(y, m, d))?;
                d.and_time(t.time())
            }
        })
    };
    for _ in 0..100_000 {
        t = step(t)?;
        if t > after {
            return Some(t);
        }
    }
    None
}

fn local(ms: u64) -> NaiveDateTime {
    DateTime::from_timestamp_millis(ms as i64)
        .map(|d| d.with_timezone(&Local).naive_local())
        .unwrap_or_default()
}

fn to_ms(t: NaiveDateTime) -> u64 {
    Local
        .from_local_datetime(&t)
        .earliest()
        .map_or(0, |d| d.timestamp_millis().max(0) as u64)
}

pub fn parse_preview(text: &str) -> ParsedPreview {
    let p = crate::shared::nlp_vi::parse(text, Local::now().naive_local());
    ParsedPreview {
        title: p.title,
        due_at: p.due.map(to_ms),
        repeat: p.repeat,
    }
}

enum Cmd {
    Changed,
    Complete(String),
    Snooze(String, u32),
}

#[derive(Clone)]
pub struct RemindersHandle {
    tx: mpsc::UnboundedSender<Cmd>,
    db: Db,
}

impl RemindersHandle {
    pub fn list(&self, filter: &str) -> Result<Vec<Reminder>, StorageError> {
        let (cond, order) = match filter {
            "missed" => ("status = 'missed'", "due_at DESC"),
            "done" => ("status = 'done'", "completed_at DESC"),
            _ => (
                "status IN ('pending', 'ringing')",
                "COALESCE(snoozed_until, due_at)",
            ),
        };
        load(
            &self.db,
            &format!("WHERE {cond} ORDER BY {order} LIMIT 500"),
        )
    }

    pub fn create(&self, input: ReminderInput) -> Result<Reminder, StorageError> {
        let r = Reminder {
            id: new_id(),
            title: input.title.trim().chars().take(200).collect(),
            note: input.note.filter(|n| !n.trim().is_empty()),
            due_at: input.due_at,
            repeat: input.repeat,
            lead_min: input.lead_min,
            snoozed_until: None,
            status: ReminderStatus::Pending,
            completed_at: None,
        };
        save(&self.db, &r)?;
        let _ = self.tx.send(Cmd::Changed);
        Ok(r)
    }

    pub fn update(&self, id: &str, input: ReminderInput) -> Result<(), StorageError> {
        if let Some(mut r) = get(&self.db, id) {
            r.title = input.title.trim().chars().take(200).collect();
            r.note = input.note;
            r.due_at = input.due_at;
            r.repeat = input.repeat;
            r.lead_min = input.lead_min;
            r.snoozed_until = None;
            r.status = ReminderStatus::Pending;
            save(&self.db, &r)?;
        }
        let _ = self.tx.send(Cmd::Changed);
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), StorageError> {
        self.db.call(|c| {
            c.execute("DELETE FROM reminders_items WHERE id = ?1", [id])
                .map(|_| ())
        })?;
        let _ = self.tx.send(Cmd::Changed);
        Ok(())
    }

    pub fn complete(&self, id: &str) {
        let _ = self.tx.send(Cmd::Complete(id.into()));
    }

    pub fn snooze(&self, id: &str, minutes: u32) {
        let _ = self.tx.send(Cmd::Snooze(id.into(), minutes));
    }
}

fn load(db: &Db, clause: &str) -> Result<Vec<Reminder>, StorageError> {
    db.call(|c| {
        let mut stmt = c.prepare(&format!(
            "SELECT id, title, note, due_at, repeat_json, lead_min, snoozed_until, status, completed_at FROM reminders_items {clause}"
        ))?;
        let rows = stmt.query_map([], row_to_reminder)?;
        rows.collect()
    })
}

fn get(db: &Db, id: &str) -> Option<Reminder> {
    db.call(|c| {
        c.query_row(
            "SELECT id, title, note, due_at, repeat_json, lead_min, snoozed_until, status, completed_at FROM reminders_items WHERE id = ?1",
            [id],
            row_to_reminder,
        )
    })
    .ok()
}

fn row_to_reminder(r: &rusqlite::Row<'_>) -> rusqlite::Result<Reminder> {
    let repeat: String = r.get(4)?;
    let status: String = r.get(7)?;
    Ok(Reminder {
        id: r.get(0)?,
        title: r.get(1)?,
        note: r.get(2)?,
        due_at: r.get::<_, i64>(3)? as u64,
        repeat: serde_json::from_str(&repeat).unwrap_or(Repeat::None),
        lead_min: r.get::<_, i64>(5)? as u32,
        snoozed_until: r.get::<_, Option<i64>>(6)?.map(|v| v as u64),
        status: match status.as_str() {
            "ringing" => ReminderStatus::Ringing,
            "missed" => ReminderStatus::Missed,
            "done" => ReminderStatus::Done,
            _ => ReminderStatus::Pending,
        },
        completed_at: r.get::<_, Option<i64>>(8)?.map(|v| v as u64),
    })
}

fn save(db: &Db, r: &Reminder) -> Result<(), StorageError> {
    let status = match r.status {
        ReminderStatus::Pending => "pending",
        ReminderStatus::Ringing => "ringing",
        ReminderStatus::Missed => "missed",
        ReminderStatus::Done => "done",
    };
    let now = now_ms() as i64;
    db.call(|c| {
        c.execute(
            "INSERT INTO reminders_items (id, title, note, due_at, repeat_json, lead_min, snoozed_until, status, created_at, updated_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET title = ?2, note = ?3, due_at = ?4, repeat_json = ?5, lead_min = ?6,
               snoozed_until = ?7, status = ?8, updated_at = ?9, completed_at = ?10",
            rusqlite::params![
                r.id,
                r.title,
                r.note,
                r.due_at as i64,
                serde_json::to_string(&r.repeat).unwrap_or_default(),
                i64::from(r.lead_min),
                r.snoozed_until.map(|v| v as i64),
                status,
                now,
                r.completed_at.map(|v| v as i64),
            ],
        )
        .map(|_| ())
    })
}

/// Moves a fired/finished occurrence forward (repeat) or closes it.
fn finish(r: &mut Reminder, status: ReminderStatus, now: u64) {
    r.snoozed_until = None;
    match next_occurrence(&r.repeat, local(r.due_at), local(now)) {
        Some(next) => {
            r.due_at = to_ms(next);
            r.status = ReminderStatus::Pending;
        }
        None => {
            r.status = status;
            r.completed_at = (status == ReminderStatus::Done).then_some(now);
        }
    }
}

pub fn setup(
    app: &AppHandle,
    db: Db,
    hub: ActivityHub,
    audio: AudioEngine,
    settings: watch::Receiver<Settings>,
    bus: EventBus,
    sup: &Supervisor,
) -> RemindersHandle {
    let (tx, rx) = mpsc::unbounded_channel();
    let handle = RemindersHandle { tx, db: db.clone() };
    let for_actions = handle.clone();
    hub.register_handler(
        "reminders",
        Arc::new(move |activity, action| {
            let id = activity.strip_prefix("reminders:").unwrap_or_default();
            match action {
                "done" => for_actions.complete(id),
                a if a.starts_with("snooze_") => {
                    for_actions.snooze(id, a[7..].parse().unwrap_or(5))
                }
                _ => return Err(format!("unknown reminder action `{action}`")),
            }
            Ok(())
        }),
    );
    let rx = shared(rx);
    let app = app.clone();
    sup.spawn("reminders", move || {
        service(
            app.clone(),
            rx.clone(),
            db.clone(),
            hub.clone(),
            audio.clone(),
            settings.clone(),
            bus.clone(),
        )
    });
    handle
}

async fn service(
    app: AppHandle,
    rx: Shared<Cmd>,
    db: Db,
    hub: ActivityHub,
    audio: AudioEngine,
    settings: watch::Receiver<Settings>,
    bus: EventBus,
) {
    let mut events = bus.subscribe();
    let mut rev = 0u64;
    // Only broadcast when something visible changed (the loop also wakes on timers).
    let mut last_sent: Option<(Vec<Reminder>, Vec<Reminder>, u32)> = None;
    // Mirrors AppEvent::IslandHidden; a hidden island falls back to a Windows toast.
    let mut island_hidden = false;
    let mut ringing_since: std::collections::HashMap<String, u64> =
        std::collections::HashMap::new();
    loop {
        let now = now_ms();
        let mut active = load(&db, "WHERE status IN ('pending', 'ringing')").unwrap_or_default();
        let mut missed_now = 0u32;
        for r in &mut active {
            match r.status {
                ReminderStatus::Pending if r.fire_at() <= now => {
                    if now - r.fire_at() > MISSED_GRACE_MS {
                        // Machine was off or asleep: record as missed, roll repeats forward.
                        let mut missed = r.clone();
                        missed.id = if r.repeat == Repeat::None {
                            r.id.clone()
                        } else {
                            new_id()
                        };
                        missed.status = ReminderStatus::Missed;
                        missed.repeat = Repeat::None;
                        let _ = save(&db, &missed);
                        if r.repeat != Repeat::None {
                            finish(r, ReminderStatus::Missed, now);
                            let _ = save(&db, r);
                        } else {
                            r.status = ReminderStatus::Missed;
                        }
                        missed_now += 1;
                    } else {
                        r.status = ReminderStatus::Ringing;
                        ringing_since.insert(r.id.clone(), now);
                        let _ = save(&db, r);
                        let s = settings.borrow();
                        if let Some(sound) = SoundId::named(&s.app.sound_reminder) {
                            audio.ring(sound, s.app.sound_volume, Duration::from_millis(RING_MS));
                        }
                        if island_hidden {
                            crate::platform::toast::show(
                                &app.config().identifier,
                                &r.title,
                                r.note.as_deref().unwrap_or("PCisland"),
                            );
                        }
                        tracing::info!(module = "reminders", id = %r.id, toast = island_hidden, "reminder fired");
                    }
                }
                ReminderStatus::Ringing => {
                    let since = *ringing_since.entry(r.id.clone()).or_insert(now);
                    if now - since >= RING_MS {
                        // Ignored: keep it as missed, roll repeats forward.
                        let mut missed = r.clone();
                        missed.status = ReminderStatus::Missed;
                        if r.repeat != Repeat::None {
                            missed.id = new_id();
                            missed.repeat = Repeat::None;
                            finish(r, ReminderStatus::Missed, now);
                            let _ = save(&db, r);
                        } else {
                            r.status = ReminderStatus::Missed;
                        }
                        let _ = save(&db, &missed);
                        ringing_since.remove(&r.id);
                        hub.remove(&format!("reminders:{}", r.id));
                    }
                }
                _ => {}
            }
        }
        if missed_now > 0 {
            hub.alert(
                Activity::new(
                    "reminders",
                    "reminders:missed",
                    "reminders.missed_summary",
                    priority::ALERT,
                    crate::core::activity::payload(&CountPayload { count: missed_now }),
                )
                .transient(10_000),
            );
        }

        // Publish activities and state.
        let pending: Vec<Reminder> = active
            .iter()
            .filter(|r| r.status == ReminderStatus::Pending)
            .cloned()
            .collect();
        let ringing: Vec<Reminder> = active
            .iter()
            .filter(|r| r.status == ReminderStatus::Ringing)
            .cloned()
            .collect();
        for r in &ringing {
            let snoozes = settings.borrow().reminders.snooze_options_min.clone();
            let mut actions =
                vec![ActivityAction::new("done", "check", "reminders.action.done").primary()];
            actions.extend(snoozes.iter().map(|m| {
                ActivityAction::new(
                    &format!("snooze_{m}"),
                    "alarm-clock",
                    &format!("reminders.action.snooze{m}"),
                )
            }));
            hub.upsert(
                Activity::new(
                    "reminders",
                    format!("reminders:{}", r.id),
                    "reminders.due",
                    priority::CRITICAL,
                    serde_json::to_value(r).unwrap_or_default(),
                )
                .with_actions(actions),
            );
        }
        let window = u64::from(settings.borrow().reminders.show_upcoming_min) * 60_000;
        match pending
            .iter()
            .min_by_key(|r| r.fire_at())
            .filter(|r| window > 0 && r.fire_at() <= now + window)
        {
            Some(next) => hub.upsert(
                Activity::new(
                    "reminders",
                    "reminders:upcoming",
                    "reminders.upcoming",
                    priority::AMBIENT,
                    serde_json::to_value(next).unwrap_or_default(),
                )
                .with_actions(vec![ActivityAction::new(
                    "done",
                    "check",
                    "reminders.action.done",
                )]),
            ),
            None => hub.remove("reminders:upcoming"),
        }
        let missed_count = db
            .call(|c| {
                c.query_row(
                    "SELECT COUNT(*) FROM reminders_items WHERE status = 'missed'",
                    [],
                    |r| r.get::<_, i64>(0),
                )
            })
            .unwrap_or(0) as u32;
        let current = (pending.clone(), ringing.clone(), missed_count);
        if last_sent.as_ref() != Some(&current) {
            rev += 1;
            let snap = RemindersSnapshot {
                upcoming: pending.clone(),
                ringing: ringing.clone(),
                missed_count,
                rev,
            };
            for label in ["island", "main"] {
                let _ = app.emit_to(label, events::REMINDERS_STATE, &snap);
            }
            last_sent = Some(current);
        }

        // Sleep until the next event: a fire time, an upcoming window start, or a ring timeout.
        let mut next = pending
            .iter()
            .map(Reminder::fire_at)
            .min()
            .unwrap_or(u64::MAX);
        if let Some(first) = pending.iter().map(Reminder::fire_at).min() {
            next = next.min(first.saturating_sub(window).max(now + 1_000));
        }
        if !ringing.is_empty() {
            next = next.min(now + 5_000);
        }
        let wait = Duration::from_millis(next.saturating_sub(now_ms()).clamp(500, 30_000));
        let cmd = tokio::select! {
            cmd = async { rx.lock().await.recv().await } => match cmd { Some(c) => Some(c), None => return },
            // After sleep/resume: re-evaluate missed and due reminders immediately.
            // Island visibility only updates the toast fallback flag.
            Ok(ev @ (AppEvent::Resumed | AppEvent::IslandHidden(_))) = events.recv() => {
                if let AppEvent::IslandHidden(hidden) = ev {
                    island_hidden = hidden;
                }
                None
            }
            () = tokio::time::sleep(wait) => None,
        };
        let now = now_ms();
        match cmd {
            Some(Cmd::Complete(id)) => {
                if let Some(mut r) = get(&db, &id) {
                    finish(&mut r, ReminderStatus::Done, now);
                    let _ = save(&db, &r);
                }
                ringing_since.remove(&id);
                hub.remove(&format!("reminders:{id}"));
                audio.stop_alert();
            }
            Some(Cmd::Snooze(id, minutes)) => {
                if let Some(mut r) = get(&db, &id) {
                    r.status = ReminderStatus::Pending;
                    r.snoozed_until = Some(now + u64::from(minutes.clamp(1, 24 * 60)) * 60_000);
                    let _ = save(&db, &r);
                }
                ringing_since.remove(&id);
                hub.remove(&format!("reminders:{id}"));
                audio.stop_alert();
            }
            Some(Cmd::Changed) | None => {}
        }
        // Reminders that were deleted or edited must not keep stale activities.
        for id in ringing.iter().map(|r| r.id.clone()) {
            if get(&db, &id).is_none_or(|r| r.status != ReminderStatus::Ringing) {
                hub.remove(&format!("reminders:{id}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .and_then(|x| x.and_hms_opt(h, min, 0))
            .expect("date")
    }

    #[test]
    fn daily_keeps_time_of_day() {
        let due = dt(2026, 10, 30, 8, 0);
        assert_eq!(
            next_occurrence(&Repeat::Daily, due, dt(2026, 10, 30, 8, 0)),
            Some(dt(2026, 10, 31, 8, 0))
        );
        assert_eq!(
            next_occurrence(&Repeat::Daily, due, dt(2026, 11, 3, 9, 0)),
            Some(dt(2026, 11, 4, 8, 0))
        );
    }

    #[test]
    fn weekdays_skip_weekend() {
        // 2026-10-30 is a Friday.
        let due = dt(2026, 10, 30, 8, 0);
        assert_eq!(
            next_occurrence(&Repeat::Weekdays, due, due),
            Some(dt(2026, 11, 2, 8, 0))
        );
    }

    #[test]
    fn weekly_on_monday() {
        let due = dt(2026, 11, 2, 9, 0);
        assert_eq!(
            next_occurrence(&Repeat::Weekly { days: vec![0] }, due, due),
            Some(dt(2026, 11, 9, 9, 0))
        );
    }

    #[test]
    fn monthly_31st_clamps_to_month_end() {
        let due = dt(2026, 10, 31, 9, 0);
        assert_eq!(
            next_occurrence(&Repeat::Monthly { day: 31 }, due, due),
            Some(dt(2026, 11, 30, 9, 0))
        );
    }

    #[test]
    fn every_n_minutes_and_none() {
        let due = dt(2026, 10, 30, 9, 0);
        assert_eq!(
            next_occurrence(
                &Repeat::Every { minutes: 120 },
                due,
                dt(2026, 10, 30, 12, 0)
            ),
            Some(dt(2026, 10, 30, 13, 0))
        );
        assert_eq!(next_occurrence(&Repeat::None, due, due), None);
    }

    #[test]
    fn finish_rolls_repeat_forward_and_closes_one_shot() {
        let now = to_ms(dt(2026, 10, 30, 8, 5));
        let mut r = Reminder {
            id: "a".into(),
            title: "t".into(),
            note: None,
            due_at: to_ms(dt(2026, 10, 30, 8, 0)),
            repeat: Repeat::Daily,
            lead_min: 0,
            snoozed_until: Some(1),
            status: ReminderStatus::Ringing,
            completed_at: None,
        };
        finish(&mut r, ReminderStatus::Done, now);
        assert_eq!(r.status, ReminderStatus::Pending);
        assert_eq!(r.due_at, to_ms(dt(2026, 10, 31, 8, 0)));
        assert_eq!(r.snoozed_until, None);
        r.repeat = Repeat::None;
        finish(&mut r, ReminderStatus::Done, now);
        assert_eq!(r.status, ReminderStatus::Done);
        assert_eq!(r.completed_at, Some(now));
    }

    #[test]
    fn db_roundtrip() {
        let db = Db::open_in_memory().expect("db");
        let r = Reminder {
            id: "x".into(),
            title: "Họp".into(),
            note: None,
            due_at: 1_000,
            repeat: Repeat::Weekly { days: vec![0, 2] },
            lead_min: 5,
            snoozed_until: None,
            status: ReminderStatus::Pending,
            completed_at: None,
        };
        save(&db, &r).expect("save");
        assert_eq!(get(&db, "x"), Some(r));
    }
}

#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CountPayload {
    pub count: u32,
}
