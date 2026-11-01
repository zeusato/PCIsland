import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { noteCommands } from '$lib/ipc/commands';
import type { NoteTask } from '$lib/ipc/bindings/NoteTask';
import TaskAlarmEditor from './TaskAlarmEditor.svelte';

afterEach(() => vi.restoreAllMocks());
const task: NoteTask = { id: 'b', noteId: 'note', lineIndex: 1, label: 'Task B', checked: false, alarm: null, dueAt: null, firedAt: null };

it.each(['at', 'afterNow', 'afterPrevious'])('saves %s reminders against the task ID', async (mode) => {
  const save = vi.spyOn(noteCommands, 'setTaskAlarm').mockResolvedValue();
  const close = vi.fn();
  const target = document.createElement('div');
  const component = mount(TaskAlarmEditor, { target, props: { task, hasPrevious: true, close } });
  flushSync();
  const select = target.querySelector('select')!;
  select.value = mode; select.dispatchEvent(new Event('change')); flushSync();
  const input = target.querySelector('input')!;
  input.value = mode === 'at' ? '2099-01-01T10:30:00' : '10';
  input.dispatchEvent(new Event('input')); flushSync();
  target.querySelector('form')!.dispatchEvent(new Event('submit', { cancelable: true }));
  await vi.waitFor(() => expect(close).toHaveBeenCalled());
  expect(save).toHaveBeenCalledWith('b', { mode, at: mode === 'at' ? new Date('2099-01-01T10:30:00').getTime() : null, delayMs: mode === 'at' ? null : 10000 });
  await unmount(component);
});

it('disables dependency mode for the first task and rejects past dates', async () => {
  const save = vi.spyOn(noteCommands, 'setTaskAlarm').mockResolvedValue();
  const target = document.createElement('div');
  const component = mount(TaskAlarmEditor, { target, props: { task, hasPrevious: false, close: vi.fn() } });
  flushSync();
  expect(target.querySelector<HTMLOptionElement>('option[value="afterPrevious"]')?.disabled).toBe(true);
  const select = target.querySelector('select')!;
  select.value = 'at'; select.dispatchEvent(new Event('change')); flushSync();
  const input = target.querySelector('input')!;
  input.value = '2020-01-01T10:30:00'; input.dispatchEvent(new Event('input')); flushSync();
  target.querySelector('form')!.dispatchEvent(new Event('submit', { cancelable: true })); flushSync();
  expect(save).not.toHaveBeenCalled();
  expect(target.querySelector('[role="alert"]')).not.toBeNull();
  await unmount(component);
});
