import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import Welcome from './Welcome.svelte';
import { focusSoundsCommands } from '$lib/ipc/commands';
import { videoCommands } from '$lib/ipc/video';
import { updateSettings } from '$lib/stores/live.svelte';

const pack = (id: string, role: 'focus' | 'break', state: 'notInstalled' | 'installed' = 'notInstalled') =>
  ({ id, role, name: { vi: id === 'lofi' ? 'Lo-fi' : 'Nghỉ', en: id }, tracks: 5, bytes: 30 * 1024 * 1024, state });

vi.mock('$lib/ipc/commands', () => ({ inTauri: true, focusSoundsCommands: {
  status: vi.fn(async () => ({ release: null, packs: [] })),
  catalog: vi.fn(async () => ({ release: 'focus-sounds-v1', license: '', packs: [pack('lofi', 'focus'), pack('rest', 'break')] })),
  download: vi.fn(async (ids: string[], onProgress: (p: object) => void) => {
    onProgress({ pack: 'lofi', file: 'Rainy Desk', doneBytes: 1, totalBytes: 2, doneFiles: 0, totalFiles: 2 });
    return { release: 'focus-sounds-v1', packs: ids.map((id) => ({ id, role: 'focus', name: { vi: id, en: id }, tracks: 5, bytes: 1, complete: true })) };
  }),
  cancel: vi.fn(), remove: vi.fn(),
} }));
vi.mock('$lib/ipc/video', () => ({ videoCommands: {
  toolsStatus: vi.fn(async () => ({ installed: false, version: null })),
  installTools: vi.fn(async () => ({ installed: true, version: '2026.10.26' })),
} }));
vi.mock('$lib/stores/live.svelte', () => ({ live: { settings: { pomodoro: { focusSoundPacks: [] } } }, updateSettings: vi.fn(async () => {}) }));

const settle = async () => { for (let i = 0; i < 10; i++) { await Promise.resolve(); flushSync(); } };
const button = (target: HTMLElement, text: string) => [...target.querySelectorAll('button')].find((b) => b.textContent?.includes(text))!;
afterEach(() => vi.clearAllMocks());

it('offers yt-dlp and every pack, downloads only after confirmation and enables the packs', async () => {
  const target = document.createElement('div');
  const component = mount(Welcome, { target }); await settle();
  expect(target.textContent).toContain('yt-dlp');
  expect(target.textContent).toContain('Lo-fi');
  expect(videoCommands.installTools).not.toHaveBeenCalled();
  expect(focusSoundsCommands.download).not.toHaveBeenCalled();
  // Untick the break pack; the button shows only the chosen music size.
  const rest = target.querySelectorAll<HTMLInputElement>('input[type=checkbox]')[2]!;
  rest.checked = false; rest.dispatchEvent(new Event('change', { bubbles: true })); await settle();
  expect(button(target, 'Tải đã chọn').textContent).toContain('30 MB');
  button(target, 'Tải đã chọn').click(); await settle();
  expect(videoCommands.installTools).toHaveBeenCalledOnce();
  expect(focusSoundsCommands.download).toHaveBeenCalledWith(['lofi'], expect.any(Function));
  expect(updateSettings).toHaveBeenCalledWith('pomodoro', { focusSoundPacks: ['lofi'] });
  expect(updateSettings).not.toHaveBeenCalledWith('app', { welcomeDone: true });
  await vi.waitFor(() => expect(button(target, 'Bắt đầu dùng')).toBeTruthy());
  expect(target.textContent).toContain('Đã tải xong');
  button(target, 'Bắt đầu dùng').click(); await settle();
  expect(updateSettings).toHaveBeenCalledWith('app', { welcomeDone: true });
  await unmount(component);
});

it('can be dismissed without downloading anything', async () => {
  const target = document.createElement('div');
  const component = mount(Welcome, { target }); await settle();
  button(target, 'Để sau').click(); await settle();
  expect(updateSettings).toHaveBeenCalledWith('app', { welcomeDone: true });
  expect(videoCommands.installTools).not.toHaveBeenCalled();
  expect(focusSoundsCommands.download).not.toHaveBeenCalled();
  await unmount(component);
});
