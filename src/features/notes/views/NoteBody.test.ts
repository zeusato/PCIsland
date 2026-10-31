import { expect, it, vi } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import NoteBody from './NoteBody.svelte';

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
