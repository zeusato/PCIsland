import { runInNewContext } from 'node:vm';
import { expect, it, vi } from 'vitest';
import script from '../../../src-tauri/src/modules/web_player/bridge.js?raw';

it.each(['album', 'song'])('selects the requested %s song outside the old queue, even with the same title, and waits before seeking/playing', async (pageType) => {
  const callbacks: Array<() => void> = [];
  const invoke = vi.fn(async () => {});
  const page = document.implementation.createHTMLDocument();
  page.body.innerHTML = pageType === 'album'
    ? '<div class="playlist-content"><div class="media-item"><figure title="Same title"></figure><button class="action-play">Play</button></div></div>'
    : '<main><div class="media song-header"><div class="media-content"><div class="actions"><button class="btn-play-all">Play song</button></div></div></div></main>';
  const click = vi.fn(); page.querySelector('button')!.addEventListener('click', click);
  const old = { encodeId: 'old', title: 'Same title', link: '/bai-hat/old/old.html' };
  const next = { encodeId: 'new', title: 'Same title', link: '/bai-hat/new/new.html' };
  const saved = { ids: ['old'], currentId: 'old', itemMap: { old, new: next } };
  const browser: Record<string, unknown> = {
    __TAURI_INTERNALS__: { invoke }, addEventListener() {},
    localStorage: { getItem: () => JSON.stringify(saved) },
    fetch: async () => ({ clone: () => ({ json: async () => ({ err: 0, data: { link: '/album/new/a.html', song: { items: [next] } } }) }) }),
  }; browser.top = browser;
  class Session { value: object | null = null; get metadata() { return this.value; } set metadata(v: object | null) { this.value = v; } setActionHandler() {} }
  class Media {
    currentSrc = 'old-stream'; currentTime = 42; duration = 180; readyState = 4; paused = true; ended = false; playbackRate = 1;
    play = vi.fn(async () => { this.paused = false; });
  }
  const media = new Media(); const session = new Session();
  runInNewContext(script, { window: browser, navigator: { mediaSession: session }, HTMLMediaElement: Media,
    document: { body: page.body, title: 'Album', querySelector: page.querySelector.bind(page), querySelectorAll: (s: string) => s === 'audio, video' ? [media] : page.querySelectorAll(s), addEventListener() {} },
    location: { href: pageType === 'album' ? 'https://zingmp3.vn/album/new/a.html' : 'https://zingmp3.vn/bai-hat/new/new.html' }, URL,
    setTimeout: (fn: () => void) => { callbacks.push(fn); return callbacks.length; }, clearTimeout() {},
  });
  session.metadata = { title: 'Same title' };
  if (pageType === 'album') await (browser.fetch as (url: string) => Promise<unknown>)('https://zingmp3.vn/api/v2/page/get/playlist');
  await Promise.resolve();
  const bridge = browser.__pcisland as { resume: (saved: object) => void; cancelResume: () => void };
  bridge.resume({ title: 'Same title', trackUrl: 'https://zingmp3.vn/bai-hat/new/new.html', positionMs: 0 });
  expect(click).toHaveBeenCalledOnce();
  expect(media.currentTime).toBe(42);
  expect(media.play).not.toHaveBeenCalled();
  for (let i = 0; i < 5 && callbacks.length; i++) callbacks.shift()?.();
  expect(click).toHaveBeenCalledOnce();
  expect(media.play).not.toHaveBeenCalled();
  expect(invoke).toHaveBeenCalledWith('web_player_report', { msg: expect.objectContaining({ queue: [{ url: 'https://zingmp3.vn/bai-hat/old/old.html', title: 'Same title' }] }) });
  saved.currentId = 'new'; saved.ids = ['new']; media.currentSrc = 'new-stream';
  for (let i = 0; i < 10 && callbacks.length; i++) callbacks.shift()?.();
  expect(media.currentTime).toBe(0);
  expect(media.play).toHaveBeenCalledOnce();
});
