//! Pure timer logic: every function takes `now` (epoch ms) explicitly so it is testable
//! and correct across sleep/resume (deadline-based, docs/modules/timer.md §5).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, rename = "CountdownState")]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum CountdownState {
    Running {
        #[ts(type = "number")]
        ends_at: u64,
    },
    Paused {
        #[ts(type = "number")]
        remaining_ms: u64,
    },
    Ringing {
        #[ts(type = "number")]
        ended_at: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Countdown {
    pub id: String,
    pub label: Option<String>,
    #[ts(type = "number")]
    pub duration_ms: u64,
    pub state: CountdownState,
}

impl Countdown {
    pub fn start(id: String, duration_ms: u64, label: Option<String>, now: u64) -> Self {
        Self {
            id,
            label,
            duration_ms,
            state: CountdownState::Running {
                ends_at: now + duration_ms,
            },
        }
    }
    pub fn remaining(&self, now: u64) -> u64 {
        match self.state {
            CountdownState::Running { ends_at } => ends_at.saturating_sub(now),
            CountdownState::Paused { remaining_ms } => remaining_ms,
            CountdownState::Ringing { .. } => 0,
        }
    }
    pub fn pause(&mut self, now: u64) {
        if let CountdownState::Running { .. } = self.state {
            self.state = CountdownState::Paused {
                remaining_ms: self.remaining(now),
            };
        }
    }
    pub fn resume(&mut self, now: u64) {
        if let CountdownState::Paused { remaining_ms } = self.state {
            self.state = CountdownState::Running {
                ends_at: now + remaining_ms,
            };
        }
    }
    /// Adds (or removes) time; a ringing timer starts again with `delta`.
    pub fn add(&mut self, delta_ms: i64, now: u64) {
        let shift = |v: u64| {
            if delta_ms >= 0 {
                v + delta_ms as u64
            } else {
                v.saturating_sub(delta_ms.unsigned_abs())
            }
        };
        self.state = match self.state {
            CountdownState::Running { ends_at } => CountdownState::Running {
                ends_at: shift(ends_at).max(now),
            },
            CountdownState::Paused { remaining_ms } => CountdownState::Paused {
                remaining_ms: shift(remaining_ms),
            },
            CountdownState::Ringing { .. } => CountdownState::Running {
                ends_at: now + delta_ms.max(0) as u64,
            },
        };
        if delta_ms > 0 {
            self.duration_ms += delta_ms as u64;
        }
    }
    /// Returns true when the timer just finished.
    pub fn check(&mut self, now: u64) -> bool {
        if let CountdownState::Running { ends_at } = self.state {
            if now >= ends_at {
                self.state = CountdownState::Ringing { ended_at: ends_at };
                return true;
            }
        }
        false
    }
    pub fn deadline(&self) -> Option<u64> {
        match self.state {
            CountdownState::Running { ends_at } => Some(ends_at),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Stopwatch {
    /// Set while running.
    #[ts(type = "number | null")]
    pub started_at: Option<u64>,
    #[ts(type = "number")]
    pub accumulated_ms: u64,
    #[ts(type = "Array<number>")]
    pub laps: Vec<u64>,
}

impl Stopwatch {
    pub fn elapsed(&self, now: u64) -> u64 {
        self.accumulated_ms + self.started_at.map_or(0, |s| now.saturating_sub(s))
    }
    pub fn start(&mut self, now: u64) {
        if self.started_at.is_none() {
            self.started_at = Some(now);
        }
    }
    pub fn pause(&mut self, now: u64) {
        self.accumulated_ms = self.elapsed(now);
        self.started_at = None;
    }
    pub fn lap(&mut self, now: u64) {
        self.laps.push(self.elapsed(now));
    }
    pub fn is_idle(&self) -> bool {
        self.started_at.is_none() && self.accumulated_ms == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Work,
    ShortBreak,
    LongBreak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PomodoroConfig {
    pub work_ms: u64,
    pub short_ms: u64,
    pub long_ms: u64,
    pub long_every: u32,
    pub auto_start_breaks: bool,
    pub auto_start_work: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum PomodoroRun {
    Running {
        #[ts(type = "number")]
        ends_at: u64,
    },
    Paused {
        #[ts(type = "number")]
        remaining_ms: u64,
    },
    /// Phase finished, waiting for the user to start the next one.
    Waiting,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Pomodoro {
    pub phase: Phase,
    /// Completed work phases in this cycle.
    pub completed_work: u32,
    pub run: PomodoroRun,
    #[ts(type = "number")]
    pub phase_ms: u64,
    #[ts(type = "number")]
    pub phase_started_at: u64,
}

/// What happened when a phase ended.
#[derive(Debug, Clone, PartialEq)]
pub struct PhaseEnd {
    pub finished: Phase,
    pub completed: bool,
    pub planned_ms: u64,
    pub started_at: u64,
    pub ended_at: u64,
    pub next: Phase,
    pub auto_started: bool,
}

impl Pomodoro {
    pub fn start(cfg: &PomodoroConfig, now: u64) -> Self {
        Self {
            phase: Phase::Work,
            completed_work: 0,
            run: PomodoroRun::Running {
                ends_at: now + cfg.work_ms,
            },
            phase_ms: cfg.work_ms,
            phase_started_at: now,
        }
    }
    pub fn remaining(&self, now: u64) -> u64 {
        match self.run {
            PomodoroRun::Running { ends_at } => ends_at.saturating_sub(now),
            PomodoroRun::Paused { remaining_ms } => remaining_ms,
            PomodoroRun::Waiting => 0,
        }
    }
    pub fn deadline(&self) -> Option<u64> {
        match self.run {
            PomodoroRun::Running { ends_at } => Some(ends_at),
            _ => None,
        }
    }
    pub fn pause(&mut self, now: u64) {
        if let PomodoroRun::Running { .. } = self.run {
            self.run = PomodoroRun::Paused {
                remaining_ms: self.remaining(now),
            };
        }
    }
    pub fn resume(&mut self, now: u64) {
        if let PomodoroRun::Paused { remaining_ms } = self.run {
            self.run = PomodoroRun::Running {
                ends_at: now + remaining_ms,
            };
        }
    }
    fn next_phase(&self, cfg: &PomodoroConfig, completed_work: u32) -> Phase {
        match self.phase {
            Phase::Work if completed_work % cfg.long_every == 0 => Phase::LongBreak,
            Phase::Work => Phase::ShortBreak,
            _ => Phase::Work,
        }
    }
    fn duration(cfg: &PomodoroConfig, phase: Phase) -> u64 {
        match phase {
            Phase::Work => cfg.work_ms,
            Phase::ShortBreak => cfg.short_ms,
            Phase::LongBreak => cfg.long_ms,
        }
    }
    /// Ends the current phase (naturally when `completed`, or skipped) and moves on.
    pub fn advance(&mut self, cfg: &PomodoroConfig, now: u64, completed: bool) -> PhaseEnd {
        let finished = self.phase;
        let completed_work = if finished == Phase::Work && completed {
            self.completed_work + 1
        } else {
            self.completed_work
        };
        let next = if finished == Phase::Work && !completed {
            Phase::ShortBreak
        } else {
            self.next_phase(cfg, completed_work.max(1))
        };
        let end = PhaseEnd {
            finished,
            completed,
            planned_ms: self.phase_ms,
            started_at: self.phase_started_at,
            ended_at: now,
            next,
            auto_started: match next {
                Phase::Work => cfg.auto_start_work,
                _ => cfg.auto_start_breaks,
            },
        };
        self.completed_work = if next == Phase::Work && finished == Phase::LongBreak {
            0
        } else {
            completed_work
        };
        self.phase = next;
        self.phase_ms = Self::duration(cfg, next);
        self.phase_started_at = now;
        self.run = if end.auto_started {
            PomodoroRun::Running {
                ends_at: now + self.phase_ms,
            }
        } else {
            PomodoroRun::Waiting
        };
        end
    }
    /// Starts the waiting phase.
    pub fn start_waiting(&mut self, now: u64) {
        if self.run == PomodoroRun::Waiting {
            self.phase_started_at = now;
            self.run = PomodoroRun::Running {
                ends_at: now + self.phase_ms,
            };
        }
    }
    pub fn check(&mut self, cfg: &PomodoroConfig, now: u64) -> Option<PhaseEnd> {
        match self.run {
            PomodoroRun::Running { ends_at } if now >= ends_at => {
                Some(self.advance(cfg, ends_at, true))
            }
            _ => None,
        }
    }
    pub fn in_work(&self) -> bool {
        self.phase == Phase::Work && matches!(self.run, PomodoroRun::Running { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60_000;

    fn cfg() -> PomodoroConfig {
        PomodoroConfig {
            work_ms: 25 * MIN,
            short_ms: 5 * MIN,
            long_ms: 15 * MIN,
            long_every: 4,
            auto_start_breaks: true,
            auto_start_work: false,
        }
    }

    #[test]
    fn countdown_finishes_at_deadline_even_after_sleep() {
        let mut t = Countdown::start("a".into(), 5 * MIN, None, 0);
        assert!(!t.check(4 * MIN));
        // Machine slept: next check is 10 min later, the timer still ended at 5:00.
        assert!(t.check(15 * MIN));
        assert_eq!(t.state, CountdownState::Ringing { ended_at: 5 * MIN });
        assert!(!t.check(16 * MIN));
    }

    #[test]
    fn pause_resume_keeps_remaining_time() {
        let mut t = Countdown::start("a".into(), 10 * MIN, None, 0);
        t.pause(3 * MIN);
        assert_eq!(t.remaining(50 * MIN), 7 * MIN);
        t.resume(50 * MIN);
        assert_eq!(t.deadline(), Some(57 * MIN));
    }

    #[test]
    fn add_minute_extends_running_and_restarts_ringing() {
        let mut t = Countdown::start("a".into(), MIN, None, 0);
        t.add(MIN as i64, 10);
        assert_eq!(t.deadline(), Some(2 * MIN));
        t.check(3 * MIN);
        t.add(MIN as i64, 3 * MIN);
        assert_eq!(t.deadline(), Some(4 * MIN));
    }

    #[test]
    fn stopwatch_accumulates_and_laps() {
        let mut s = Stopwatch::default();
        s.start(1_000);
        s.lap(3_000);
        s.pause(4_000);
        assert_eq!(s.elapsed(99_000), 3_000);
        s.start(10_000);
        assert_eq!(s.elapsed(11_000), 4_000);
        assert_eq!(s.laps, vec![2_000]);
    }

    #[test]
    fn pomodoro_cycle_long_break_every_fourth() {
        let c = cfg();
        let mut p = Pomodoro::start(&c, 0);
        let mut now = 0;
        let mut phases = Vec::new();
        for _ in 0..8 {
            now = p.deadline().unwrap_or(now);
            let end = p.check(&c, now).expect("phase end");
            phases.push(end.next);
            p.start_waiting(now);
        }
        use Phase::{LongBreak as L, ShortBreak as S, Work as W};
        assert_eq!(phases, vec![S, W, S, W, S, W, L, W]);
    }

    #[test]
    fn breaks_auto_start_but_work_waits() {
        let c = cfg();
        let mut p = Pomodoro::start(&c, 0);
        let end = p.check(&c, 25 * MIN).expect("work ends");
        assert!(end.auto_started);
        assert!(matches!(p.run, PomodoroRun::Running { .. }));
        let end = p.check(&c, 30 * MIN).expect("break ends");
        assert!(!end.auto_started);
        assert_eq!(p.run, PomodoroRun::Waiting);
    }

    #[test]
    fn skipping_work_does_not_count_it() {
        let c = cfg();
        let mut p = Pomodoro::start(&c, 0);
        let end = p.advance(&c, MIN, false);
        assert!(!end.completed);
        assert_eq!(p.completed_work, 0);
        assert_eq!(p.phase, Phase::ShortBreak);
    }
}
