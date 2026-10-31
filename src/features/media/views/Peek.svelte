<script lang="ts">
  import type { NowPlayingDto } from '$lib/ipc/bindings/NowPlayingDto';
  import { mediaCommands } from '$lib/ipc/commands';
  import { t } from '$lib/i18n/index.svelte';
  import { Music, Pause, Play, SkipBack, SkipForward } from '@lucide/svelte';
  import { formatTime, positionAt } from '../store.svelte';
  import { live } from '$lib/stores/live.svelte';

  let { np }: { np: NowPlayingDto } = $props();
  let now = $state(Date.now());

  // Progress only ticks while the peek is mounted and the track plays.
  $effect(() => {
    if (np.status !== 'playing') return;
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
      if (x.timeMs <= position) line = x.text;
      else break;
    }
    return line;
  });
  const subtitle = $derived([np.artist, np.appName].filter(Boolean).join(' · '));
</script>

<div class="peek">
  <div class="top">
    {#if np.artworkUrl}
      <img class="art" src={np.artworkUrl} alt="" />
    {:else}
      <span class="art placeholder"><Music size={24} /></span>
    {/if}
    <div class="text">
      <div class="title">{np.isAd ? t('media.peek.ad') : np.title}</div>
      <div class="subtitle">{subtitle}</div>
    </div>
  </div>

  {#if lyric}<div class="lyric">{lyric}</div>{/if}

  {#if progress !== null && position !== null && np.durationMs}
    <div class="progress">
      <span class="time tabular">{formatTime(position)}</span>
      <div class="track"><div class="fill" style:transform={`scaleX(${progress})`}></div></div>
      <span class="time tabular">{formatTime(np.durationMs)}</span>
    </div>
  {/if}

  <div class="controls">
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
</div>

<style>
  .peek {
    display: flex;
    flex-direction: column;
    gap: var(--sp-5);
    padding: var(--sp-6) var(--sp-7) var(--sp-5);
    height: 100%;
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
    color: var(--c-text-2);
    font-size: var(--fs-small);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .lyric {
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
  .controls {
    display: flex;
    justify-content: center;
    gap: var(--sp-6);
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
