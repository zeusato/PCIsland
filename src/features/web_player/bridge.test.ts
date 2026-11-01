import { runInNewContext } from 'node:vm';
import { expect, it, vi } from 'vitest';
import script from '../../../src-tauri/src/modules/web_player/bridge.js?raw';

function repeatFixture() {
  const callbacks: Array<() => void> = [];
  const events = new Map<string, (event: object) => void>();
  const invoke = vi.fn(async () => {});
  const body = { innerText: 'Music' };
  const browser: Record<string, unknown> = { __TAURI_INTERNALS__: { invoke }, addEventListener(type: string, cb: (event: object) => void) { events.set(type, cb); } };
  browser.top = browser;
  class Media {
    currentSrc = 'stream'; currentTime = 180; duration = 180; paused = true; ended = true; playbackRate = 1; loop = false;
    play() { this.paused = false; this.ended = false; return Promise.resolve(); }
    pause() { this.paused = true; }
    addEventListener() {}
  }
  const audio = new Media();
  const clicks = [vi.fn(), vi.fn()];
  const rows = ['First', 'Last'].map((title, i) => ({ getAttribute: (key: string) => key === 'href' ? `/watch?v=${i}` : title, querySelector: () => null, closest: () => null, click: clicks[i] }));
  runInNewContext(script, {
    window: browser, navigator: {}, HTMLMediaElement: Media,
    document: { body, title: 'Last', querySelectorAll: (selector: string) => selector === 'audio, video' ? [audio] : rows, querySelector: () => null, addEventListener() {} },
    location: { href: 'https://www.youtube.com/watch?v=1' }, URL,
    setTimeout: (cb: () => void) => { callbacks.push(cb); return callbacks.length; },
  });
  const bridge = browser.__pcisland as { setRepeat: (mode: string) => boolean };
  const ended = () => { const stopImmediatePropagation = vi.fn(); events.get('ended')?.({ target: audio, stopImmediatePropagation }); return stopImmediatePropagation; };
  return { bridge, audio, clicks, ended, rows, body, invoke, flush: () => { for (let i = 0; i < 20 && callbacks.length; i++) callbacks.shift()?.(); } };
}

it('applies repeat-one to the real media element and disables it when switched off', () => {
  const f = repeatFixture();
  expect(f.bridge.setRepeat('one')).toBe(true);
  expect(f.audio.loop).toBe(true);
  expect(f.ended()).toHaveBeenCalledOnce();
  expect(f.audio.currentTime).toBe(0);
  expect(f.audio.paused).toBe(false);
  expect(f.bridge.setRepeat('off')).toBe(true);
  expect(f.audio.loop).toBe(false);
});

it('wraps the online playlist in list mode and stops on its last track in off mode', () => {
  const f = repeatFixture();
  expect(f.bridge.setRepeat('all')).toBe(true);
  expect(f.ended()).toHaveBeenCalledOnce();
  expect(f.clicks[0]).toHaveBeenCalledOnce();
  f.bridge.setRepeat('off');
  expect(f.ended()).toHaveBeenCalledOnce();
  expect(f.audio.paused).toBe(true);
  expect(f.clicks[0]).toHaveBeenCalledOnce();
});

it('does not report list repeat as applied when no queue is available or access is blocked', () => {
  const f = repeatFixture();
  f.rows.length = 0;
  expect(f.bridge.setRepeat('all')).toBe(false);
  f.flush();
  expect(f.invoke).toHaveBeenLastCalledWith('web_player_report', { msg: expect.objectContaining({ repeat: null, repeatError: true }) });
  f.body.innerText = 'You have been blocked';
  expect(f.bridge.setRepeat('one')).toBe(false);
  expect(f.audio.loop).toBe(false);
});

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

it('reports playlist entries, selects a row, and resumes the saved track position', () => {
  const callbacks: Array<() => void> = [];
  const invoke = vi.fn(async () => {});
  const click = vi.fn();
  const browser: Record<string, unknown> = { __TAURI_INTERNALS__: { invoke }, addEventListener() {} };
  browser.top = browser;
  class Session {
    value: Record<string, unknown> | null = null;
    get metadata() { return this.value; }
    set metadata(value: Record<string, unknown> | null) { this.value = value; }
    setActionHandler() {}
  }
  class Media {
    currentSrc = 'stream'; currentTime = 0; duration = 180; readyState = 4; paused = true; ended = false; playbackRate = 1;
    play = vi.fn(async () => { this.paused = false; });
  }
  const media = new Media();
  const session = new Session();
  const row = { getAttribute: (key: string) => key === 'href' ? '/bai-hat/test/123.html' : 'Saved track', querySelector: () => null, closest: () => null, click };
  runInNewContext(script, {
    window: browser, navigator: { mediaSession: session }, HTMLMediaElement: Media,
    document: { querySelectorAll: (selector: string) => selector === 'audio, video' ? [media] : [row], querySelector: (selector: string) => selector.includes('playbackSoundBadge') ? row : null, addEventListener() {}, title: 'Playlist' },
    location: { href: 'https://zingmp3.vn/album/list/abc.html' }, URL,
    setTimeout: (callback: () => void) => { callbacks.push(callback); return callbacks.length; }, clearTimeout() {},
  });
  session.metadata = { title: 'Saved track' };
  callbacks.shift()?.();
  const url = 'https://zingmp3.vn/bai-hat/test/123.html';
  expect(invoke).toHaveBeenCalledWith('web_player_report', { msg: expect.objectContaining({ pageUrl: 'https://zingmp3.vn/album/list/abc.html', queue: [{ url, title: 'Saved track' }] }) });
  const bridge = browser.__pcisland as { select: (url: string) => void; resume: (saved: object) => void };
  bridge.select(url);
  expect(click).toHaveBeenCalledOnce();
  bridge.resume({ title: 'Saved track', trackUrl: url, positionMs: 42000 });
  expect(media.currentTime).toBe(42);
  expect(media.play).toHaveBeenCalledOnce();
  bridge.resume({ title: 'Different track', trackUrl: 'https://evil.example/', positionMs: 100000 });
  expect(media.currentTime).toBe(42);
});

