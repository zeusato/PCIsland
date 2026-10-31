import { runInNewContext } from 'node:vm';
import { expect, it, vi } from 'vitest';
import script from '../../../src-tauri/src/modules/web_player/bridge.js?raw';

it('keeps signed artwork URLs intact while still limiting metadata text', () => {
  const callbacks: Array<() => void> = [];
  const invoke = vi.fn(async () => {});
  const browser: Record<string, unknown> = { __TAURI_INTERNALS__: { invoke }, addEventListener() {} };
  browser.top = browser;
  class Session {
    value: Record<string, unknown> | null = null;
    get metadata() { return this.value; }
    set metadata(value: Record<string, unknown> | null) { this.value = value; }
    setActionHandler() {}
  }
  class Media { play() {} }
  const session = new Session();
  runInNewContext(script, {
    window: browser, navigator: { mediaSession: session }, HTMLMediaElement: Media,
    document: { querySelectorAll: () => [], querySelector: () => null, addEventListener() {}, title: 'Track' },
    location: { href: 'https://zingmp3.vn/' }, URL,
    setTimeout: (callback: () => void) => { callbacks.push(callback); return 1; },
  });
  const url = `https://cdn.example/cover.jpg?signature=${'x'.repeat(600)}`;
  session.metadata = { title: 't'.repeat(600), artwork: [{ src: url }] };
  callbacks.shift()?.();
  expect(invoke).toHaveBeenCalledWith('web_player_report', { msg: expect.objectContaining({ artwork: url, title: 't'.repeat(512) }) });
});
