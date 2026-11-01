// PCisland stream sniffer — injected into a hidden, muted view of an arbitrary page
// (docs/modules/video-player.md §5.3, same approach as the stream-pocket add-on):
// network responses by content type, resource timing, and a DOM scan. Reports only
// http(s) media URLs through `video_sniff_report`; never reads cookies or page data.
(() => {
  if (window.top !== window || window.__pcislandSniff) return;
  const requestId = __PCISLAND_REQUEST__;
  window.__pcislandSniff = true;
  const found = new Map();
  let blobs = 0;
  let lastSent = '';
  const started = Date.now();

  // Port of core.mjs `classify` from the add-on.
  const classify = (url, mime = '') => {
    let path;
    try { path = new URL(url).pathname.toLowerCase(); } catch { return null; }
    const type = String(mime).toLowerCase().split(';')[0].trim();
    if (/\.m3u8$/.test(path) || /(?:vnd\.apple\.mpegurl|x-mpegurl)/.test(type)) return 'hls';
    if (/\.mpd$/.test(path) || type === 'application/dash+xml') return 'dash';
    if (/\.(?:ts|m4s|cmfv|cmfa)$/.test(path) || type === 'video/mp2t') return null;
    if (/\.(?:mp4|webm|mov|m4v|mkv|mp3|m4a|ogg|oga|wav|aac|flac)$/.test(path) || /^(video|audio)\//.test(type)) return 'file';
    return null;
  };
  const add = (raw, mime = '') => {
    if (!raw) return;
    let url;
    try { url = new URL(raw, document.baseURI); } catch { return; }
    if (url.protocol === 'blob:') { blobs++; return; }
    if (!['http:', 'https:'].includes(url.protocol) || url.href.length > 4096 || found.size >= 40) return;
    const kind = classify(url.href, mime);
    if (kind && kind !== 'dash' && !found.has(url.href)) found.set(url.href, mime || kind);
  };

  const fetch0 = window.fetch;
  if (fetch0) window.fetch = function (...args) {
    return fetch0.apply(this, args).then((r) => { try { add(r.url, r.headers.get('content-type') || ''); } catch {} return r; });
  };
  const open0 = XMLHttpRequest.prototype.open;
  XMLHttpRequest.prototype.open = function (method, url, ...rest) {
    this.addEventListener('load', () => { try { add(this.responseURL || url, this.getResponseHeader('content-type') || ''); } catch {} }, { once: true });
    return open0.call(this, method, url, ...rest);
  };
  try {
    new PerformanceObserver((list) => {
      for (const e of list.getEntries()) add(e.name, e.contentType || (['video', 'audio'].includes(e.initiatorType) ? e.initiatorType + '/unknown' : ''));
    }).observe({ type: 'resource', buffered: true });
  } catch { /* Older engines: the DOM scan still runs. */ }

  const scan = () => {
    for (const m of document.querySelectorAll('video,audio')) {
      const type = m.tagName === 'AUDIO' ? 'audio/unknown' : 'video/unknown';
      add(m.currentSrc, type);
      add(m.getAttribute('src'), type);
      for (const s of m.querySelectorAll('source')) add(s.src, s.type || type);
    }
    for (const a of document.querySelectorAll('a[href]')) {
      if (/\.(mp4|webm|mov|m4v|mkv|m3u8|mp3|m4a|ogg|wav|aac|flac)(?:[?#]|$)/i.test(a.href)) add(a.href);
    }
    for (const meta of document.querySelectorAll('meta[property^="og:video"],meta[name="twitter:player:stream"]')) {
      if (!/(width|height|type)$/.test(meta.getAttribute('property') || '')) add(meta.content);
    }
  };

  const report = (done) => {
    const items = [...found].map(([url, mime]) => ({ url, mime }));
    const key = JSON.stringify(items) + blobs + done;
    if (key === lastSent) return;
    lastSent = key;
    window.__TAURI_INTERNALS__?.invoke('video_sniff_report', { requestId, items, blobs, done }).catch(() => {});
  };
  const tick = () => {
    try { scan(); } catch {}
    const elapsed = Date.now() - started;
    const done = elapsed > 20000 || (found.size > 0 && elapsed > 6000);
    report(done);
    if (!done) setTimeout(tick, 1000);
  };
  document.addEventListener('DOMContentLoaded', () => setTimeout(tick, 500), { once: true });
})();
