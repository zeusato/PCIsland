import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { live } from '$lib/stores/live.svelte';
import { playerCommands, webPlayerCommands } from '$lib/ipc/commands';
import type { WebPlayerSnapshot } from '$lib/ipc/bindings/WebPlayerSnapshot';
import RepeatControl from './RepeatControl.svelte';

vi.mock('$lib/ipc/commands', async (original) => ({ ...await original<object>(), inTauri: true }));
const events = vi.hoisted(() => ({ update: (_state: WebPlayerSnapshot) => {} }));
vi.mock('$lib/ipc/events', () => ({ onWebPlayerState: async (callback: typeof events.update) => { events.update = callback; return () => {}; } }));
afterEach(() => { live.player = null; vi.restoreAllMocks(); });

it('selects local repeat one, list, or off and waits for backend state', async () => {
  live.player = { queue: [], index: null, playing: false, positionMs: 0, positionAt: 0, durationMs: null, volume: 1, shuffle: false, repeat: 'off', rev: 1 };
  const control = vi.spyOn(playerCommands, 'control').mockResolvedValue();
  const target = document.createElement('div');
  const component = mount(RepeatControl, { target, props: { source: 'local' } });
  flushSync();
  try {
    for (const mode of ['one', 'all', 'off'] as const) {
      target.querySelector('button')!.click(); flushSync();
      target.querySelector<HTMLButtonElement>(`[data-mode="${mode}"]`)!.click(); flushSync();
      expect(control).toHaveBeenLastCalledWith(`repeat_${mode}`);
      expect(target.querySelector('button')!.disabled).toBe(true);
      live.player!.repeat = mode; flushSync();
      expect(target.querySelector('button')!.disabled).toBe(false);
    }
  } finally { await unmount(component); }
});

it('uses the web player acknowledgement and reports unsupported repeat without a fake mode change', async () => {
  const state: WebPlayerSnapshot = { lifecycle: 'hidden', siteId: 'youtube', url: null, history: [], queue: [], repeat: null, repeatError: false, rev: 1 };
  vi.spyOn(webPlayerCommands, 'getState').mockResolvedValue(state);
  const set = vi.spyOn(webPlayerCommands, 'setRepeat').mockResolvedValue();
  const target = document.createElement('div');
  const component = mount(RepeatControl, { target, props: { source: 'web' } });
  try {
    await vi.waitFor(() => { flushSync(); expect(webPlayerCommands.getState).toHaveBeenCalled(); });
    target.querySelector('button')!.click(); flushSync();
    target.querySelector<HTMLButtonElement>('[data-mode="one"]')!.click(); flushSync();
    expect(set).toHaveBeenCalledWith('one');
    expect(target.textContent).toContain('Không lặp');
    events.update({ ...state, repeat: 'one', rev: 2 }); flushSync();
    expect(target.textContent).toContain('Lặp 1 bài');
    target.querySelector('button')!.click(); flushSync();
    target.querySelector<HTMLButtonElement>('[data-mode="all"]')!.click(); flushSync();
    events.update({ ...state, repeat: 'one', repeatError: true, rev: 3 }); flushSync();
    expect(target.querySelector('[role="status"]')).not.toBeNull();
    expect(target.textContent).toContain('Lặp 1 bài');
    target.querySelector('button')!.click(); flushSync();
    target.querySelector<HTMLButtonElement>('[data-mode="off"]')!.click(); flushSync();
    expect(set).toHaveBeenLastCalledWith('off');
  } finally { await unmount(component); }
});
