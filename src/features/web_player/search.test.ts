import { runInNewContext } from 'node:vm';
import { expect, it, vi } from 'vitest';
import script from '../../../src-tauri/src/modules/web_player/search.js?raw';

function search(hostname: string, html: string) {
  const page = document.implementation.createHTMLDocument();
  page.body.innerHTML = html;
  const invoke = vi.fn(async () => {});
  const callbacks: Array<() => void> = [];
  const browser: Record<string, unknown> = { __TAURI_INTERNALS__: { invoke } }; browser.top = browser;
  class Media { play() {} }
  runInNewContext(script, { window: browser, document: page, HTMLMediaElement: Media, DOMException, URL,
    location: { hostname, href: `https://${hostname}/search`, origin: `https://${hostname}` },
    setTimeout: (fn: () => void) => callbacks.push(fn),
  });
  (browser.__pcislandSearch as (id: string) => void)('test');
  while (callbacks.length) callbacks.shift()!();
  return invoke.mock.calls as unknown as Array<[string, { items: Array<{ title: string; artist: string | null; uploader: string | null }> }]>;
}

it('extracts Zing artist credits from the observed search row, never treating album as uploader', () => {
  const calls = search('zingmp3.vn', '<main><div class="media-item"><figure title="Đại Thiên Bồng"></figure><div class="card-info"><a class="title" href="/bai-hat/a/1.html">Đại Thiên Bồng</a><h3 class="subtitle"><a>Lee Phú Quý</a></h3></div><div class="album-info">Lan Nhân Nhứ Quả</div></div></main>');
  expect(calls[0]?.[1].items).toEqual([{ url: 'https://zingmp3.vn/bai-hat/a/1.html', title: 'Đại Thiên Bồng', artist: 'Lee Phú Quý', uploader: null }]);
});

it('extracts all Spotify artists from the observed role=row layout and deduplicates credits', () => {
  const calls = search('open.spotify.com', '<main><div role="row"><a href="/track/1">Đại Thiên Bồng 3</a><a href="/artist/1">Trương Tiến Giáp</a><a href="/artist/2">DG MEDIA</a><a href="/artist/2">DG MEDIA</a><a href="/album/1">Album</a></div></main>');
  expect(calls[0]?.[1].items[0]).toMatchObject({ artist: 'Trương Tiến Giáp, DG MEDIA', uploader: null });
});

it('keeps missing credits absent and does not borrow artists from the next track', () => {
  const calls = search('open.spotify.com', '<main><div role="row"><a href="/track/1">Unknown</a></div><div role="row"><a href="/track/2">Known</a><a href="/artist/2">Artist B</a></div></main>');
  expect(calls[0]?.[1].items[0]).toMatchObject({ artist: null, uploader: null });
  expect(calls[0]?.[1].items[1]).toMatchObject({ artist: 'Artist B' });
});