it('restores the real Zing queue while the sidebar is unmounted and keeps album separate from track', () => {
  const callbacks: Array<() => void> = [];
  const invoke = vi.fn(async () => {});
  const saved = { ids: ['internal-2', 'internal-1'], currentId: 'internal-1', itemMap: {
    'internal-1': { encodeId: 'song-1', title: 'Saved song', link: '/bai-hat/saved/song-1.html', currentPlaylist: { link: '/album/list/album-1.html' } },
    'internal-2': { encodeId: 'song-2', title: 'Other song', link: '/bai-hat/other/song-2.html' },
  } };
  const getItem = vi.fn((key: string) => key === 'zmp3_np_queue.v1.1' ? JSON.stringify(saved) : null);
  const browser: Record<string, unknown> = { __TAURI_INTERNALS__: { invoke }, localStorage: { getItem }, addEventListener() {} };
  browser.top = browser;
  class Session { value: object | null = null; get metadata() { return this.value; } set metadata(v: object | null) { this.value = v; } setActionHandler() {} }
  class Media { play() {} }
  const session = new Session();
  runInNewContext(script, { window: browser, navigator: { mediaSession: session }, HTMLMediaElement: Media,
    document: { querySelectorAll: () => [], querySelector: () => null, addEventListener() {}, title: 'Home' },
    location: { href: 'https://zingmp3.vn/' }, URL, setTimeout: (cb: () => void) => { callbacks.push(cb); return 1; },
  });
  session.metadata = { title: 'Saved song' };
  callbacks.shift()?.();
  expect(invoke).toHaveBeenCalledWith('web_player_report', { msg: expect.objectContaining({
    pageUrl: 'https://zingmp3.vn/album/list/album-1.html', trackUrl: 'https://zingmp3.vn/bai-hat/saved/song-1.html',
    queue: [{ url: 'https://zingmp3.vn/bai-hat/other/song-2.html', title: 'Other song' }, { url: 'https://zingmp3.vn/bai-hat/saved/song-1.html', title: 'Saved song' }],
  }) });
  expect(getItem.mock.calls.every(([key]) => key === 'zmp3_np_queue.v1.1')).toBe(true);
});

it('activates resume only once and stops immediately on an anti-bot page', () => {
  const callbacks: Array<() => void> = [];
  const play = vi.fn();
  const body = { innerText: 'You have been blocked. We detected unusual activity from your device or network.' };
  const browser: Record<string, unknown> = { addEventListener() {} }; browser.top = browser;
  class Session { value: object | null = null; get metadata() { return this.value; } set metadata(v: object | null) { this.value = v; } setActionHandler(_action: string, _handler: () => void) {} }
  class Media { play() {} }
  const session = new Session();
  runInNewContext(script, { window: browser, navigator: { mediaSession: session }, HTMLMediaElement: Media,
    document: { body, querySelectorAll: () => [], querySelector: () => null, addEventListener() {} },
    location: { href: 'https://soundcloud.com/artist/track' }, URL,
    setTimeout: (cb: () => void) => { callbacks.push(cb); return callbacks.length; }, clearTimeout() {},
  });
  session.metadata = { title: 'Saved song' }; session.setActionHandler('play', play);
  const bridge = browser.__pcisland as { resume: (saved: object) => void };
  bridge.resume({ title: 'Saved song', positionMs: 12000 });
  expect(play).not.toHaveBeenCalled();
  body.innerText = 'Music page';
  bridge.resume({ title: 'Saved song', positionMs: 12000 });
  for (let i = 0; i < 40 && callbacks.length; i++) callbacks.shift()?.();
  expect(play).toHaveBeenCalledOnce();
});

it('reports progress from audio that is never attached to the document', () => {
  const callbacks: Array<() => void> = [];
  const events = new Map<string, () => void>();
  const invoke = vi.fn(async () => {});
  const browser: Record<string, unknown> = { __TAURI_INTERNALS__: { invoke }, addEventListener() {} }; browser.top = browser;
  class Media {
    currentSrc = 'stream'; currentTime = 42; duration = 180; paused = true; ended = false; playbackRate = 1;
    addEventListener(type: string, cb: () => void) { events.set(type, cb); }
    play() { this.paused = false; return Promise.resolve(); }
  }
  runInNewContext(script, { window: browser, navigator: {}, HTMLMediaElement: Media,
    document: { querySelectorAll: () => [], querySelector: () => null, addEventListener() {}, title: 'Detached track' },
    location: { href: 'https://soundcloud.com/artist/track' }, URL,
    setTimeout: (cb: () => void) => { callbacks.push(cb); return 1; },
  });
  const audio = new Media(); void audio.play();
  events.get('timeupdate')?.(); callbacks.shift()?.();
  expect(invoke).toHaveBeenCalledWith('web_player_report', { msg: expect.objectContaining({ positionMs: 42000, durationMs: 180000, status: 'playing' }) });
});
