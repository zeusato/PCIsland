import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { noteCommands } from '$lib/ipc/commands';
import type { Note } from '$lib/ipc/bindings/Note';
import QuickPanel from './QuickPanel.svelte';

afterEach(() => vi.restoreAllMocks());
async function settle() {
  for (let i = 0; i < 10; i++) { await Promise.resolve(); flushSync(); }
}

it('opens the latest note for interaction, adds a task to it, and only composes on New', async () => {
  let notes: Note[] = [
    { id: 'old', text: 'Ghim cũ', pinned: true, createdAt: 1, updatedAt: 1 },
    { id: 'latest', text: 'Việc hôm nay\n[] Mua sữa', pinned: false, createdAt: 2, updatedAt: 2 },
  ];
  vi.spyOn(noteCommands, 'list').mockImplementation(async () => notes.map((note) => ({ ...note })));
  const toggle = vi.spyOn(noteCommands, 'toggleCheck').mockImplementation(async (id) => {
    const note = notes.find((n) => n.id === id)!;
    note.text = note.text.replace('[]', '[x]');
    return { ...note };
  });
  const save = vi.spyOn(noteCommands, 'save').mockImplementation(async (text, id) => {
    const note = { id: id ?? 'new', text, pinned: false, createdAt: 3, updatedAt: 3 };
    notes = [...notes.filter((n) => n.id !== note.id), note];
    return note;
  });
  const target = document.createElement('div');
  document.body.append(target);
  const component = mount(QuickPanel, { target, props: { close: vi.fn() } });
  await settle();
  const button = (label: string) => [...target.querySelectorAll('button')].find((b) => b.textContent === label)!;
  expect(target.querySelector('textarea')).toBeNull();
  expect(target.querySelector('.note-content')?.textContent).toContain('Việc hôm nay');
  target.querySelector<HTMLButtonElement>('[role="checkbox"]')!.click();
  await settle();
  expect(toggle).toHaveBeenCalledWith('latest', 1);
  expect(target.querySelector('[role="checkbox"]')?.getAttribute('aria-checked')).toBe('true');
  button('+ Ô tick').click();
  await settle();
  const input = target.querySelector('input')!;
  input.value = 'Gọi khách';
  input.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
  target.querySelector('form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  await settle();
  expect(save).toHaveBeenCalledWith('Việc hôm nay\n[x] Mua sữa\n[ ] Gọi khách', 'latest', 2);
  expect(target.querySelectorAll('[role="checkbox"]')).toHaveLength(2);
  button('Tạo mới').click();
  await settle();
  const area = target.querySelector('textarea')!;
  expect(area.value).toBe('');
  area.value = 'Ghi chú mới';
  area.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
  button('Lưu').click();
  await settle();
  expect(save).toHaveBeenLastCalledWith('Ghi chú mới', undefined);
  expect(target.querySelector('textarea')).toBeNull();
  expect(target.querySelector('.note-content')?.textContent).toContain('Ghi chú mới');
  button('Danh sách ghi chú').click();
  flushSync();
  button('Ghim cũ').click();
  flushSync();
  expect(target.querySelector('.note-content')?.textContent).toContain('Ghim cũ');
  await unmount(component);
  target.remove();
});

it('keeps an empty notebook in browse mode until explicitly creating a note', async () => {
  vi.spyOn(noteCommands, 'list').mockResolvedValue([]);
  const target = document.createElement('div');
  const component = mount(QuickPanel, { target, props: { close: vi.fn() } });
  await settle();
  expect(target.textContent).toContain('Chưa có ghi chú nào');
  expect(target.querySelector('textarea')).toBeNull();
  expect([...target.querySelectorAll('button')].find((b) => b.textContent === '+ Ô tick')?.disabled).toBe(true);
  await unmount(component);
});
