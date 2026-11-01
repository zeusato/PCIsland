import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import Panel from './Panel.svelte';
import { maintenanceCommands as api } from '$lib/ipc/commands';
import type { CleanupProgress } from '$lib/ipc/bindings/CleanupProgress';

vi.mock('$lib/ipc/commands', () => ({ maintenanceCommands: {
  clean: vi.fn(),
  resources: vi.fn(async () => ({ ramTotal: 16e9, ramAvailable: 4e9, systemCache: 1e9, diskTotal: 500e9, diskFree: 20e9, appWorkingSet: 1, appPrivateBytes: 1, sampledAt: 0 })),
  processes: vi.fn(async () => [
    { target: { pid: 101, started: '1000' }, name: 'background.exe', workingSet: 128e6, privateBytes: 200e6, reason: 'candidate' },
    { target: { pid: 202, started: '2000' }, name: 'music.exe', workingSet: 250e6, privateBytes: 300e6, reason: 'active' },
  ]),
  reclaim: vi.fn(async () => ({ targets: [{ pid: 101, started: '1000' }], processed: 1, skipped: 0, workingSetBefore: 128e6, workingSetAfter: 30e6, privateBefore: 200e6, privateAfter: 200e6 })),
  measure: vi.fn(async () => ({ count: 1, workingSet: 45e6, privateBytes: 200e6 })),
  logs: vi.fn(async (clean: boolean) => ({ bytes: clean ? 300 : 500, files: clean ? 1 : 2, skipped: clean ? 1 : 0 })),
  open: vi.fn(async () => {}),
} }));
const settle = async () => { for (let i = 0; i < 8; i++) { await Promise.resolve(); flushSync(); } };
const button = (target: HTMLElement, text: string) => [...target.querySelectorAll('button')].find(b => b.textContent?.includes(text))!;
afterEach(() => { vi.clearAllMocks(); vi.useRealTimers(); });

it('only submits explicitly selected eligible identities and measures again after 30 seconds', async () => {
  vi.useFakeTimers();
  const target = document.createElement('div'); const component = mount(Panel, { target }); flushSync(); await settle();
  button(target, 'Quét app').click(); await settle();
  const inputs = target.querySelectorAll('input');
  expect(inputs[1]?.disabled).toBe(true);
  expect(button(target, 'Thu hồi RAM').disabled).toBe(true);
  button(target, 'Chọn app nền').click(); await settle();
  button(target, 'Thu hồi RAM').click(); await settle();
  expect(api.reclaim).toHaveBeenCalledExactlyOnceWith([{ pid: 101, started: '1000' }]);
  expect(target.textContent).toContain('Đã xử lý 1 tiến trình');
  await vi.advanceTimersByTimeAsync(30000); await settle();
  expect(api.measure).toHaveBeenCalledExactlyOnceWith([{ pid: 101, started: '1000' }]);
  expect(target.textContent).toContain('Sau 30 giây');
  await unmount(component);
  const calls = vi.mocked(api.resources).mock.calls.length;
  await vi.advanceTimersByTimeAsync(30000);
  expect(api.resources).toHaveBeenCalledTimes(calls);
});

it('shows actual log deletion results and can open Windows Storage separately', async () => {
  const target = document.createElement('div'); const component = mount(Panel, { target }); flushSync(); await settle();
  expect(button(target, 'Dọn log cũ').disabled).toBe(true);
  button(target, 'Kiểm tra log cũ').click(); await settle();
  expect(api.logs).toHaveBeenLastCalledWith(false);
  button(target, 'Dọn log cũ').click(); await settle();
  expect(api.logs).toHaveBeenLastCalledWith(true);
  expect(target.textContent).toContain('300 B'); expect(target.textContent).toContain('bỏ qua 1');
  vi.mocked(api.open).mockRejectedValueOnce(new Error('unavailable'));
  button(target, 'Dọn dung lượng Windows').click(); await settle();
  expect(api.open).toHaveBeenCalledWith('storage');
  expect(target.querySelector('[role="alert"]')).not.toBeNull();
  await unmount(component);
});

it('cleans in one click, renders real progress, prevents duplicate runs and reports skipped items', async () => {
  let report!: (p: CleanupProgress) => void;
  let complete!: (p: CleanupProgress) => void;
  vi.mocked(api.clean).mockImplementationOnce(callback => { report = callback; return new Promise(resolve => { complete = resolve; }); });
  const target = document.createElement('div'); const component = mount(Panel, { target }); flushSync(); await settle();
  button(target, 'Dọn rác ngay').click(); await settle();
  expect(api.clean).toHaveBeenCalledTimes(1);
  expect(target.querySelector('progress')?.hasAttribute('value')).toBe(false);
  const update: CleanupProgress = { phase: 'delete', category: 'browsers', scanned: 40, total: 10, processed: 4, files: 3, bytes: 3072, skipped: 1, busyApps: 1, limited: false };
  report(update); await settle();
  const progress = target.querySelector('progress')!;
  expect(progress.value).toBe(4); expect(progress.max).toBe(10);
  expect(target.textContent).toContain('3.0 KiB');
  expect([...target.querySelectorAll('button')].every(b => b.disabled)).toBe(true);
  complete({ ...update, phase: 'done', processed: 10, files: 8 }); await settle();
  expect(target.querySelector('progress')).toBeNull();
  expect(target.textContent).toContain('có mục được giữ lại');
  expect(target.textContent).toContain('Giữ cache của 1');
  expect(button(target, 'Dọn rác ngay').disabled).toBe(false);
  await unmount(component);
});

it('stops the progress indicator on cleanup failure and allows retry', async () => {
  vi.mocked(api.clean).mockRejectedValueOnce(new Error('busy'));
  const target = document.createElement('div'); const component = mount(Panel, { target }); flushSync(); await settle();
  button(target, 'Dọn rác ngay').click(); await settle();
  expect(target.querySelector('progress')).toBeNull();
  expect(target.textContent).toContain('Chưa hoàn tất dọn rác');
  expect(button(target, 'Dọn rác ngay').disabled).toBe(false);
  await unmount(component);
});

it('stops the delayed measurement when the panel closes', async () => {
  vi.useFakeTimers(); const target = document.createElement('div'); const component = mount(Panel, { target }); flushSync(); await settle();
  button(target, 'Quét app').click(); await settle(); button(target, 'Chọn app nền').click(); await settle(); button(target, 'Thu hồi RAM').click(); await settle();
  await unmount(component); await vi.advanceTimersByTimeAsync(31000);
  expect(api.measure).not.toHaveBeenCalled();
});
