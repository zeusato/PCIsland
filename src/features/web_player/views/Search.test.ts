import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { webPlayerCommands } from '$lib/ipc/commands';
import Search from './Search.svelte';
afterEach(() => vi.restoreAllMocks());

it('searches in the background and plays only the chosen result', async () => {
  const search = vi.spyOn(webPlayerCommands, 'search').mockResolvedValue([{ title: 'A song', url: 'https://zingmp3.vn/bai-hat/a/1.html' }]);
  const play = vi.spyOn(webPlayerCommands, 'playResult').mockResolvedValue();
  const open = vi.spyOn(webPlayerCommands, 'open').mockResolvedValue();
  const onplay = vi.fn();
  const target = document.createElement('div');
  const component = mount(Search, { target, props: { onplay } });
  const input = target.querySelector('input')!;
  input.value = ' A song '; input.dispatchEvent(new Event('input', { bubbles: true })); flushSync();
  target.querySelector('form')!.dispatchEvent(new Event('submit', { cancelable: true }));
  await vi.waitFor(() => { flushSync(); expect(target.querySelector('.result')).not.toBeNull(); });
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
