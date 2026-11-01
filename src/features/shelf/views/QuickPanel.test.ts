import { expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { registerShelf } from '../index';
import { live } from '$lib/stores/live.svelte';
import { quickActions } from '$lib/registry/quickActions.svelte';
import { dragFilesOut } from '$lib/ipc/drag';
import Launcher from '../../../apps/island/Launcher.svelte';

vi.mock('$lib/ipc/drag', () => ({ dragFilesOut: vi.fn(async () => {}) }));

it('opens the saved file shelf from the launcher while another activity owns the island and allows drag out', async () => {
  live.shelf = [{ id: 'saved', path: 'D:\\saved.txt', name: 'saved.txt', size: 12, isDir: false, addedAt: 1, missing: false, thumbnailUrl: null }];
  registerShelf();
  expect(quickActions().find((a) => a.id === 'shelf.open')?.status?.()).toBe('1 file');
  const target = document.createElement('div');
  const component = mount(Launcher, { target }); flushSync();
  [...target.querySelectorAll('button')].find((b) => b.textContent?.includes('Kệ file'))!.click(); flushSync();
  expect(target.textContent).toContain('saved.txt');
  target.querySelector('[aria-label="Kéo mục đã chọn: saved.txt"]')!.dispatchEvent(new MouseEvent('mousedown', { button: 0, bubbles: true }));
  expect(dragFilesOut).toHaveBeenCalledExactlyOnceWith(['D:\\saved.txt']);
  await unmount(component); live.shelf = [];
});
