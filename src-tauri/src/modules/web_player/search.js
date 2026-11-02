// Search-only WebView: same persistent profile, no playback or access to app state.
(() => {
  if (window.top !== window) return;
  HTMLMediaElement.prototype.play = () => Promise.reject(new DOMException('Search view is muted', 'NotAllowedError'));
  const selectors = {
    'zingmp3.vn': 'main .media-item a.title[href*="/bai-hat/"]',
    'open.spotify.com': 'main a[href*="/track/"]',
    'soundcloud.com': '.searchList__item a.soundTitle__title[href]',
  };
  const rowOf = { 'zingmp3.vn': '.media-item', 'open.spotify.com': '[role="row"]', 'soundcloud.com': '.searchList__item' };
  let started = false;
  window.__pcislandSearch = (requestId) => {
    if (started) return;
    started = true;
    let previous = '', stable = 0, attempts = 0;
    const poll = () => {
      const items = new Map();
      for (const node of document.querySelectorAll(selectors[location.hostname] || ':not(*)')) {
        const href = node.getAttribute('href');
        if (!href) continue;
        const url = new URL(href, location.href);
        const title = (node.closest('.media-item')?.querySelector('figure[title]')?.getAttribute('title') || node.innerText || node.textContent || '').trim();
        // Verified against the public search DOM on 2026-11-01. Keep artist
        // credits separate from uploaders; neither search page exposes an uploader.
        const row = node.closest(rowOf[location.hostname] || '[role="row"]');
        // SoundCloud only names the uploading account, never a separate artist.
        const soundCloud = location.hostname === 'soundcloud.com';
        const credits = row?.querySelectorAll(location.hostname === 'zingmp3.vn' ? '.card-info .subtitle a' : soundCloud ? '.soundTitle__username' : 'a[href*="/artist/"]') ?? [];
        const names = [...new Set([...credits].map((a) => (a.innerText || a.textContent || '').trim()).filter(Boolean))].join(', ').slice(0, 256) || null;
        if (url.origin === location.origin && title && !url.pathname.includes('/sets/')) items.set(url.href, { url: url.href, title: title.slice(0, 512), artist: soundCloud ? null : names, uploader: soundCloud ? names : null });
        if (items.size >= 30) break;
      }
      const result = [...items.values()];
      const key = JSON.stringify(result);
      stable = key === previous ? stable + 1 : 0;
      previous = key;
      if (result.length && stable >= 2 || ++attempts >= 40) {
        window.__TAURI_INTERNALS__?.invoke('web_player_search_report', { requestId, items: result }).catch(() => {});
      } else setTimeout(poll, 350);
    };
    poll();
  };
})();
