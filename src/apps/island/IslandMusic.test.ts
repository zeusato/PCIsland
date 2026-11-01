import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { islandCommands, mediaCommands, webPlayerCommands } from '$lib/ipc/commands';
import { live } from '$lib/stores/live.svelte';
import { media } from '../../features/media/store.svelte';
import { registerWebPlayer } from '../../features/web_player';
import IslandRoot from './IslandRoot.svelte';

vi.mock('$lib/ipc/commands', async (original) => ({ ...await original<object>(), inTauri: true }));
vi.mock('$lib/ipc/events', () => Object.fromEntries(['onWebPlayerState', 'onIslandVisibility', 'onIslandPointer', 'onIslandHotkey', 'onDragDrop'].map((name) => [name, async () => () => {}])));
vi.mock('$lib/motion/transitions', () => ({ contentIn: () => ({ duration: 0 }), contentOut: () => ({ duration: 0 }) }));
vi.mock('svelte/motion', () => ({ Spring: class { constructor(public current: unknown) {} async set(value: unknown) { this.current = value; } } }));
afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); live.island = null; media.snapshot = null; });

it('fits site options, resumes straight to the player before metadata arrives, and grows for the queue', async () => {
  const observers = new Map<Element, () => void>();
  vi.stubGlobal('ResizeObserver', class {
    node?: Element;
    constructor(private callback: () => void) {}
    observe(node: Element) { this.node = node; observers.set(node, this.callback); }
    disconnect() { if (this.node) observers.delete(this.node); }
  });
  vi.stubGlobal('matchMedia', () => ({ matches: true }));
  let height = 220;
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(() => ({ height } as DOMRect));
  const hits = vi.spyOn(islandCommands, 'setHitRegions').mockResolvedValue();
  vi.spyOn(mediaCommands, 'selectSource').mockResolvedValue();
  vi.spyOn(webPlayerCommands, 'listSites').mockResolvedValue([{ id: 'zing_mp3', name: 'Zing MP3', homeUrl: 'https://zingmp3.vn/' }]);
  const snapshot = { lifecycle: 'closed' as const, siteId: null, url: null, rev: 1, queue: Array.from({ length: 20 }, (_, i) => ({ title: `Song ${i}`, url: `https://zingmp3.vn/bai-hat/${i}` })), history: [{ siteId: 'zing_mp3', title: 'Saved track', pageUrl: 'https://zingmp3.vn/album/saved', trackUrl: null, positionMs: 1000, queue: [] }] };
  vi.spyOn(webPlayerCommands, 'getState').mockResolvedValue(snapshot);
  vi.spyOn(webPlayerCommands, 'requestQueue').mockResolvedValue();
  const resume = vi.spyOn(webPlayerCommands, 'resume').mockRejectedValueOnce(new Error('offline')).mockResolvedValue();
  registerWebPlayer();
  const target = document.createElement('div');
  document.body.append(target);
  const component = mount(IslandRoot, { target });
  const button = (text: string) => [...target.querySelectorAll('button')].find((node) => node.textContent?.trim() === text)!;
  const measure = () => { observers.forEach((callback) => callback()); flushSync(); };
  try {
    flushSync(); (target.querySelector('.island') as HTMLElement).click(); flushSync();
    button('Nhạc').click(); flushSync();
    await vi.waitFor(() => { flushSync(); expect(button('Zing MP3')).toBeDefined(); });
    button('Zing MP3').click(); flushSync();
    height = 320; measure();
    await vi.waitFor(() => expect(hits.mock.calls.some(([rects]) => rects[0]?.height === 348)).toBe(true));
    button('Chạy tiếp').click();
    await vi.waitFor(() => { flushSync(); expect(target.querySelector('[role="alert"]')).not.toBeNull(); });
    expect(button('Chạy tiếp')).toBeDefined();
    button('Chạy tiếp').click();
    await vi.waitFor(() => { flushSync(); expect(target.querySelector('.launcher')).toBeNull(); });
    expect(resume).toHaveBeenCalledTimes(2);
    expect(target.querySelector('[role="status"]')).not.toBeNull();
    media.snapshot = { sources: [], rev: 1, nowPlaying: { sourceId: 'web', appName: 'Zing MP3', title: 'Resumed track', artist: null, album: null, artworkUrl: null, status: 'paused', positionMs: 1000, positionAt: Date.now(), durationMs: 100000, rate: 1, caps: { playPause: true, next: true, previous: true, seek: true }, isAd: false } };
    flushSync();
    expect(target.textContent).toContain('Resumed track');
    button('Danh sách phát').click(); flushSync();
    await vi.waitFor(() => { flushSync(); expect(target.textContent).toContain('Song 19'); });
    height = 390; measure();
    await vi.waitFor(() => expect(hits.mock.calls.some(([rects]) => rects[0]?.height === 418)).toBe(true));
    button('Quay lại · Danh sách phát').click(); flushSync();
    await vi.waitFor(() => expect(hits.mock.calls.at(-1)?.[0][0]?.height).toBe(200));
    expect(target.textContent).toContain('Resumed track');
  } finally { await unmount(component); target.remove(); }
});
