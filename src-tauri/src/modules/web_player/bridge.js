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

  const invoke = (cmd, args) => window.__TAURI_INTERNALS__?.invoke(cmd, args);

  const activeMedia = () => {
    const all = Array.from(document.querySelectorAll('audio, video')).concat(Array.from(tracked));
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
    const playing = media ? !media.paused && !media.ended : sessionState === 'playing';
    const hasSomething = metadata || media;
    return {
      kind: 'state',
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
  const play = HTMLMediaElement.prototype.play;
  HTMLMediaElement.prototype.play = function (...args) {
    tracked.add(this);
    return play.apply(this, args);
  };
  for (const type of ['play', 'pause', 'ended', 'durationchange', 'ratechange', 'seeked', 'loadedmetadata']) {
    document.addEventListener(type, report, true);
  }

  const exec = (action) => {
    const handler = handlers.get(action === 'play_pause' ? (activeMedia()?.paused === false ? 'pause' : 'play') : action);
    if (handler) {
      handler({ action });
      return;
    }
    const media = activeMedia();
    if (action === 'play_pause' && media) {
      if (media.paused) media.play();
      else media.pause();
    }
  };

  Object.defineProperty(window, '__pcisland', {
    value: Object.freeze({ exec }),
    enumerable: false,
    configurable: false,
    writable: false,
  });
  window.addEventListener('pagehide', () => invoke('web_player_report', { msg: { ...snapshot(), status: 'none' } }));
})();
