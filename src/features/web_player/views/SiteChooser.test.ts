import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { webPlayerCommands } from '$lib/ipc/commands';
import type { WebPlayerSnapshot } from '$lib/ipc/bindings/WebPlayerSnapshot';
import SiteChooser from './SiteChooser.svelte';
import { media } from '../../media/store.svelte';

vi.mock('$lib/ipc/commands', async (original) => ({ ...await original<object>(), inTauri: true }));
vi.mock('$lib/ipc/events', () => ({ onWebPlayerState: async () => () => {} }));
afterEach(() => { vi.restoreAllMocks(); media.snapshot = null; });

it('offers resume or homepage for saved sites and opens new sites directly', async () => {
  vi.spyOn(webPlayerCommands, 'listSites').mockResolvedValue([
    { id: 'zing_mp3', name: 'Zing MP3', homeUrl: 'https://zingmp3.vn/' },
    { id: 'youtube', name: 'YouTube', homeUrl: 'https://www.youtube.com/' },
  ]);
  const state: WebPlayerSnapshot = { lifecycle: 'closed', siteId: null, url: null, rev: 0, queue: [], history: [{ siteId: 'zing_mp3', pageUrl: 'https://zingmp3.vn/album/test', title: 'Last track', trackUrl: null, positionMs: 12000, queue: [] }] };
  vi.spyOn(webPlayerCommands, 'getState').mockResolvedValue(state);
  const open = vi.spyOn(webPlayerCommands, 'open').mockResolvedValue();
  const resume = vi.spyOn(webPlayerCommands, 'resume').mockResolvedValue();
  const target = document.createElement('div');
  const component = mount(SiteChooser, { target });
  const button = (text: string) => [...target.querySelectorAll('button')].find((b) => b.textContent === text)!;
  await vi.waitFor(() => { flushSync(); expect(button('Zing MP3')).toBeDefined(); });
  button('Zing MP3').click(); flushSync();
  expect(open).not.toHaveBeenCalled();
  expect(target.textContent).toContain('Last track');
  button('Chạy tiếp').click();
  await vi.waitFor(() => expect(resume).toHaveBeenCalledWith('zing_mp3')); flushSync();
  await vi.waitFor(() => { flushSync(); expect(button('Mở trang gốc').disabled).toBe(false); });
  button('Mở trang gốc').click();
  await vi.waitFor(() => expect(open).toHaveBeenCalledWith('zing_mp3')); flushSync();
  await vi.waitFor(() => { flushSync(); expect(button('YouTube').disabled).toBe(false); });
  button('YouTube').click();
  await vi.waitFor(() => expect(open).toHaveBeenLastCalledWith('youtube'));
  await unmount(component);
});

it('shows the active site on island and reveals site choices only after Change site', async () => {
  vi.spyOn(webPlayerCommands, 'listSites').mockResolvedValue([
    { id: 'zing_mp3', name: 'Zing MP3', homeUrl: 'https://zingmp3.vn/' },
    { id: 'soundcloud', name: 'SoundCloud', homeUrl: 'https://soundcloud.com/' },
  ]);
  vi.spyOn(webPlayerCommands, 'getState').mockResolvedValue({ lifecycle: 'hidden', siteId: 'zing_mp3', url: 'https://zingmp3.vn/album/saved', rev: 1, queue: [], history: [{ siteId: 'zing_mp3', pageUrl: 'https://zingmp3.vn/album/saved', title: 'Saved song', trackUrl: null, positionMs: 42000, queue: [] }] });
  media.snapshot = { nowPlaying: null, sources: [{ sourceId: 'web', appName: 'Zing MP3', status: 'playing' }], rev: 1 };
  const show = vi.spyOn(webPlayerCommands, 'show').mockResolvedValue();
  const open = vi.spyOn(webPlayerCommands, 'open').mockResolvedValue();
  const target = document.createElement('div');
  const component = mount(SiteChooser, { target, props: { island: true } });
  const button = (text: string) => [...target.querySelectorAll('button')].find((b) => b.textContent === text);
  await vi.waitFor(() => { flushSync(); expect(target.textContent).toContain('Saved song'); });
  expect(target.textContent).toContain('Đang phát');
  expect(button('SoundCloud')).toBeUndefined();
  button('Mở trang')!.click(); await vi.waitFor(() => expect(show).toHaveBeenCalledOnce());
  expect(open).not.toHaveBeenCalled();
  button('Đổi trang')!.click(); flushSync();
  expect(button('SoundCloud')).toBeDefined();
  await unmount(component);
});
