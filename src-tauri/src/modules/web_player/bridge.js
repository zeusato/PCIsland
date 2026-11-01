// PCisland bridge — injected into music sites before their own scripts run
// (docs/modules/web-player.md §5). Reads Media Session + media elements and reports
// through the single `web_player_report` command; exposes `__pcisland.exec(action)`.
(() => {
  if (window.top !== window || window.__pcisland) return;

  const MAX = 512;
  const clip = (v) => (typeof v === 'string' ? v.slice(0, MAX) : null);
  const handlers = new Map();
  const tracked = new Set();
  let metadata = null;
  let sessionState = 'none';
  let pending = null;
  let lastSent = 0;
  let lastPositionReport = 0;
  let restore = null;
  let restoreTimer = null;
  let appNavigation = false;
  let repeatMode = null;
  let repeatError = false;
  const handledEnds = new WeakSet();
  // An explicit source/track change in PCisland must not hang behind a hidden
  // "leave while playing" dialog. Ordinary navigation keeps the site's own guard.
  window.addEventListener('beforeunload', (event) => {
    if (appNavigation) event.stopImmediatePropagation();
  }, true);
  const queueNodes = new Map();
  // Album/search rows can select a song without belonging to the playing queue.
  const pageNodes = new Map();
  const isZing = new URL(location.href).hostname === 'zingmp3.vn';
  const isSoundCloud = new URL(location.href).hostname === 'soundcloud.com';
  const isSpotify = new URL(location.href).hostname === 'open.spotify.com';
  const knownTracks = new Map();
  const zingTracks = new Map();
  let zingPlaylist = null;
  const accessBlocked = () => /you have been blocked|we detected unusual activity|verify (?:that )?you are human/i.test(document.body?.innerText?.slice(0, 16000) || '');
  const urlOf = (value) => {
    if (typeof value !== 'string' || !value.trim()) return null;
    try { const url = new URL(value, location.href); return url.protocol === 'https:' && url.origin === new URL(location.href).origin ? url.href : null; }
    catch { return null; }
  };
  // Zing v1.22.4, verified against real Network + player state (2026-11-01).
  // Only music metadata is retained. Never capture cookies, request signatures or audio URLs.
  const zingTrack = (item) => {
    const url = item && urlOf(item.link);
    if (!url || !item.title || !url.includes('/bai-hat/')) return null;
    const track = { url, title: clip(item.title), id: item.encodeId, playlistUrl: item.currentPlaylist?.link && urlOf(item.currentPlaylist.link) };
    if (track.id) { zingTracks.set(track.id, track); if (zingTracks.size > 500) zingTracks.delete(zingTracks.keys().next().value); }
    return track;
  };
  const zingEndpoint = (value) => {
    try { const u = new URL(value, location.href); return u.hostname === 'zingmp3.vn' && ['/api/v2/page/get/playlist', '/api/v2/song/get/info'].includes(u.pathname) ? u.pathname : null; }
    catch { return null; }
  };
  const captureZing = (endpoint, json) => {
    if (json?.err !== 0 || !json.data) return;
    if (endpoint === '/api/v2/page/get/playlist') {
      zingPlaylist = { url: urlOf(json.data.link), items: (json.data.song?.items ?? []).slice(0, 100).map(zingTrack).filter(Boolean) };
    } else zingTrack(json.data);
    report();
  };
  if (isZing) {
    if (window.fetch) {
      const original = window.fetch;
      window.fetch = function (...args) {
        const endpoint = zingEndpoint(typeof args[0] === 'string' ? args[0] : args[0]?.url);
        return original.apply(this, args).then((response) => {
          if (endpoint) response.clone().json().then((json) => captureZing(endpoint, json)).catch(() => {});
          return response;
        });
      };
    }
    if (window.XMLHttpRequest) {
      const proto = window.XMLHttpRequest.prototype;
      const open = proto.open;
      const urls = new WeakMap();
      proto.open = function (method, url, ...args) { urls.set(this, zingEndpoint(url)); return open.call(this, method, url, ...args); };
      const send = proto.send;
      proto.send = function (...args) {
        const endpoint = urls.get(this);
        if (endpoint) this.addEventListener('load', () => {
          try { captureZing(endpoint, this.responseType === 'json' ? this.response : JSON.parse(this.responseText)); } catch { /* Ignore unrelated/error responses. */ }
        }, { once: true });
        return send.apply(this, args);
      };
    }
  }
  const readZing = () => {
    if (!isZing) return null;
    let queue = [], current = null;
    try {
      // This exact key contains Zing's persisted queue, including when its sidebar is unmounted.
      const raw = window.localStorage?.getItem('zmp3_np_queue.v1.1');
      const saved = raw && raw.length <= 2_000_000 ? JSON.parse(raw) : null;
      if (saved?.itemMap && Array.isArray(saved.ids)) {
        queue = saved.ids.slice(0, 100).map((id) => zingTrack(saved.itemMap[id])).filter(Boolean);
        current = zingTrack(saved.itemMap[saved.currentId]);
      }
    } catch { /* DOM/API metadata remains available if storage is blocked. */ }
    if (metadata?.title && current?.title !== metadata.title) current = [...zingTracks.values()].find((item) => item.title === metadata.title) ?? null;
    if (!queue.length && zingPlaylist?.items.some((item) => item.title === metadata?.title)) queue = zingPlaylist.items;
    if (current && !current.playlistUrl) current.playlistUrl = urlOf(document.querySelector('.player-controls .song-title-item a')?.getAttribute('href')) || zingPlaylist?.url;
    return { queue, current };
  };
  const requestQueue = () => {
    if (isZing && !document.querySelector('.player-queue__list')) document.querySelector('.queue-expand-button')?.click();
    if (isSpotify) {
      const button = document.querySelector('[data-testid="control-button-queue"]');
      if (button?.getAttribute('aria-pressed') !== 'true') button?.click();
    }
    report();
    setTimeout(report, 300);
  };
  // `zing` may be passed in so one report parses Zing's stored queue only once.
  const readQueue = (zing = readZing()) => {
    queueNodes.clear();
    pageNodes.clear();
    if (zing) {
      for (const item of zing.queue) queueNodes.set(item.url, { title: item.title, node: null });
      // Queue rows contain links; playlist-page titles are spans, so map them through observed API metadata.
      const rows = document.querySelectorAll('.player-queue__list .media-item, .playlist-content .media-item');
      for (const row of rows) {
        const title = row.querySelector('figure[title]')?.getAttribute('title');
        const anchor = row.querySelector('a[href*="/bai-hat/"]');
        const candidates = (zingPlaylist?.items ?? [...zingTracks.values()]).filter((item) => item.title === title);
        const known = candidates.length === 1 ? candidates[0] : null;
        const url = anchor ? urlOf(anchor.getAttribute('href')) : known?.url;
        if (url && title) {
          const entry = { title, node: row.querySelector('button.action-play') || anchor };
          pageNodes.set(url, entry);
          if (queueNodes.has(url) || !zing.queue.length) queueNodes.set(url, entry);
        }
      }
      if (queueNodes.size) return [...queueNodes].slice(0,100).map(([url, item]) => ({ url, title: item.title }));
    }
    const selectors = [
      'ytd-playlist-panel-video-renderer a#wc-endpoint', 'ytmusic-playlist-panel-video-renderer a[href*="watch?"]',
      '.player-queue .media-item a[href*="/bai-hat/"]', '.playlist-section .media-item a[href*="/bai-hat/"]',
      '.list-song-item a[href*="/bai-hat/"]', '[data-testid="tracklist-row"] a[href*="/track/"]',
      '.queue__item a[href]', '.trackItem a.trackItem__trackTitle', '.soundList__item a.soundTitle__title', '.list_song_in_album a.name_song',
      'main [role="row"] a[href*="/track/"]',
    ];
    for (const node of document.querySelectorAll(selectors.join(','))) {
      const url = urlOf(node.getAttribute('href'));
      const title = clip(node.getAttribute('title') || node.querySelector('#video-title')?.textContent || node.innerText || node.textContent)?.trim();
      if (url && title && !queueNodes.has(url)) queueNodes.set(url, { node, title });
      if (queueNodes.size >= 100) break;
    }
    for (const [url, item] of queueNodes) knownTracks.set(item.title, url);
    if (knownTracks.size > 500) knownTracks.delete(knownTracks.keys().next().value);
    // Spotify's queue uses text labels, not track links. Prefer a previously observed
    // public track URL; otherwise use a local row bookmark, never a fabricated track ID.
    if (isSpotify) {
      const queued = new Map();
      for (const row of document.querySelectorAll('[role="tabpanel"] [data-encore-id="listRow"]')) {
        const label = row.getAttribute('aria-labelledby');
        const title = clip(label && document.getElementById(label)?.textContent)?.trim();
        const node = row.querySelector('[data-testid="play-button"]');
        if (title && node) queued.set(knownTracks.get(title) || `${location.origin}/#pcisland-queue=${encodeURIComponent(label)}`, { title, node });
        if (queued.size >= 100) break;
      }
      if (queued.size) { queueNodes.clear(); for (const entry of queued) queueNodes.set(...entry); }
    }
    return [...queueNodes].map(([url, item]) => ({ url, title: item.title }));
  };

  const invoke = (cmd, args) => window.__TAURI_INTERNALS__?.invoke(cmd, args);

  const activeMedia = () => {
    const all = Array.from(document.querySelectorAll('audio, video')).concat(Array.from(tracked).reverse());
    return all.find((m) => !m.paused && !m.ended) || all.find((m) => m.currentSrc) || null;
  };

  const pickArtwork = (art) => {
    if (!Array.isArray(art) || art.length === 0) return null;
    const best = art[art.length - 1];
    try {
      const url = new URL(best.src, location.href);
      // Signed CDN URLs can exceed metadata's 512-character limit; never truncate them.
      return url.protocol === 'https:' && url.href.length <= 8192 ? url.href : null;
    } catch {
      return null;
    }
  };

  const snapshot = () => {
    const media = activeMedia();
    const zing = readZing();
    const playing = media ? !media.paused && !media.ended : sessionState === 'playing';
    const hasSomething = metadata || media;
    const queue = readQueue(zing);
    const currentLink = document.querySelector('.playbackSoundBadge__titleLink, [data-testid="now-playing-widget-v2"] a');
    const trackUrl = zing?.current?.url || (isSpotify && knownTracks.get(metadata?.title)) || urlOf(currentLink?.getAttribute('href'));
    const soundContext = isSoundCloud && trackUrl ? new URL(trackUrl).searchParams.get('in') : null;
    return {
      kind: 'state',
      pageUrl: zing?.current?.playlistUrl || (soundContext && urlOf('/' + soundContext)) || (isSpotify && urlOf(currentLink?.getAttribute('href'))) || location.href,
      trackUrl: trackUrl || location.href,
      queue,
      repeat: repeatMode,
      repeatError,
      title: clip(metadata?.title || document.title || ''),
      artist: clip(metadata?.artist || null),
      album: clip(metadata?.album || null),
      artwork: pickArtwork(metadata?.artwork),
      status: !hasSomething ? 'none' : playing ? 'playing' : 'paused',
      positionMs: media && Number.isFinite(media.currentTime) ? Math.round(media.currentTime * 1000) : null,
      durationMs: media && Number.isFinite(media.duration) ? Math.round(media.duration * 1000) : null,
      rate: media ? media.playbackRate : 1,
      isAd: !!document.querySelector('.ad-showing'),
      caps: {
        next: handlers.has('nexttrack'),
        previous: handlers.has('previoustrack'),
        seek: handlers.has('seekto') || !!media,
      },
    };
  };

  // Coalesce bursts: at most one report per 100 ms (≤ 10 msg/s).
  const report = () => {
    if (pending) return;
    const wait = Math.max(0, 100 - (Date.now() - lastSent));
    pending = setTimeout(() => {
      pending = null;
      lastSent = Date.now();
      invoke('web_player_report', { msg: snapshot() })?.catch(() => {});
    }, wait);
  };

  // --- Media Session hooks ---
  if ('mediaSession' in navigator) {
    const proto = Object.getPrototypeOf(navigator.mediaSession);
    const meta = Object.getOwnPropertyDescriptor(proto, 'metadata');
    if (meta?.set) {
      Object.defineProperty(proto, 'metadata', {
        configurable: true,
        get() {
          return meta.get.call(this);
        },
        set(v) {
          metadata = v ? { title: v.title, artist: v.artist, album: v.album, artwork: v.artwork } : null;
          meta.set.call(this, v);
          report();
        },
      });
    }
    const state = Object.getOwnPropertyDescriptor(proto, 'playbackState');
    if (state?.set) {
      Object.defineProperty(proto, 'playbackState', {
        configurable: true,
        get() {
          return state.get.call(this);
        },
        set(v) {
          sessionState = v;
          state.set.call(this, v);
          report();
        },
      });
    }
    const setAction = proto.setActionHandler;
    proto.setActionHandler = function (action, handler) {
      if (handler) handlers.set(action, handler);
      else handlers.delete(action);
      return setAction.call(this, action, handler);
    };
  }

  // --- Media elements, including ones never attached to the DOM ---
  const onEnded = (event) => {
    if (repeatMode === null || handledEnds.has(event) || accessBlocked() || document.querySelector('.ad-showing')) return;
    const media = event.target;
    if (!media || media !== activeMedia()) return;
    const state = snapshot();
    let index = state.queue.findIndex((item) => item.url === state.trackUrl);
    if (index < 0) index = state.queue.findIndex((item) => item.title === state.title);
    if (repeatMode !== 'one' && index < 0) {
      if (repeatMode === 'all') { repeatError = true; report(); }
      return;
    }
    handledEnds.add(event);
    event.stopImmediatePropagation();
    if (repeatMode === 'one') {
      media.currentTime = 0;
      media.play()?.catch(() => { repeatError = true; report(); });
    } else {
      const next = state.queue[index + 1] || (repeatMode === 'all' ? state.queue[0] : null);
      if (next && state.queue.length === 1) {
        media.currentTime = 0;
        media.play()?.catch(() => { repeatError = true; report(); });
      } else if (next) select(next.url);
      else media.pause();
    }
    report();
  };
  // Capture before site handlers advance the queue; detached Audio has its own listener.
  window.addEventListener('ended', onEnded, true);
  const play = HTMLMediaElement.prototype.play;
  HTMLMediaElement.prototype.play = function (...args) {
    if (repeatMode !== null && !document.querySelector('.ad-showing')) this.loop = repeatMode === 'one';
    if (!tracked.has(this)) {
      tracked.add(this);
      this.addEventListener?.('ended', onEnded, true);
      for (const type of ['play', 'pause', 'ended', 'durationchange', 'ratechange', 'seeked', 'loadedmetadata']) this.addEventListener?.(type, report);
      // Audio created with new Audio() may never enter the DOM; document listeners
      // do not receive its timeupdate events, so save its progress here as well.
      this.addEventListener?.('timeupdate', () => {
        if (Date.now() - lastPositionReport >= 5000) { lastPositionReport = Date.now(); report(); }
      });
      if (tracked.size > 64) tracked.delete(tracked.values().next().value);
    }
    return play.apply(this, args);
  };
  for (const type of ['play', 'pause', 'ended', 'durationchange', 'ratechange', 'seeked', 'loadedmetadata']) {
    document.addEventListener(type, report, true);
  }
  document.addEventListener('timeupdate', () => {
    if (Date.now() - lastPositionReport >= 5000) { lastPositionReport = Date.now(); report(); }
  }, true);
  window.addEventListener('popstate', report);

  const exec = (action) => {
    if ((action === 'play' || action === 'play_pause') && accessBlocked()) return false;
    if (action === 'play' && activeMedia()?.paused === false) return true;
    const handler = handlers.get(action === 'play_pause' ? (activeMedia()?.paused === false ? 'pause' : 'play') : action);
    if (handler) {
      handler({ action });
      return true;
    }
    // Some sites restore a paused badge before creating their audio element or handlers.
    const button = isSoundCloud ? document.querySelector('.playControls__play') : isSpotify ? document.querySelector('[data-testid="control-button-playpause"]') : null;
    if (button && !button.disabled) {
      const playing = isSoundCloud ? button.classList.contains('playing') : button.getAttribute('aria-label') === 'Pause';
      if (action === 'play_pause' || action === 'play' && !playing || action === 'pause' && playing) { button.click(); return true; }
    }
    const media = activeMedia();
    if (media) {
      if (action === 'play' || action === 'play_pause' && media.paused) media.play()?.catch(() => {});
      else if (action === 'pause' || action === 'play_pause') media.pause();
      return true;
    }
    return false;
  };

  const select = (url, attempt = 0) => {
    if (accessBlocked()) return;
    if ((isZing || isSpotify) && attempt === 0) requestQueue();
    readQueue();
    const item = pageNodes.get(url) || queueNodes.get(url);
    if (item?.node) {
      const row = item.node.closest('[data-testid="tracklist-row"], [role="row"], .media-item, .list-song-item, .queue__item, .trackItem, .sound');
      const playButton = row?.querySelector('button.action-play, [data-testid="play-button"], button[aria-label^="Play"], .playButton, .icon.ic-play, .play');
      (playButton || item.node).click();
      setTimeout(() => { if (snapshot().trackUrl === url) exec('play'); }, 1000);
    } else if ((isZing || isSpotify) && attempt < 8) setTimeout(() => select(url, attempt + 1), 250);
    else if (repeatMode !== null) { repeatError = true; report(); }
  };
  const setRepeat = (mode, restoreMode = false) => {
    if (!['off', 'all', 'one'].includes(mode)) return false;
    const media = activeMedia();
    if (restoreMode && !media && !accessBlocked()) { repeatMode = mode; report(); return true; }
    repeatError = accessBlocked() || !media || !Number.isFinite(media.duration) || !!document.querySelector('.ad-showing');
    if (!repeatError && mode === 'all') {
      requestQueue();
      const state = snapshot();
      repeatError = !state.queue.some((item) => item.url === state.trackUrl || item.title === state.title);
    }
    if (!repeatError) {
      repeatMode = mode;
      media.loop = mode === 'one';
    }
    report();
    return !repeatError;
  };
  const navigate = (url) => {
    try {
      const target = new URL(url);
      if (target.protocol !== 'https:') return;
      exec('pause');
      appNavigation = true;
      location.assign(target.href);
    } catch { /* Invalid navigation is rejected by the native caller too. */ }
  };
  const resume = (saved) => {
    restore = { ...saved, attempts: 0, activated: false };
    const attempt = () => {
      if (!restore || ++restore.attempts > 28) return;
      if (accessBlocked()) { restore = null; return; }
      const media = activeMedia();
      const actualUrl = snapshot().trackUrl;
      const expectedUrl = urlOf(restore.trackUrl);
      const matching = metadata?.title?.trim() === restore.title?.trim() && (!expectedUrl || actualUrl === expectedUrl);
      if (media && matching && media.readyState >= 1) {
        if (Number.isFinite(media.duration)) media.currentTime = Math.min(restore.positionMs / 1000, Math.max(0, media.duration - 1));
        if (media.paused) exec('play'); restore = null; report(); return;
      }
      const queue = readQueue();
      const fallback = !expectedUrl && queue.find((item) => item.title === restore.title);
      const selectionUrl = expectedUrl || fallback?.url;
      const item = pageNodes.get(selectionUrl) || queueNodes.get(selectionUrl);
      if (restore.attempts === 1 && item && !item.node) requestQueue();
      // One activation per restore. Poll readiness, never repeatedly click Play.
      if (!restore.activated) {
        if (matching) restore.activated = exec('play');
        else if (item?.node) { select(selectionUrl); restore.activated = true; }
        else if (urlOf(restore.trackUrl) === location.href) {
          const button = isZing ? document.querySelector('.song-header button.btn-play-all') : isSoundCloud ? document.querySelector('.soundTitle__playButtonHero .playButton') : isSpotify ? document.querySelector('main [data-testid="play-button"]') : null;
          if (button) { button.click(); restore.activated = true; }
        }
      }
      if (!restore.activated && !restore.navigated && restore.attempts === 8 && restore.trackUrl && restore.trackUrl !== location.href) {
        const url = urlOf(restore.trackUrl);
        if (url) { restore.navigated = true; sessionStorage.setItem('pcisland.resume', JSON.stringify(restore)); navigate(url); return; }
      }
      restoreTimer = setTimeout(attempt, 1000);
    };
    if (restoreTimer) clearTimeout(restoreTimer);
    attempt();
  };

  const cancelResume = () => {
    restore = null;
    if (restoreTimer) clearTimeout(restoreTimer);
    window.sessionStorage?.removeItem('pcisland.resume');
  };

  Object.defineProperty(window, '__pcisland', {
    value: Object.freeze({ exec, select, resume, cancelResume, requestQueue, navigate, setRepeat }),
    enumerable: false,
    configurable: false,
    writable: false,
  });
  window.addEventListener('pagehide', () => invoke('web_player_report', { msg: { ...snapshot(), status: 'none' } }));
  try {
    const saved = window.sessionStorage?.getItem('pcisland.resume');
    if (saved) { window.sessionStorage.removeItem('pcisland.resume'); resume(JSON.parse(saved)); }
  } catch { /* Storage may be unavailable before a site's document is ready. */ }
  document.addEventListener('DOMContentLoaded', report, { once: true });
})();
