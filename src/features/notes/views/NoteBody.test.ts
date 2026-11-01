import { expect, it, vi } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import NoteBody from './NoteBody.svelte';
import { noteCommands } from '$lib/ipc/commands';
import type { NoteTask } from '$lib/ipc/bindings/NoteTask';
vi.mock('$lib/ipc/commands', async (original) => ({ ...await original<object>(), inTauri: true }));
const events = vi.hoisted(() => ({ update: () => {} }));
vi.mock('$lib/ipc/events', () => ({ onNotesState: async (callback: () => void) => { events.update = callback; return () => {}; } }));

it('renders real clickable checkboxes and preserves original line indices', async () => {
  const target = document.createElement('div');
  document.body.append(target);
  const toggle = vi.fn(async () => {});
  const edit = vi.fn();
  const component = mount(NoteBody, { target, props: { text: 'Nội dung\n\n[] việc một\n[X] việc hai', ontoggle: toggle, onedit: edit } });
  flushSync();
  const boxes = target.querySelectorAll<HTMLButtonElement>('[role="checkbox"]');
  expect(boxes).toHaveLength(2);
  expect(boxes[0]?.getAttribute('aria-checked')).toBe('false');
  expect(boxes[1]?.getAttribute('aria-checked')).toBe('true');
  boxes[0]?.click();
  await Promise.resolve();
  expect(toggle).toHaveBeenCalledWith(2);
  expect(edit).not.toHaveBeenCalled();
  await unmount(component);
  target.remove();
});

it('shows live countdowns and scheduled times at the end of their own task rows', async () => {
  vi.useFakeTimers();
  const at = new Date('2026-11-01T10:00:00').getTime();
  vi.setSystemTime(at);
  const task = (id: string, lineIndex: number, mode: string, dueAt: number | null): NoteTask => ({
    id, noteId: 'note', lineIndex, label: id, checked: false, alarm: { mode, at: mode === 'at' ? dueAt : null, delayMs: 10000 }, dueAt, firedAt: null,
  });
  const tasks = [task('Countdown', 0, 'afterNow', at + 10000), task('Appointment', 1, 'at', at + 3600000), task('Waiting', 2, 'afterPrevious', null)];
  vi.spyOn(noteCommands, 'tasks').mockImplementation(async () => structuredClone(tasks));
  const target = document.createElement('div');
  const component = mount(NoteBody, { target, props: { noteId: 'note', text: '[] Countdown\n[] Appointment\n[] Waiting', ontoggle: vi.fn(async () => {}) } });
  try {
    flushSync(); await vi.advanceTimersByTimeAsync(0); flushSync();
    const labels = () => [...target.querySelectorAll('.task-row .alarm-time')].map((node) => node.textContent);
    expect(labels()[0]).toBe('0:10');
    expect(labels()[1]).toContain('11:00:00');
    expect(labels()[2]).toBe('Chờ dòng trước · 0:10');
    await vi.advanceTimersByTimeAsync(3000); flushSync();
    expect(labels()[0]).toBe('0:07');
    await vi.advanceTimersByTimeAsync(10000); flushSync();
    expect(labels()[0]).toBe('0:00');
    tasks[0]!.firedAt = at + 10000;
    events.update(); await vi.advanceTimersByTimeAsync(0); flushSync();
    expect(labels()[0]).toBe('Đã đến giờ nhắc');
  } finally { await unmount(component); vi.restoreAllMocks(); vi.useRealTimers(); }
});
