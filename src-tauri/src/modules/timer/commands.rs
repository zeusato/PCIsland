use tauri::State;

use super::logic::Countdown;
use super::{Cmd, PomodoroStats, TimerHandle, TimerSnapshot};
use crate::ipc::{IpcError, IpcResult};

#[tauri::command]
pub async fn timer_get_state(timer: State<'_, TimerHandle>) -> IpcResult<TimerSnapshot> {
    Ok(timer.snapshot().await)
}

#[tauri::command]
pub async fn timer_start(
    timer: State<'_, TimerHandle>,
    duration_ms: u64,
    label: Option<String>,
) -> IpcResult<Countdown> {
    let label = label
        .map(|l| l.chars().take(80).collect::<String>())
        .filter(|l| !l.trim().is_empty());
    timer
        .start(duration_ms, label)
        .await
        .ok_or_else(|| IpcError::new("timer.unavailable", "timer service stopped"))
}

#[tauri::command]
pub fn timer_control(timer: State<'_, TimerHandle>, id: String, action: String) -> IpcResult<()> {
    let cmd = match action.as_str() {
        "pause" => Cmd::Pause(id),
        "resume" => Cmd::Resume(id),
        "cancel" => Cmd::Cancel(id),
        "dismiss" => Cmd::Dismiss(id),
        "repeat" => Cmd::Repeat(id),
        "add_minute" => Cmd::Add(id, 60_000),
        _ => return Err(IpcError::new("timer.bad_action", action)),
    };
    timer.send(cmd);
    Ok(())
}

#[tauri::command]
pub fn stopwatch_control(timer: State<'_, TimerHandle>, action: String) -> IpcResult<()> {
    timer.send(match action.as_str() {
        "start" => Cmd::StopwatchStart,
        "pause" => Cmd::StopwatchPause,
        "reset" => Cmd::StopwatchReset,
        "lap" => Cmd::StopwatchLap,
        _ => return Err(IpcError::new("timer.bad_action", action)),
    });
    Ok(())
}

#[tauri::command]
pub fn pomodoro_control(timer: State<'_, TimerHandle>, action: String) -> IpcResult<()> {
    timer.send(match action.as_str() {
        "start" => Cmd::PomodoroStart,
        "pause" => Cmd::PomodoroPause,
        "resume" => Cmd::PomodoroResume,
        "skip" => Cmd::PomodoroSkip,
        "stop" => Cmd::PomodoroStop,
        "next" => Cmd::PomodoroNext,
        _ => return Err(IpcError::new("timer.bad_action", action)),
    });
    Ok(())
}

#[tauri::command]
pub fn pomodoro_stats(timer: State<'_, TimerHandle>, since: u64) -> PomodoroStats {
    timer.stats(since)
}

#[tauri::command]
pub fn focus_set(timer: State<'_, TimerHandle>, active: bool) {
    timer.send(Cmd::Focus(active));
}
