import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { timerCommands } from '$lib/ipc/commands';
import { live } from '$lib/stores/live.svelte';
import { registerTimer } from '..';
import Launcher from '../../../apps/island/Launcher.svelte';

afterEach(() => { live.timer = null; vi.restoreAllMocks(); });

it('opens stopwatch controls without starting, and reflects start/pause/reset', async () => {
  registerTimer();
  const command = vi.spyOn(timerCommands, 'stopwatch').mockResolvedValue();
  live.timer = { timers: [], stopwatch: { accumulatedMs: 0, startedAt: null, laps: [] }, pomodoro: null, focusManual: false, focusActive: false, rev: 1 };
  const target = document.createElement('div');
  const component = mount(Launcher, { target });
  flushSync();
  const button = (text: string) => [...target.querySelectorAll('button')].find((b) => b.textContent?.trim() === text)!;
  button('Bấm giờ').click(); flushSync();
  expect(command).not.toHaveBeenCalled();
  expect(target.querySelector('.stopwatch')).not.toBeNull();
  button('Bắt đầu').click();
  expect(command).toHaveBeenLastCalledWith('start');
  live.timer.stopwatch.startedAt = Date.now(); flushSync();
  button('Tạm dừng').click();
  expect(command).toHaveBeenLastCalledWith('pause');
  button('Đặt lại').click();
  expect(command).toHaveBeenLastCalledWith('reset');
  await unmount(component);
});
