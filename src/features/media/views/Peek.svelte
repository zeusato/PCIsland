<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { NowPlayingDto } from '$lib/ipc/bindings/NowPlayingDto';
  import { lyricsCommands, mediaCommands } from '$lib/ipc/commands';
  import { t } from '$lib/i18n/index.svelte';
  import Music from '@lucide/svelte/icons/music';
  import Pause from '@lucide/svelte/icons/pause';
  import Play from '@lucide/svelte/icons/play';
  import SkipBack from '@lucide/svelte/icons/skip-back';
  import SkipForward from '@lucide/svelte/icons/skip-forward';
  import { formatTime, media, positionAt } from '../store.svelte';
  import { live } from '$lib/stores/live.svelte';

  // `trailing`: extra buttons on the right of the transport row (video window controls).
  let { np, active = true, trailing }: { np: NowPlayingDto; active?: boolean; trailing?: Snippet } = $props();
  let now = $state(Date.now());
  let failedArtwork = $state('');

  // Progress only ticks while the peek is mounted and the track plays.
  $effect(() => {
    if (!active || np.status !== 'playing') return;
    const id = setInterval(() => (now = Date.now()), 250);
    return () => clearInterval(id);
  });

  const position = $derived(positionAt(np, now));
  const progress = $derived(
    position !== null && np.durationMs ? Math.min(1, position / np.durationMs) : null,
  );
  // Current synced lyric line (docs/modules/lyrics-visualizer.md §A3).
  const lyric = $derived.by(() => {
    const l = live.lyrics;
    if (!l?.synced || position === null || !live.settings?.lyrics.enabled) return null;
    let line: string | null = null;
    for (const x of l.lines) {
      if (x.timeMs + l.offsetMs <= position) line = x.text;
      else break;
    }
    return line;
  });
  const subtitle = $derived(np.artist ?? '');
  // Several players at once: the chip cycles which one the island follows.
  const sources = $derived(media.snapshot?.sources ?? []);
  function nextSource() {
    if (sources.length < 2) return;
    const i = sources.findIndex((s) => s.sourceId === np.sourceId);
    const next = sources[(i + 1) % sources.length];
    if (next) mediaCommands.selectSource(next.sourceId);
  }
</script>

<div class="peek">
  <div class="top">
    {#if np.artworkUrl && np.artworkUrl !== failedArtwork}
      <img class="art" src={np.artworkUrl} alt="" onerror={(event) => failedArtwork = event.currentTarget.getAttribute('src') ?? ''} />
    {:else}
      <span class="art placeholder"><Music size={24} /></span>
    {/if}
    <div class="text">
      <div class="title">{np.isAd ? t('media.peek.ad') : np.title}</div>
      <div class="subtitle">
        {#if sources.length > 1}
          <button class="chip" title={t('media.peek.switchSource')} onclick={nextSource}>{np.appName} · {sources.length}</button>
        {:else}
          <span class="app">{np.appName}</span>
        {/if}
        {#if subtitle}<span>{subtitle}</span>{/if}
      </div>
    </div>
  </div>

  {#if lyric}
    <div class="lyric-row">
      <button class="nudge" aria-label={t('lyrics.earlier')} title={t('lyrics.earlier')} onclick={() => lyricsCommands.adjustOffset(-500)}>−</button>
      <div class="lyric">{lyric}</div>
      <button class="nudge" aria-label={t('lyrics.later')} title={t('lyrics.later')} onclick={() => lyricsCommands.adjustOffset(500)}>+</button>
    </div>
  {/if}

  {#if progress !== null && position !== null && np.durationMs}
    <div class="progress">
      <span class="time tabular">{formatTime(position)}</span>
      <div class="track"><div class="fill" style:transform={`scaleX(${progress})`}></div></div>
      <span class="time tabular">{formatTime(np.durationMs)}</span>
    </div>
  {/if}

  <div class="controls">
    <span></span>
    <div class="transport">
    <button aria-label={t('media.peek.previous')} disabled={!np.caps.previous} onclick={() => mediaCommands.previous()}>
      <SkipBack size={18} />
    </button>
    <button class="primary" aria-label={t('media.peek.playPause')} onclick={() => mediaCommands.playPause()}>
      {#if np.status === 'playing'}<Pause size={20} />{:else}<Play size={20} />{/if}
    </button>
    <button aria-label={t('media.peek.next')} disabled={!np.caps.next} onclick={() => mediaCommands.next()}>
      <SkipForward size={18} />
    </button>
    </div>
    <div class="trailing">{#if trailing}{@render trailing()}{/if}</div>
  </div>
</div>

<style>
  .peek {
    display: flex;
    flex-direction: column;
    gap: var(--sp-5);
    padding: var(--sp-6) var(--sp-7) var(--sp-5);
    height: 100%;
    /* Long titles (YouTube) must ellipsize, not widen the island. */
    width: 100%;
    min-width: 0;
    box-sizing: border-box;
  }
  .top {
    display: flex;
    gap: var(--sp-5);
    align-items: center;
    min-width: 0;
  }
  .art {
    width: 56px;
    height: 56px;
    border-radius: 12px;
    object-fit: cover;
    flex: none;
  }
  .placeholder {
    display: grid;
    place-items: center;
    background: var(--c-surface-3);
    color: var(--c-text-2);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .title {
    font-size: var(--fs-title-s);
    font-weight: var(--fw-semibold);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .subtitle {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    color: var(--c-text-2);
    font-size: var(--fs-small);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .subtitle > span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .app {
    flex: none;
    color: var(--c-text-3);
  }
  .subtitle .chip {
    display: inline-flex;
    flex: none;
    width: auto;
    height: 18px;
    padding: 0 var(--sp-3);
    border-radius: var(--r-pill);
    background: var(--c-accent-soft);
    color: var(--c-accent-bright);
    font-size: var(--fs-caption);
  }
  .lyric-row {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    min-width: 0;
  }
  .lyric-row .nudge {
    width: 20px;
    height: 20px;
    flex: none;
    color: var(--c-text-3);
    font-size: var(--fs-small);
    opacity: 0;
    transition: opacity 120ms;
  }
  .lyric-row:hover .nudge {
    opacity: 1;
  }
  .lyric {
    flex: 1;
    min-width: 0;
    color: var(--c-accent-bright);
    font-size: var(--fs-small);
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .progress {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
  }
  .time {
    color: var(--c-text-3);
    font-size: var(--fs-caption);
  }
  .track {
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: var(--c-surface-3);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--c-accent);
    transform-origin: 0 50%;
  }
  /* Three columns keep the transport centred whether or not trailing buttons exist. */
  .controls {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
  }
  .transport {
    display: flex;
    gap: var(--sp-6);
  }
  .trailing {
    display: flex;
    justify-content: flex-end;
    min-width: 0;
  }
  button {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: 50%;
    background: transparent;
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    background: var(--c-surface-3);
  }
  button:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .primary {
    background: var(--c-accent-soft);
    color: var(--c-accent-bright);
  }
</style>
