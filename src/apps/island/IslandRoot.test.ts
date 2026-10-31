import { expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { live } from '$lib/stores/live.svelte';
import { islandCommands, mediaCommands, noteCommands, reminderCommands } from '$lib/ipc/commands';
import { registerTimer } from '../../features/timer';
import { registerNotes } from '../../features/notes';
import { registerReminders } from '../../features/reminders';
import IslandRoot from './IslandRoot.svelte';

vi.hoisted(() => {
  Object.defineProperty(window, 'matchMedia', { configurable: true, value: () => ({ matches: false, addEventListener() {}, removeEventListener() {} }) });
});

it.each(['create', 'edit', 'check', 'reminder'])('keeps the hover-open island expanded while composing %s, then allows collapse after cancel', async (mode) => {
  vi.useFakeTimers();
  registerNotes();
  registerReminders();
  live.island = null;
  vi.spyOn(noteCommands, 'list').mockResolvedValue([{ id: 'note', text: 'Ghi chú đã lưu', pinned: false, createdAt: 0, updatedAt: 0 }]);
  vi.spyOn(islandCommands, 'requestFocus').mockResolvedValue();
  vi.spyOn(islandCommands, 'releaseFocus').mockResolvedValue();
  vi.spyOn(reminderCommands, 'parse').mockResolvedValue({ title: 'Nhắc việc', dueAt: null, repeat: { kind: 'none' } });
  const target = document.createElement('div');
  document.body.append(target);
  const component = mount(IslandRoot, { target });
  flushSync();
  const island = target.querySelector<HTMLElement>('.island')!;
  const button = (label: string) => [...target.querySelectorAll('button')].find((b) => b.textContent?.trim() === label)!;
  // Hover only: explicitly clicking the island would pin it and hide the regression.
  island.dispatchEvent(new Event('pointerenter'));
  await vi.advanceTimersByTimeAsync(300);
  flushSync();
  button(mode === 'reminder' ? 'Nhắc việc' : 'Ghi chú').click();
  await vi.advanceTimersByTimeAsync(0);
  flushSync();
  // A pending leave timeout must see an editor opened afterwards, too.
  island.dispatchEvent(new Event('pointerleave'));
  if (mode !== 'reminder') button({ create: 'Tạo mới', edit: 'Sửa', check: '+ Ô tick' }[mode]!).click();
  await vi.advanceTimersByTimeAsync(0);
  flushSync();
  const input = target.querySelector<HTMLInputElement | HTMLTextAreaElement>('input, textarea')!;
  expect(input).not.toBeNull();
  await vi.advanceTimersByTimeAsync(500);
  flushSync();
  expect(target.contains(input)).toBe(true);
  input.value = 'Nội dung đang soạn';
  input.dispatchEvent(new Event('input', { bubbles: true }));
  input.blur();
  island.dispatchEvent(new Event('pointerleave'));
  await vi.advanceTimersByTimeAsync(2000);
  flushSync();
  expect(target.contains(input)).toBe(true);
  expect(input.value).toBe('Nội dung đang soạn');
  if (mode === 'reminder') input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  else button('Hủy').click();
  flushSync();
  island.dispatchEvent(new Event('pointerleave'));
  await vi.advanceTimersByTimeAsync(500);
  flushSync();
  expect(target.querySelector('.launcher')).toBeNull();
  await unmount(component);
  target.remove();
  vi.restoreAllMocks();
  vi.useRealTimers();
});

vi.mock('$lib/motion/transitions', () => ({ contentIn: () => ({ duration: 0 }), contentOut: () => ({ duration: 0 }) }));

it('opens quick features from music and returns without opening main or stopping playback', async () => {
  registerTimer();
  const openMain = vi.spyOn(islandCommands, 'openMain');
  const playPause = vi.spyOn(mediaCommands, 'playPause');
  live.island = {
    primary: { id: 'media:now-playing', source: 'media', kind: 'media.now_playing', priority: 50, transientMs: null,
      payload: { sourceId: 'web', appName: 'Zing MP3', title: 'Test track', artist: 'Artist', album: null, artworkUrl: null, status: 'playing', positionMs: 1000, positionAt: Date.now(), durationMs: 10000, rate: 1, caps: { playPause: true, next: true, previous: true, seek: false }, isAd: false },
      actions: [], placement: 'auto', startedAt: 0 },
    secondary: null, alert: null, criticalWaiting: 0, pinnedId: null, focusActive: false, rev: 1,
  };
  const target = document.createElement('div');
  document.body.append(target);
  const component = mount(IslandRoot, { target });
  flushSync();
  expect(target.querySelector('[role="progressbar"]')).not.toBeNull();
  target.querySelector<HTMLElement>('.island')?.click();
  flushSync();
  const findButton = (text: string) => [...target.querySelectorAll('button')].find((button) => button.textContent?.trim() === text);
  expect(findButton('Tính năng')).toBeDefined();
  findButton('Tính năng')?.click();
  flushSync();
  expect(findButton('Hẹn giờ')).toBeDefined();
  findButton('Hẹn giờ')?.click();
  flushSync();
  expect(target.querySelector('[role="spinbutton"]')).not.toBeNull();
  findButton('Quay lại tác vụ')?.click();
  flushSync();
  expect(target.textContent).toContain('Test track');
  expect(target.querySelector('[role="spinbutton"]')).toBeNull();
  expect(openMain).not.toHaveBeenCalled();
  expect(playPause).not.toHaveBeenCalled();
  await unmount(component);
  target.remove();
  live.island = null;
  vi.restoreAllMocks();
});
