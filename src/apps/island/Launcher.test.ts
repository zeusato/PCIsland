import { expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { live } from '$lib/stores/live.svelte';
import { appCommands, timerCommands } from '$lib/ipc/commands';
import { registerTimer } from '../../features/timer';
import Launcher from './Launcher.svelte';

it('shows Pomodoro state on the launcher button and feedback after starting', async () => {
  registerTimer();
  const command = vi.spyOn(timerCommands, 'pomodoro').mockResolvedValue();
  const toast = vi.spyOn(appCommands, 'toast').mockResolvedValue();
  const target = document.createElement('div');
  const component = mount(Launcher, { target });
  flushSync();
  const button = [...target.querySelectorAll('button')].find((b) => b.textContent?.includes('Pomodoro'))!;
  expect(button.getAttribute('aria-pressed')).toBe('false');
  button.click();
  await Promise.resolve();
  flushSync();
  expect(command).toHaveBeenCalledWith('start');
  await vi.waitFor(() => expect(toast).toHaveBeenCalledWith('Pomodoro', expect.stringContaining('Không thay đổi thông báo Windows')));
  live.timer = { timers: [], stopwatch: { accumulatedMs: 0, startedAt: null, laps: [] }, focusManual: false, focusActive: true, rev: 1,
    pomodoro: { phase: 'work', completedWork: 0, phaseMs: 1500000, phaseStartedAt: 0, run: { kind: 'running', endsAt: 1500000 } } };
  flushSync();
  expect(button.dataset.state).toBe('active');
  expect(button.textContent).toContain('Tập trung');
  live.timer.pomodoro!.phase = 'shortBreak';
  flushSync();
  expect(button.dataset.state).toBe('break');
  expect(button.textContent).toContain('Nghỉ ngắn');
  live.timer.pomodoro!.run = { kind: 'paused', remainingMs: 5000 };
  flushSync();
  expect(button.dataset.state).toBe('paused');
  expect(button.textContent).toContain('Tạm dừng');
  await unmount(component);
  live.timer = null;
  vi.restoreAllMocks();
});
