import { afterEach, expect, it, vi } from 'vitest';
import { live } from '$lib/stores/live.svelte';
import { feedback } from '$lib/stores/feedback.svelte';
import { timerCommands } from '$lib/ipc/commands';
import type { Settings } from '$lib/ipc/bindings/Settings';
import { pomodoroStartMessage, pomodoroState, quickPomodoro } from './pomodoro.svelte';

afterEach(() => { live.timer = null; live.settings = null; feedback.message = ''; vi.restoreAllMocks(); });

it('describes the configured focus, Windows notifications and music behavior', () => {
  live.settings = { pomodoro: { workMin: 45, focusWithPomodoro: false, pauseMediaOnBreak: true, focusPlaylistId: 'list-1' } } as Settings;
  expect(pomodoroStartMessage()).toContain('45 phút');
  expect(pomodoroStartMessage()).toContain('Focus tắt');
  expect(pomodoroStartMessage()).toContain('Không thay đổi thông báo Windows');
  expect(pomodoroStartMessage()).toContain('tạm dừng nhạc');
  expect(pomodoroStartMessage()).toContain('playlist');
  live.timer = { timers: [], stopwatch: { accumulatedMs: 0, startedAt: null, laps: [] }, pomodoro: null, focusManual: true, focusActive: true, rev: 1 };
  expect(pomodoroStartMessage()).toContain('Focus bật');
});

it('starts with feedback, then pauses/resumes the existing session instead of restarting it', async () => {
  const command = vi.spyOn(timerCommands, 'pomodoro').mockResolvedValue();
  await quickPomodoro();
  expect(command).toHaveBeenLastCalledWith('start');
  expect(feedback.message).toContain('25 phút');
  live.timer = { timers: [], stopwatch: { accumulatedMs: 0, startedAt: null, laps: [] }, focusManual: false, focusActive: true, rev: 1,
    pomodoro: { phase: 'work', completedWork: 0, phaseMs: 1500000, phaseStartedAt: 0, run: { kind: 'running', endsAt: 1500000 } } };
  expect(pomodoroState()).toBe('active');
  await quickPomodoro();
  expect(command).toHaveBeenLastCalledWith('pause');
  live.timer.pomodoro!.run = { kind: 'paused', remainingMs: 60000 };
  expect(pomodoroState()).toBe('paused');
  await quickPomodoro();
  expect(command).toHaveBeenLastCalledWith('resume');
  live.timer.pomodoro!.phase = 'shortBreak';
  live.timer.pomodoro!.run = { kind: 'running', endsAt: 100000 };
  expect(pomodoroState()).toBe('break');
  live.timer.pomodoro!.run = { kind: 'waiting' };
  await quickPomodoro();
  expect(command).toHaveBeenLastCalledWith('next');
});
