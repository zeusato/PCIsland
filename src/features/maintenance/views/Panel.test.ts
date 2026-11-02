import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import Panel from './Panel.svelte';
import { resetMaintenance } from '../controller.svelte';
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
  recycleBin: vi.fn(async () => ({ bytes: 2 * 1024 ** 3, items: 42 })),
  emptyRecycleBin: vi.fn(async () => ({ bytes: 2 * 1024 ** 3, items: 42 })),
  windowsUpdate: vi.fn(async () => ({ status: 'cancelled', bytes: 0 })),
  apps: vi.fn(async () => [
    { name: 'chrome.exe', processes: [{ pid: 20, started: 't20' }, { pid: 21, started: 't21' }], workingSet: 900e6, privateBytes: 1e9, windows: 2, audio: false },
    { name: 'music.exe', processes: [{ pid: 50, started: 't50' }], workingSet: 300e6, privateBytes: 3e8, windows: 1, audio: true },
  ]),
  closeApp: vi.fn(async () => ({ status: 'pending', closed: 1, remaining: [{ pid: 21, started: 't21' }], skipped: 0 })),
  forceClose: vi.fn(async () => ({ status: 'closed', closed: 1, remaining: [], skipped: 0 })),
} }));
const settle = async () => { for (let i = 0; i < 8; i++) { await Promise.resolve(); flushSync(); } };
const button = (target: HTMLElement, text: string) => [...target.querySelectorAll('button')].find(b => b.textContent?.includes(text))!;
afterEach(() => { vi.clearAllMocks(); vi.useRealTimers(); resetMaintenance(); });

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
  const update: CleanupProgress = { phase: 'delete', category: 'browsers', scanned: 40, total: 10, processed: 4, files: 3, bytes: 3072, folders: 0, skipped: 1, busyApps: 1, limited: false,
    groups: [{ category: 'shaders', estimated: 5 * 1024 ** 2, candidates: 7, bytes: 0, files: 0 }] };
  report(update); await settle();
  const progress = target.querySelector('progress')!;
  expect(progress.value).toBe(4); expect(progress.max).toBe(10);
  expect(target.textContent).toContain('3.0 KiB');
  expect([...target.querySelectorAll('button')].every(b => b.disabled)).toBe(true);
  expect(target.textContent).toContain('Cache shader GPU');
  expect(target.textContent).toContain('Tìm thấy 5.0 MiB · 7 file');
  complete({ ...update, phase: 'done', processed: 10, files: 8, folders: 2, groups: [{ category: 'shaders', estimated: 5 * 1024 ** 2, candidates: 7, bytes: 4 * 1024 ** 2, files: 6 }] }); await settle();
  expect(target.textContent).toContain('Đã xóa 4.0 MiB / tìm thấy 5.0 MiB');
  expect(target.textContent).toContain('Đã xóa 2 thư mục trống');
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

it('keeps cleanup progress and the delayed measurement when the panel closes and reopens', async () => {
  let report!: (p: CleanupProgress) => void;
  let complete!: (p: CleanupProgress) => void;
  vi.mocked(api.clean).mockImplementationOnce(callback => { report = callback; return new Promise(resolve => { complete = resolve; }); });
  let target = document.createElement('div'); let component = mount(Panel, { target }); flushSync(); await settle();
  button(target, 'Dọn rác ngay').click(); await settle();
  await unmount(component);
  const update: CleanupProgress = { phase: 'delete', category: 'temp', scanned: 40, total: 10, processed: 4, files: 3, bytes: 3072, folders: 0, skipped: 0, busyApps: 0, limited: false, groups: [] };
  report(update);
  target = document.createElement('div'); component = mount(Panel, { target }); flushSync(); await settle();
  expect(target.querySelector('progress')?.value).toBe(4);
  expect(api.clean).toHaveBeenCalledTimes(1);
  await unmount(component);
  complete({ ...update, phase: 'done', processed: 10 });
  target = document.createElement('div'); component = mount(Panel, { target }); flushSync(); await settle();
  expect(target.querySelector('progress')).toBeNull();
  expect(target.textContent).toContain('3.0 KiB');
  await unmount(component);
});

it('finishes the delayed RAM measurement even if the panel closed meanwhile', async () => {
  vi.useFakeTimers(); const target = document.createElement('div'); const component = mount(Panel, { target }); flushSync(); await settle();
  button(target, 'Quét app').click(); await settle(); button(target, 'Chọn app nền').click(); await settle(); button(target, 'Thu hồi RAM').click(); await settle();
  await unmount(component); await vi.advanceTimersByTimeAsync(31000);
  expect(api.measure).toHaveBeenCalledTimes(1);
});

