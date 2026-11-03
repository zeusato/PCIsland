<script lang="ts">
  // Floating video player page (docs/modules/video-player.md §5.5). Plays a proxied
  // `vstream://` source: HLS (hls.js), a video+audio pair synced on the audio clock, a
  // single file, or the site's embed (fallback A). Reports state for the island.
  import { onDestroy, onMount } from 'svelte';
  import Hls from 'hls.js';
  import Pin from '@lucide/svelte/icons/pin';
  import PinOff from '@lucide/svelte/icons/pin-off';
  import Minimize2 from '@lucide/svelte/icons/minimize-2';
  import ExternalLink from '@lucide/svelte/icons/external-link';
  import X from '@lucide/svelte/icons/x';
  import Play from '@lucide/svelte/icons/play';
  import Pause from '@lucide/svelte/icons/pause';
  import Volume2 from '@lucide/svelte/icons/volume-2';
  import VolumeX from '@lucide/svelte/icons/volume-x';
  import Maximize from '@lucide/svelte/icons/maximize';
  import Minimize from '@lucide/svelte/icons/minimize';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import { inTauri } from '$lib/ipc/commands';
  import { onVideoCommand, onVideoSource, onVideoState, videoCommands, videoWindow } from '$lib/ipc/video';
  import type { PlaybackSource } from '$lib/ipc/bindings/PlaybackSource';
  import type { VideoCommand } from '$lib/ipc/bindings/VideoCommand';
  import type { VideoStatus } from '$lib/ipc/bindings/VideoStatus';
  import { getLocale, t } from '$lib/i18n/index.svelte';
  import { languageLabel, manifestLanguage, preferredTrack } from './languages';
  import { formatTime, syncStep } from './sync';

  let source = $state<PlaybackSource | null>(null);
  let videoEl: HTMLVideoElement | undefined = $state();
  let audioEl: HTMLAudioElement | undefined = $state();
  let hls: Hls | null = null;
  let status = $state<VideoStatus>('loading');
  let position = $state(0);
  let duration = $state(0);
  let muted = $state(false);
  let volume = $state(1);
  let pinned = $state(false);
  let fullscreen = $state(false);
  let controls = $state(true);
  let error = $state<string | null>(null);
  /** "Continuing from 3:17" banner, shown for a few seconds after a resume. */
  let resumedFrom = $state<number | null>(null);
  let resumeTimer: ReturnType<typeof setTimeout> | undefined;
  let hlsLevels = $state<number[]>([]);
  let hlsQuality = $state<number | null>(null);
  let hlsLanguages = $state<string[]>([]);
  let selectedLanguage = $state('');
  let preferredLanguage = 'vi';
  let metadataEvents: AbortController | undefined;
  const languages = $derived(source?.kind === 'hls' ? hlsLanguages : Object.keys(source?.audioLanguages ?? {}));
  let wantPlay = true;
  let refreshing = false;
  let lastReport = 0;
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  let syncTimer: ReturnType<typeof setInterval> | undefined;
  // Bookkeeping only (never rendered), so a plain Set is intended.
  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  const waiting = new Set<HTMLMediaElement>();

  const isPair = $derived(source?.kind === 'pair');
  const isEmbed = $derived(source?.kind === 'embed');
  /** The element whose clock and sound the user hears. */
  const clock = (): HTMLMediaElement | undefined => (isPair ? audioEl : videoEl);
  const qualities = $derived(source?.kind === 'hls'
    ? [...new Set([...hlsLevels, ...source.qualities])].sort((a, b) => b - a)
    : (source?.qualities ?? []));
  const currentQuality = $derived(source?.kind === 'hls' ? hlsQuality : (source?.quality ?? null));

  function report(force = false) {
    if (!inTauri || !source || isEmbed) return;
    const now = Date.now();
    if (!force && now - lastReport < 5000) return;
    lastReport = now;
    void videoCommands
      .report({ status, positionMs: position * 1000, durationMs: duration > 0 ? duration * 1000 : null, muted })
      .catch(() => {});
  }
  function setStatus(next: VideoStatus) {
    if (status !== next) {
      status = next;
      report(true);
    }
  }

  function teardown() {
    metadataEvents?.abort();
    hls?.destroy();
    hls = null;
    hlsLevels = [];
    hlsQuality = null;
    hlsLanguages = [];
    selectedLanguage = '';
    clearInterval(syncTimer);
    waiting.clear();
    for (const el of [videoEl, audioEl]) {
      if (!el) continue;
      el.pause();
      el.removeAttribute('src');
      el.load();
    }
  }

  async function playBoth() {
    wantPlay = true;
    const c = clock();
    if (!c) return;
    try {
      await c.play();
      if (isPair) await videoEl?.play();
    } catch {
      setStatus('paused');
    }
  }
  function pauseBoth() {
    wantPlay = false;
    videoEl?.pause();
    audioEl?.pause();
  }
  function seekTo(seconds: number) {
    const target = Math.max(0, duration > 0 ? Math.min(seconds, duration - 0.5) : seconds);
    const c = clock();
    if (!c) return;
    c.currentTime = target;
    if (isPair && videoEl) videoEl.currentTime = target;
    position = target;
    report(true);
  }

  function load(next: PlaybackSource, language = 'vi', height = next.quality) {
    teardown();
    preferredLanguage = language;
    error = null;
    // Only a fresh open announces the resume; renewals keep playing silently.
    if (next.resumedFromMs !== null && next.rev !== source?.rev) {
      resumedFrom = next.resumedFromMs;
      clearTimeout(resumeTimer);
      resumeTimer = setTimeout(() => (resumedFrom = null), 8000);
    } else if (next.resumedFromMs === null) resumedFrom = null;
    source = next;
    position = next.startMs / 1000;
    duration = next.durationMs ? next.durationMs / 1000 : 0;
    status = 'loading';
    if (next.kind === 'embed') return;
    const v = videoEl;
    const a = audioEl;
    if (!v || !a) return;
    const start = next.startMs / 1000;
    if (next.kind === 'hls') {
      if (!Hls.isSupported()) {
        error = t('video.error.playback');
        return;
      }
      // Workers need blob: in the CSP; the main thread is fast enough for one stream.
      hls = new Hls({ enableWorker: false, startPosition: start > 0 ? start : -1, maxBufferLength: 30 });
      const player = hls;
      player.on(Hls.Events.AUDIO_TRACKS_UPDATED, (_event, { audioTracks }) => {
        hlsLanguages = [...new Set(audioTracks.map(manifestLanguage))];
        const preferred = preferredTrack(audioTracks, manifestLanguage, preferredLanguage);
        if (preferred && player.audioTrack !== preferred.id) player.audioTrack = preferred.id;
      });
      player.on(Hls.Events.AUDIO_TRACK_SWITCHING, (_event, track) => {
        selectedLanguage = manifestLanguage(track);
      });
      hls.on(Hls.Events.MANIFEST_PARSED, () => {
        if (!hls) return;
        const cap = next.autoMaxHeight;
        // Highest level whose height fits the cap (`video.max_height`).
        let capIndex = -1;
        let capHeight = -1;
        hls.levels.forEach((l, i) => {
          if (l.height <= cap && l.height >= capHeight) {
            capIndex = i;
            capHeight = l.height;
          }
        });
        hls.autoLevelCapping = Math.max(0, capIndex);
        hlsLevels = hls.levels.map((l) => l.height).filter((h) => h > 0);
        const index = hls.levels.findIndex((l) => l.height === height);
        hlsQuality = index < 0 ? null : height;
        if (index >= 0) hls.currentLevel = index;
        if (wantPlay) void playBoth();
        else setStatus('paused');
      });
      hls.on(Hls.Events.ERROR, (_e, data) => {
        if (!data.fatal) return;
        if (data.type === Hls.ErrorTypes.NETWORK_ERROR) void refresh();
        else if (data.type === Hls.ErrorTypes.MEDIA_ERROR) hls?.recoverMediaError();
        else error = t('video.error.playback');
      });
      hls.loadSource(next.video);
      hls.attachMedia(v);
      v.muted = muted;
      v.volume = volume;
      return;
    }
    v.src = next.video;
    if (next.kind === 'pair' && next.audio) {
      const tracks = Object.entries(next.audioLanguages);
      const track = preferredTrack(tracks, ([tag]) => tag, language) ?? tracks.find(([, url]) => url === next.audio);
      selectedLanguage = track?.[0] ?? '';
      a.src = track?.[1] ?? next.audio;
      v.muted = true;
      a.muted = muted;
      a.volume = volume;
      syncTimer = setInterval(syncTick, 250);
    } else {
      v.muted = muted;
      v.volume = volume;
    }
    const ready = () => {
      if (start > 0) seekTo(start);
      if (wantPlay) void playBoth();
      else setStatus('paused');
    };
    metadataEvents = new AbortController();
    (next.kind === 'pair' ? a : v).addEventListener('loadedmetadata', ready, { once: true, signal: metadataEvents.signal });
  }

  // Pair: audio is the clock; the video follows (spike S2).
  function syncTick() {
    const v = videoEl;
    const a = audioEl;
    if (!v || !a) return;
    if (document.hidden && wantPlay && a.paused && !waiting.has(a)) void a.play().catch(() => {});
    if (a.paused || v.paused || waiting.size || document.hidden) {
      v.playbackRate = 1;
      return;
    }
    const step = syncStep(v.currentTime - a.currentTime);
    if (step.kind === 'jump') {
      v.currentTime = a.currentTime;
      v.playbackRate = 1;
    } else v.playbackRate = step.kind === 'rate' ? step.rate : 1;
  }

  function onWaiting(el: HTMLMediaElement) {
    // Hidden window: Chromium suspends video-only playback; keep the audio going.
    if (isPair && el === videoEl && document.hidden) return;
    waiting.add(el);
    setStatus('loading');
    if (isPair) {
      if (el === videoEl) audioEl?.pause();
      else videoEl?.pause();
    }
  }
  function onReady(el: HTMLMediaElement) {
    if (!waiting.delete(el) || waiting.size) return;
    if (wantPlay) void playBoth();
  }
  function onVisibility() {
    if (!isPair || document.hidden || !videoEl || !audioEl) return;
    waiting.delete(videoEl);
    videoEl.currentTime = audioEl.currentTime;
    if (wantPlay && !audioEl.paused) void videoEl.play().catch(() => {});
  }

  async function refresh() {
    if (refreshing || !inTauri) return;
    refreshing = true;
    try {
      const rev = source?.rev;
      const quality = currentQuality;
      const next = await videoCommands.refresh(position * 1000);
      if (rev === source?.rev) load(next, preferredLanguage, quality);
    } catch {
      error = t('video.error.expired');
    } finally {
      refreshing = false;
    }
  }
  function onMediaError(el: HTMLMediaElement) {
    if (el.error && (el.error.code === MediaError.MEDIA_ERR_NETWORK || el.error.code === MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED)) void refresh();
    else error = t('video.error.playback');
    setStatus('error');
  }

  async function chooseQuality(value: string) {
    const height = value === 'auto' ? null : Number(value);
    if (source?.kind === 'hls' && hls && (height === null || hlsLevels.includes(height))) {
      hlsQuality = height;
      hls.currentLevel = height === null ? -1 : hls.levels.findIndex((l) => l.height === height);
      return;
    }
    try {
      const rev = source?.rev;
      const next = await videoCommands.setQuality(height, position * 1000);
      if (rev === source?.rev) load(next, preferredLanguage);
    } catch {
      error = t('video.error.quality');
    }
  }

  function chooseLanguage(language: string) {
    if (!source) return;
    if (hls) {
      const track = hls.audioTracks.find((track) => manifestLanguage(track) === language);
      if (track) {
        preferredLanguage = language;
        hls.audioTrack = track.id;
      }
    } else if (source.kind === 'pair') {
      load({ ...source, startMs: (clock()?.currentTime ?? position) * 1000, resumedFromMs: null }, language);
    }
  }

  function toggleMute() {
    muted = !muted;
    const c = clock();
    if (c) c.muted = muted;
    report(true);
  }
  function setVolume(value: number) {
    volume = value;
    const c = clock();
    if (c) {
      c.volume = value;
      if (value > 0 && muted) toggleMute();
    }
  }
  async function toggleFullscreen() {
    fullscreen = !(await videoWindow.isFullscreen());
    await videoWindow.setFullscreen(fullscreen);
  }
  function togglePlay() {
    if (status === 'playing' || wantPlay && status === 'loading') pauseBoth();
    else void playBoth();
  }

  function command(c: VideoCommand) {
    switch (c.action) {
      case 'play_pause': togglePlay(); break;
      case 'pause': pauseBoth(); break;
      case 'play': void playBoth(); break;
      case 'mute': toggleMute(); break;
      case 'back': seekTo(position - 10); break;
      case 'forward': seekTo(position + 10); break;
      case 'seek': if (c.value !== null) seekTo(c.value / 1000); break;
    }
  }

  function poke() {
    controls = true;
    clearTimeout(hideTimer);
    hideTimer = setTimeout(() => { if (status === 'playing') controls = false; }, 2000);
  }

  function onKey(e: KeyboardEvent) {
    if ((e.target as HTMLElement)?.tagName === 'SELECT') return;
    const keys: Record<string, () => void> = {
      ' ': togglePlay,
      ArrowLeft: () => seekTo(position - 5),
      ArrowRight: () => seekTo(position + 5),
      ArrowUp: () => setVolume(Math.min(1, volume + 0.1)),
      ArrowDown: () => setVolume(Math.max(0, volume - 0.1)),
      m: toggleMute,
      M: toggleMute,
      f: () => void toggleFullscreen(),
      F: () => void toggleFullscreen(),
      Escape: () => {
        if (fullscreen) void toggleFullscreen();
        else void videoCommands.window('close');
      },
    };
    const run = keys[e.key];
    if (run) {
      e.preventDefault();
      run();
      poke();
    }
  }

  const subs: Promise<() => void>[] = [];
  onMount(() => {
    document.addEventListener('visibilitychange', onVisibility);
    if (!inTauri) return;
    subs.push(onVideoSource(load), onVideoCommand(command), onVideoState((s) => (pinned = s.pinned)));
    void videoCommands.current().then((s) => { if (s && (!source || s.rev > source.rev)) load(s); });
    void videoCommands.state().then((s) => (pinned = s.pinned)).catch(() => {});
    poke();
  });
  onDestroy(() => {
    document.removeEventListener('visibilitychange', onVisibility);
    teardown();
    clearTimeout(hideTimer);
    clearTimeout(resumeTimer);
    for (const s of subs) void s.then((u) => u());
  });
