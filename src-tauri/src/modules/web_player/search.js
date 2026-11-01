// Search-only WebView: same persistent profile, no playback or access to app state.
(() => {
  if (window.top !== window) return;
  HTMLMediaElement.prototype.play = () => Promise.reject(new DOMException('Search view is muted', 'NotAllowedError'));
  const selectors = {
    'zingmp3.vn': 'main .media-item a.title[href*="/bai-hat/"]',
    'open.spotify.com': 'main a[href*="/track/"]',
  };
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
        const row = node.closest(location.hostname === 'zingmp3.vn' ? '.media-item' : '[role="row"]');
        const credits = row?.querySelectorAll(location.hostname === 'zingmp3.vn' ? '.card-info .subtitle a' : 'a[href*="/artist/"]') ?? [];
        const artist = [...new Set([...credits].map((a) => (a.innerText || a.textContent || '').trim()).filter(Boolean))].join(', ').slice(0, 256) || null;
        if (url.origin === location.origin && title && !url.pathname.includes('/sets/')) items.set(url.href, { url: url.href, title: title.slice(0, 512), artist, uploader: null });
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
