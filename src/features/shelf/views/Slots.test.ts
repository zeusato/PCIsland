import { expect, it } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import type { ShelfItem } from '$lib/ipc/bindings/ShelfItem';
import Slots from './Slots.svelte';

it.each([0, 1, 2, 3])('shows %i existing files and a single next drop slot until full', async (count) => {
  const items: ShelfItem[] = Array.from({ length: count }, (_, i) => ({ id: String(i), path: `D:\\file${i}.png`, name: `file${i}.png`, size: 100, isDir: false, addedAt: i, missing: false, thumbnailUrl: `artwork://image${i}` }));
  const target = document.createElement('div');
  const component = mount(Slots, { target, props: { items, receiving: true } });
  flushSync();
  expect(target.querySelectorAll('.file')).toHaveLength(count);
  expect(target.querySelectorAll('img')).toHaveLength(count);
  expect(target.querySelectorAll('.empty')).toHaveLength(count === 3 ? 0 : 1);
  expect(target.querySelectorAll('.slot')).toHaveLength(Math.min(count + 1, 3));
  await unmount(component);
});