</script>

<svelte:window onkeydown={onKey} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="player" class:idle={!controls && !isEmbed} onmousemove={poke} onmouseleave={() => { if (status === 'playing') controls = false; }}>
  {#if isEmbed && source}
    <iframe
      class="embed"
      src={source.video}
      title={source.title}
      allow="autoplay; encrypted-media; picture-in-picture; fullscreen"
      referrerpolicy="strict-origin-when-cross-origin"
      allowfullscreen
    ></iframe>
  {/if}
  <video
    bind:this={videoEl}
    class:hidden={isEmbed}
    playsinline
    onclick={togglePlay}
    ondblclick={() => void toggleFullscreen()}
    onplaying={() => { if (!isPair) { waiting.clear(); setStatus('playing'); } }}
    onpause={() => { if (!isPair && !videoEl?.ended) setStatus('paused'); }}
    onwaiting={() => videoEl && onWaiting(videoEl)}
    oncanplay={() => videoEl && onReady(videoEl)}
    onended={() => { if (!isPair) setStatus('ended'); }}
    ontimeupdate={() => { if (!isPair && videoEl) { position = videoEl.currentTime; report(); } }}
    ondurationchange={() => { if (!isPair && videoEl && Number.isFinite(videoEl.duration)) duration = videoEl.duration; }}
    onerror={() => videoEl && onMediaError(videoEl)}
  ></video>
  <audio
    bind:this={audioEl}
    onplaying={() => { waiting.clear(); setStatus('playing'); if (videoEl && !document.hidden) void videoEl.play().catch(() => {}); }}
    onpause={() => { if (isPair && !audioEl?.ended && !waiting.size) setStatus('paused'); }}
    onwaiting={() => audioEl && onWaiting(audioEl)}
    oncanplay={() => audioEl && onReady(audioEl)}
    onended={() => { videoEl?.pause(); setStatus('ended'); }}
    ontimeupdate={() => { if (audioEl) { position = audioEl.currentTime; report(); } }}
    ondurationchange={() => { if (audioEl && Number.isFinite(audioEl.duration)) duration = audioEl.duration; }}
    onerror={() => audioEl && onMediaError(audioEl)}
  ></audio>

  <header class="bar top" data-tauri-drag-region>
    <span class="title" data-tauri-drag-region title={source?.title}>{source?.title ?? t('video.title')}</span>
    {#if languages.length > 1}
      <select class="language" aria-label={t('video.audioLanguage')} title={t('video.audioLanguage')} value={selectedLanguage}
        onchange={(e) => chooseLanguage(e.currentTarget.value)}>
        {#each languages as lang (lang)}<option value={lang}>{languageLabel(lang, getLocale(), t('video.unnamedAudio'))}</option>{/each}
      </select>
    {/if}
    <button title={pinned ? t('video.unpin') : t('video.pin')} aria-label={pinned ? t('video.unpin') : t('video.pin')} class:on={pinned}
      onclick={() => void videoCommands.window(pinned ? 'unpin' : 'pin')}>
      {#if pinned}<PinOff size={14} />{:else}<Pin size={14} />{/if}
    </button>
    <button title={t('video.reset')} aria-label={t('video.reset')} onclick={() => void videoCommands.window('reset')}><Minimize2 size={14} /></button>
    <button title={t('video.openBrowser')} aria-label={t('video.openBrowser')} onclick={() => void videoCommands.openInBrowser()}><ExternalLink size={14} /></button>
    <button title={t('video.close')} aria-label={t('video.close')} onclick={() => void videoCommands.window('close')}><X size={14} /></button>
  </header>

  {#if !isEmbed}
    {#if status === 'loading' && !error}<div class="center" role="status">{t('common.loading')}</div>{/if}
    {#if resumedFrom !== null && !error}
      <div class="resume" role="status">
        <span>{t('video.resumed', { time: formatTime(resumedFrom / 1000) })}</span>
        <button onclick={() => { resumedFrom = null; seekTo(0); }}>{t('video.fromStart')}</button>
      </div>
    {/if}
    {#if error}
      <div class="center error" role="alert">
        <p>{error}</p>
        <div class="row">
          <button onclick={() => source && load(source, preferredLanguage)}><RotateCcw size={14} /> {t('video.retry')}</button>
          {#if source}<button onclick={() => source && void videoCommands.open(source.pageUrl, true)}>{t('video.useEmbed')}</button>{/if}
        </div>
      </div>
    {/if}
    <footer class="bar bottom">
      <button aria-label={status === 'playing' ? t('video.pause') : t('video.play')} onclick={togglePlay}>
        {#if status === 'playing'}<Pause size={16} />{:else}<Play size={16} />{/if}
      </button>
      {#if source?.isLive}
        <span class="live">{t('video.live')}</span>
      {:else}
        <span class="time tabular">{formatTime(position)}</span>
        <input class="seek" type="range" min="0" max={duration || 0} step="0.1" value={position} aria-label={t('video.seek')}
          oninput={(e) => seekTo(Number((e.currentTarget as HTMLInputElement).value))} />
        <span class="time tabular">{formatTime(duration)}</span>
      {/if}
      <button aria-label={muted ? t('video.unmute') : t('video.mute')} onclick={toggleMute}>
        {#if muted || volume === 0}<VolumeX size={16} />{:else}<Volume2 size={16} />{/if}
      </button>
      <input class="volume" type="range" min="0" max="1" step="0.05" value={volume} aria-label={t('video.volume')}
        oninput={(e) => setVolume(Number((e.currentTarget as HTMLInputElement).value))} />
      {#if qualities.length > 1 || source?.kind === 'hls'}
        <select aria-label={t('video.quality')} value={currentQuality === null ? 'auto' : String(currentQuality)}
          onchange={(e) => void chooseQuality((e.currentTarget as HTMLSelectElement).value)}>
          {#if source?.kind === 'hls' || source?.kind === 'pair'}<option value="auto">{t('video.auto')}</option>{/if}
          {#each qualities as q (q)}<option value={String(q)}>{q}p</option>{/each}
        </select>
      {/if}
      <button aria-label={t('video.fullscreen')} onclick={() => void toggleFullscreen()}>
        {#if fullscreen}<Minimize size={16} />{:else}<Maximize size={16} />{/if}
      </button>
    </footer>
  {/if}
</div>

<style>
  :global(body.video-body) {
    margin: 0;
    background: var(--c-island);
    overflow: hidden;
  }
  .player {
    position: fixed;
    inset: 0;
    background: var(--c-island);
    color: var(--c-text-1);
    font-family: var(--font-text);
    font-size: var(--fs-small);
  }
  .player.idle {
    cursor: none;
  }
  video,
  .embed {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    border: 0;
    background: var(--c-island);
    object-fit: contain;
  }
  .hidden {
    display: none;
  }
  .bar {
    position: absolute;
    left: 0;
    right: 0;
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding: var(--sp-2) var(--sp-3);
    transition: opacity 160ms ease;
  }
  .idle .bar {
    opacity: 0;
    pointer-events: none;
  }
  .top {
    top: 0;
    background: linear-gradient(to bottom, var(--c-video-scrim), transparent);
  }
  .bottom {
    bottom: 0;
    background: linear-gradient(to top, var(--c-video-scrim), transparent);
  }
  .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--c-text-2);
    font-size: var(--fs-caption);
  }
  .language {
    max-width: 35%;
    min-width: 0;
  }
  button {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-1);
    padding: var(--sp-1) var(--sp-2);
    border: 0;
    border-radius: var(--r-control);
    background: transparent;
    color: var(--c-text-1);
    cursor: pointer;
    font: inherit;
  }
  button:hover {
    background: var(--c-surface-3);
  }
  button:focus:not(:focus-visible) {
    outline: none;
  }
  button.on {
    color: var(--c-accent-bright);
  }
  .time {
    color: var(--c-text-2);
    font-size: var(--fs-caption);
  }
  input[type='range'] {
    appearance: none;
    height: var(--sp-5);
    margin: 0;
    background: transparent;
    cursor: pointer;
  }
  input[type='range']::-webkit-slider-runnable-track {
    height: 3px;
    border-radius: var(--r-pill);
    background: var(--c-border-strong);
  }
  input[type='range']::-webkit-slider-thumb {
    appearance: none;
    width: 10px;
    height: 10px;
    margin-top: -3.5px;
    border-radius: 50%;
    background: var(--c-accent);
  }
  .seek {
    flex: 1;
    min-width: var(--sp-9);
  }
  .volume {
    width: var(--sp-10);
  }
  select {
    max-width: 72px;
    padding: 0 var(--sp-1);
    background: var(--c-surface-2);
    color: var(--c-text-1);
    border: 1px solid var(--c-border-strong);
    border-radius: var(--r-control);
    font-size: var(--fs-caption);
  }
  /* Default 320 px window: the seek bar gets the room; volume stays on the mute button,
     ↑/↓ and the island. */
  @media (max-width: 479px) {
    .volume {
      display: none;
    }
    .bar {
      gap: var(--sp-1);
      padding: var(--sp-1) var(--sp-2);
    }
  }
  .live {
    flex: 1;
    color: var(--c-danger);
    font-weight: var(--fw-semibold);
    font-size: var(--fs-caption);
  }
  .center {
    position: absolute;
    inset: 0;
    display: grid;
    place-content: center;
    gap: var(--sp-3);
    text-align: center;
    color: var(--c-text-2);
    pointer-events: none;
  }
  .resume {
    position: absolute;
    left: 50%;
    top: var(--sp-9);
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    max-width: calc(100% - var(--sp-6));
    padding: var(--sp-1) var(--sp-1) var(--sp-1) var(--sp-4);
    border-radius: var(--r-pill);
    background: var(--c-video-scrim);
    font-size: var(--fs-caption);
    white-space: nowrap;
  }
  .resume span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .resume button {
    flex: none;
    border-radius: var(--r-pill);
    background: var(--c-accent-soft);
    color: var(--c-accent-bright);
  }
  .error {
    pointer-events: auto;
    background: var(--c-video-scrim);
    padding: var(--sp-4);
  }
  .error p {
    margin: 0;
  }
  .row {
    display: flex;
    gap: var(--sp-2);
    justify-content: center;
  }
  .row button {
    background: var(--c-surface-2);
  }
</style>
