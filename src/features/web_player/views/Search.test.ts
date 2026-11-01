import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { webPlayerCommands } from '$lib/ipc/commands';
import Search from './Search.svelte';
afterEach(() => vi.restoreAllMocks());

it('stays on results while playback is unconfirmed and reports a failure without showing the old player', async () => {
  vi.spyOn(webPlayerCommands, 'search').mockResolvedValue([{ title: 'Chosen track', url: 'https://zingmp3.vn/bai-hat/a/1.html' }]);
  let reject!: (reason: Error) => void;
  const play = vi.spyOn(webPlayerCommands, 'playResult').mockReturnValue(new Promise((_, fail) => reject = fail));
  const onplay = vi.fn();
  const target = document.createElement('div'); const component = mount(Search, { target, props: { onplay } });
  const input = target.querySelector('input')!; input.value = 'test'; input.dispatchEvent(new Event('input', { bubbles: true })); flushSync();
  target.querySelector('form')!.dispatchEvent(new Event('submit', { cancelable: true }));
  await vi.waitFor(() => { flushSync(); expect(target.querySelector('.result')).not.toBeNull(); });
  (target.querySelector('.result') as HTMLButtonElement).click(); flushSync();
  expect(play).toHaveBeenCalledOnce(); expect(onplay).not.toHaveBeenCalled();
  expect(target.querySelector('[role="status"]')?.textContent).toContain('Chosen track');
  reject(new Error('not playing'));
  await vi.waitFor(() => { flushSync(); expect(target.querySelector('[role="alert"]')).not.toBeNull(); });
  expect(onplay).not.toHaveBeenCalled();
  expect(target.querySelector<HTMLButtonElement>('.result')?.disabled).toBe(false);
  await unmount(component);
});

it('searches in the background and plays only the chosen result', async () => {
  const search = vi.spyOn(webPlayerCommands, 'search').mockResolvedValue([{ title: 'A song', artist: 'Ca sĩ A', uploader: 'Kênh B', url: 'https://zingmp3.vn/bai-hat/a/1.html' }]);
  const play = vi.spyOn(webPlayerCommands, 'playResult').mockResolvedValue();
  const open = vi.spyOn(webPlayerCommands, 'open').mockResolvedValue();
  const onplay = vi.fn();
  const target = document.createElement('div');
  const component = mount(Search, { target, props: { onplay } });
  const input = target.querySelector('input')!;
  input.value = ' A song '; input.dispatchEvent(new Event('input', { bubbles: true })); flushSync();
  target.querySelector('form')!.dispatchEvent(new Event('submit', { cancelable: true }));
  await vi.waitFor(() => { flushSync(); expect(target.querySelector('.result')).not.toBeNull(); });
  expect(target.querySelector('.result')?.textContent).toContain('Ca sĩ: Ca sĩ A');
  expect(target.querySelector('.result')?.textContent).toContain('Người đăng: Kênh B');
  (target.querySelector('.result') as HTMLButtonElement).click();
  await vi.waitFor(() => expect(onplay).toHaveBeenCalledOnce());
  expect(search).toHaveBeenCalledWith('zing_mp3', 'A song');
  expect(play).toHaveBeenCalledWith('zing_mp3', 'https://zingmp3.vn/bai-hat/a/1.html');
  expect(open).not.toHaveBeenCalled();
  await unmount(component);
});

it('shows a recoverable error without opening the service automatically', async () => {
  vi.spyOn(webPlayerCommands, 'search').mockRejectedValue(new Error('unavailable'));
  const open = vi.spyOn(webPlayerCommands, 'open').mockResolvedValue();
  const target = document.createElement('div');
  const component = mount(Search, { target });
  const input = target.querySelector('input')!;
  input.value = 'test'; input.dispatchEvent(new Event('input', { bubbles: true })); flushSync();
  target.querySelector('form')!.dispatchEvent(new Event('submit', { cancelable: true }));
  await vi.waitFor(() => { flushSync(); expect(target.querySelector('[role="alert"]')).not.toBeNull(); });
  expect(open).not.toHaveBeenCalled();
  await unmount(component);
});