it('empties the Recycle Bin only after an explicit confirmation', async () => {
  const target = document.createElement('div'); const component = mount(Panel, { target }); flushSync(); await settle();
  expect(target.textContent).toContain('42 mục · 2.0 GiB');
  button(target, 'Dọn Thùng rác').click(); await settle();
  expect(api.emptyRecycleBin).not.toHaveBeenCalled();
  expect(target.textContent).toContain('Không thể hoàn tác');
  button(target, 'Hủy').click(); await settle();
  expect(api.emptyRecycleBin).not.toHaveBeenCalled();
  button(target, 'Dọn Thùng rác').click(); await settle();
  button(target, 'Xóa vĩnh viễn').click(); await settle();
  expect(api.emptyRecycleBin).toHaveBeenCalledTimes(1);
  expect(target.textContent).toContain('Đã xóa vĩnh viễn 42 mục');
  await unmount(component);
});

it('runs the elevated Windows Update cleanup after confirmation and reports a declined UAC honestly', async () => {
  const target = document.createElement('div'); const component = mount(Panel, { target }); flushSync(); await settle();
  button(target, 'Dọn cache Windows Update').click(); await settle();
  expect(api.windowsUpdate).not.toHaveBeenCalled();
  button(target, 'Tiếp tục').click(); await settle();
  expect(api.windowsUpdate).toHaveBeenCalledTimes(1);
  expect(target.textContent).toContain('chưa dọn gì');
  vi.mocked(api.windowsUpdate).mockResolvedValueOnce({ status: 'done', bytes: 3 * 1024 ** 3 });
  button(target, 'Dọn cache Windows Update').click(); await settle(); button(target, 'Tiếp tục').click(); await settle();
  expect(target.textContent).toContain('Đã dọn 3.0 GiB cache Windows Update');
  await unmount(component);
});

it('closes an app gracefully first and only offers force close as a second confirmed step', async () => {
  const target = document.createElement('div'); const component = mount(Panel, { target }); flushSync(); await settle();
  button(target, 'Tìm app nặng').click(); await settle();
  expect(target.textContent).toContain('chrome.exe');
  expect(target.textContent).toContain('Đang phát âm thanh');
  const closeButtons = () => [...target.querySelectorAll('button')].filter(b => b.textContent === 'Đóng');
  closeButtons()[0]!.click(); await settle();
  expect(api.closeApp).not.toHaveBeenCalled();
  expect(target.textContent).toContain('Đóng chrome.exe?');
  expect(button(target, 'Buộc đóng')).toBeUndefined();
  button(target, 'Xác nhận đóng').click(); await settle();
  expect(api.closeApp).toHaveBeenCalledExactlyOnceWith([{ pid: 20, started: 't20' }, { pid: 21, started: 't21' }], false);
  expect(target.textContent).toContain('chưa thoát (1 tiến trình còn chạy)');
  expect(api.forceClose).not.toHaveBeenCalled();
  button(target, 'Buộc đóng').click(); await settle();
  expect(api.forceClose).not.toHaveBeenCalled();
  expect(target.textContent).toContain('Dữ liệu chưa lưu trong app sẽ mất');
  [...target.querySelectorAll('button')].filter(b => b.textContent === 'Buộc đóng').at(-1)!.click(); await settle();
  expect(api.forceClose).toHaveBeenCalledExactlyOnceWith([{ pid: 21, started: 't21' }]);
  expect(target.textContent).toContain('Đã đóng chrome.exe');
  await unmount(component);
});

it('warns before closing an app that plays audio and passes the explicit consent', async () => {
  vi.mocked(api.closeApp).mockResolvedValueOnce({ status: 'closed', closed: 1, remaining: [], skipped: 0 });
  const target = document.createElement('div'); const component = mount(Panel, { target }); flushSync(); await settle();
  button(target, 'Tìm app nặng').click(); await settle();
  [...target.querySelectorAll('button')].filter(b => b.textContent === 'Đóng')[1]!.click(); await settle();
  expect(target.textContent).toContain('music.exe đang phát âm thanh');
  button(target, 'Xác nhận đóng').click(); await settle();
  expect(api.closeApp).toHaveBeenCalledExactlyOnceWith([{ pid: 50, started: 't50' }], true);
  expect(target.textContent).not.toContain('Đang phát âm thanh');
  await unmount(component);
});
