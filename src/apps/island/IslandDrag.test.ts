import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { islandCommands, mediaCommands, shelfCommands, webPlayerCommands } from '$lib/ipc/commands';
import type { DragDrop } from '$lib/ipc/events';
import type { VisibilityEvent } from '$lib/ipc/bindings/VisibilityEvent';
import { live } from '$lib/stores/live.svelte';
import { media } from '../../features/media/store.svelte';
import IslandRoot from './IslandRoot.svelte';

const handlers = vi.hoisted(() => ({ drag: (_e: DragDrop) => {}, preview: (_v: boolean) => {}, visibility: (_v: VisibilityEvent) => {} }));
vi.mock('$lib/ipc/commands', async (original) => ({ ...await original<object>(), inTauri: true }));
vi.mock('$lib/ipc/events', () => ({
  onDragDrop: async (fn: typeof handlers.drag) => { handlers.drag = fn; return () => {}; },
  onIslandDragHover: async (fn: typeof handlers.preview) => { handlers.preview = fn; return () => {}; },
  onIslandVisibility: async (fn: typeof handlers.visibility) => { handlers.visibility = fn; return () => {}; },
  onIslandPointer: async () => () => {}, onIslandHotkey: async () => () => {}, onWebPlayerState: async () => () => {},
}));
vi.mock('$lib/motion/transitions', () => ({ contentIn: () => ({ duration: 0 }), contentOut: () => ({ duration: 0 }) }));
vi.mock('svelte/motion', () => ({ Spring: class { constructor(public current: unknown) {} async set(value: unknown) { this.current = value; } } }));

beforeEach(() => {
  vi.stubGlobal('matchMedia', () => ({ matches: true }));
  vi.spyOn(islandCommands, 'setHitRegions').mockResolvedValue();
  vi.spyOn(islandCommands, 'requestFocus').mockResolvedValue();
  vi.spyOn(islandCommands, 'releaseFocus').mockResolvedValue();
  vi.spyOn(shelfCommands, 'add').mockResolvedValue(1);
  vi.spyOn(mediaCommands, 'playPause').mockResolvedValue();
  vi.spyOn(islandCommands, 'openMain').mockResolvedValue();
  vi.spyOn(webPlayerCommands, 'getState').mockResolvedValue({ repeat: 'off', repeatError: false, lifecycle: 'hidden', siteId: 'zing_mp3', url: null, rev: 1, queue: [], history: [] });
  media.snapshot = { rev: 1, sources: [], nowPlaying: { sourceId: 'web', appName: 'Zing MP3', title: 'Playing track', artist: null, album: null, artworkUrl: null, status: 'playing', positionMs: 1000, positionAt: Date.now(), durationMs: 100000, rate: 1, caps: { playPause: true, next: true, previous: true, seek: true }, isAd: false } };
  live.island = { primary: { id: 'music', source: 'media', kind: 'media.now_playing', priority: 50, transientMs: null, payload: media.snapshot.nowPlaying, actions: [], placement: 'auto', startedAt: 1 }, secondary: null, alert: null, criticalWaiting: 0, pinnedId: null, focusActive: false, rev: 1 };
});
afterEach(() => { live.island = null; media.snapshot = null; vi.restoreAllMocks(); vi.unstubAllGlobals(); });
async function settle() { for (let i = 0; i < 10; i++) { await Promise.resolve(); flushSync(); } }

it.each(['compact', 'peek', 'minimal'] as const)('receives files over playing music in %s, survives morph leave, and keeps the shelf after drop', async (mode) => {
  const target = document.createElement('div'); document.body.append(target);
  const component = mount(IslandRoot, { target }); await settle();
  if (mode === 'peek') (target.querySelector('.island') as HTMLElement).click();
  if (mode === 'minimal') handlers.visibility({ mode: 'minimal', reason: 'test', hasLiveActivity: true, rev: 1 });
  await settle();
  handlers.preview(true); await settle();
  expect(target.querySelector('.drop')).not.toBeNull();
  expect(target.querySelector('.island.minimal')).toBeNull();
  handlers.drag({ type: 'enter', paths: ['D:\\sample.txt'], x: 240, y: 20 });
  handlers.drag({ type: 'leave' }); await settle();
  expect(target.querySelector('.drop')).not.toBeNull();
  handlers.preview(false); await settle();
  expect(target.querySelector('.drop')).toBeNull();
  handlers.drag({ type: 'over', paths: ['D:\\sample.txt'], x: 240, y: 20 }); await settle();
  handlers.drag({ type: 'drop', paths: ['D:\\sample.txt'], x: 240, y: 20 }); handlers.preview(false); await settle();
  expect(shelfCommands.add).toHaveBeenCalledExactlyOnceWith(['D:\\sample.txt']);
  expect(target.querySelector('.drop')).not.toBeNull();
  expect(mediaCommands.playPause).not.toHaveBeenCalled();
  await unmount(component); target.remove();
});

it('keeps the player and search query while using features, returns without restarting music, and ignores double-click', async () => {
  const target = document.createElement('div'); document.body.append(target);
  const component = mount(IslandRoot, { target }); await settle();
  const island = target.querySelector('.island') as HTMLElement;
  island.click(); await settle();
  const button = (label: string) => [...target.querySelectorAll('button')].find((b) => b.textContent?.trim() === label)!;
  button('Tìm').click(); await settle();
  const input = target.querySelector('input')!;
  input.value = 'đại thiên bồng'; input.dispatchEvent(new Event('input', { bubbles: true })); await settle();
  (target.querySelector('.switcher:not(.return-player)') as HTMLElement).click(); await settle();
  expect(target.querySelector('.player-content')?.hasAttribute('hidden')).toBe(true);
  (target.querySelector('.return-player') as HTMLElement).click(); await settle();
  expect(target.querySelector('.player-content')?.hasAttribute('hidden')).toBe(false);
  expect(target.querySelector('input')).toBe(input);
  expect(input.value).toBe('đại thiên bồng');
  expect(mediaCommands.playPause).not.toHaveBeenCalled();
  island.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
  expect(islandCommands.openMain).not.toHaveBeenCalled();
  await unmount(component); target.remove();
});
