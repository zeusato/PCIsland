import {expect,it,vi} from 'vitest';
import {flushSync,mount,unmount} from 'svelte';
import Dropdown from './Dropdown.svelte';
Object.defineProperty(HTMLElement.prototype,'scrollIntoView',{configurable:true,value:vi.fn()});
const options=[{value:'',label:'Theo Windows'},{value:'long',label:'Microphone Array (Intel Smart Sound Technology for Digital Microphones)'},{value:'usb',label:'USB Microphone'}];
it('selects devices with keyboard and Escape closes only the menu',async()=>{
  const target=document.createElement('div');document.body.append(target);const change=vi.fn();const c=mount(Dropdown,{target,props:{label:'Input',options,onchange:change}});flushSync();
  const trigger=target.querySelector<HTMLButtonElement>('[role=combobox]')!;trigger.click();flushSync();
  expect(trigger.getAttribute('aria-expanded')).toBe('true');expect(target.querySelectorAll('[role=option]')).toHaveLength(3);
  trigger.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowDown',bubbles:true}));flushSync();trigger.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));flushSync();
  expect(change).toHaveBeenCalledWith('long');expect(trigger.textContent).toContain('Microphone Array');expect(trigger.getAttribute('aria-expanded')).toBe('false');
  trigger.click();flushSync();trigger.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}));flushSync();expect(target.querySelector('[role=listbox]')).toBeNull();
  await unmount(c);target.remove();
});
