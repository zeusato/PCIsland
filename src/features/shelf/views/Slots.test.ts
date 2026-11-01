import { expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import type { ShelfItem } from '$lib/ipc/bindings/ShelfItem';
import { dragFilesOut } from '$lib/ipc/drag';
import Slots from './Slots.svelte';
vi.mock('$lib/ipc/drag',()=>({dragFilesOut:vi.fn(async()=>{})}));
const items = (count: number): ShelfItem[] => Array.from({length:count},(_,i)=>({id:String(i),path:`D:\\file${i}.txt`,name:`file${i}.txt`,size:100,isDir:false,addedAt:i,missing:false,thumbnailUrl:null}));
it('stores more than three files and selects a range for native multi-file drag',async()=>{
  const target=document.createElement('div');const files=items(8);const c=mount(Slots,{target,props:{items:files}});flushSync();
  const rows=target.querySelectorAll<HTMLButtonElement>('.file');expect(rows).toHaveLength(8);
  rows[1]!.click();rows[4]!.dispatchEvent(new MouseEvent('click',{shiftKey:true,bubbles:true}));flushSync();
  expect(target.querySelectorAll('[aria-pressed=true]')).toHaveLength(4);
  const drag=[...target.querySelectorAll('button')].find(b=>b.textContent==='Kéo mục đã chọn')!;
  drag.dispatchEvent(new MouseEvent('mousedown',{button:0,bubbles:true}));
  expect(dragFilesOut).toHaveBeenLastCalledWith(files.slice(1,5).map(f=>f.path));await unmount(c);
});
it('bounds rendering but drag-all includes every file across pages',async()=>{
  const target=document.createElement('div');const files=items(75);const c=mount(Slots,{target,props:{items:files}});flushSync();
  expect(target.querySelectorAll('.file')).toHaveLength(50);
  [...target.querySelectorAll('button')].find(b=>b.textContent==='Kéo tất cả')!.dispatchEvent(new MouseEvent('mousedown',{button:0,bubbles:true}));
  expect(dragFilesOut).toHaveBeenLastCalledWith(files.map(f=>f.path));await unmount(c);
});
it('Escape clears selection before leaving the shelf',async()=>{
  const target=document.createElement('div');const c=mount(Slots,{target,props:{items:items(4)}});flushSync();
  target.querySelector<HTMLButtonElement>('.file')!.click();flushSync();
  const outer=document.createElement('div');outer.append(target);document.body.append(outer);const parent=vi.fn();outer.addEventListener('keydown',parent);
  target.querySelector('.file')!.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}));flushSync();
  expect(parent).not.toHaveBeenCalled();outer.remove();expect(target.querySelectorAll('[aria-pressed=true]')).toHaveLength(0);await unmount(c);
});
