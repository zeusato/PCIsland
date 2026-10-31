<script lang="ts">
  import type { NowPlayingDto } from '$lib/ipc/bindings/NowPlayingDto';
  import Music from '@lucide/svelte/icons/music';
  import { bars, fresh, viz } from '$lib/stores/visualizer.svelte';
  import { now, useTicker } from '$lib/stores/clock.svelte';
  import { positionAt } from '../store.svelte';
  import { t } from '$lib/i18n/index.svelte';

  let { np }: { np: NowPlayingDto } = $props();
  const playing = $derived(np.status === 'playing');
  let failedArtwork = $state('');
  const position = $derived(positionAt(np, now()));
  const progress = $derived(position !== null && np.durationMs && np.durationMs > 0 ? Math.max(0, Math.min(1, position / np.durationMs)) : null);
  $effect(() => {
    if (playing && np.positionMs !== null && np.durationMs && np.durationMs > 0) return useTicker(250);
  });
  // Real levels while the analyzer runs (F15), CSS waves otherwise.
  const real = $derived(playing && fresh(viz.at + 1) ? bars(4) : null);
</script>

<div class="compact">
  {#if np.artworkUrl && np.artworkUrl !== failedArtwork}
    <img class="art" src={np.artworkUrl} alt="" onerror={(event) => failedArtwork = event.currentTarget.getAttribute('src') ?? ''} />
  {:else}
    <span class="art placeholder"><Music size={12} /></span>
  {/if}
  {#if progress !== null}
    <span class="progress" role="progressbar" aria-label={t('island.progress')} aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(progress * 100)}>
      <span class="fill" style:transform={`scaleX(${progress})`}></span>
    </span>
  {:else}<span class="spacer"></span>{/if}
  <span class="bars" class:paused={!playing} class:real={real} aria-hidden="true">
    {#each [0, 1, 2, 3] as i (i)}
      <i style:transform={real ? `scaleY(${Math.max(0.15, real[i] ?? 0)})` : undefined}></i>
    {/each}
  </span>
</div>

<style>
  .compact {
    display: flex;
    align-items: center;
    width: 100%;
    height: 100%;
    padding: 0 var(--sp-5);
    gap: var(--sp-5);
  }
  .art {
    width: 20px;
    height: 20px;
    border-radius: 6px;
    object-fit: cover;
    flex: none;
  }
  .placeholder {
    display: grid;
    place-items: center;
    background: var(--c-surface-3);
    color: var(--c-text-2);
  }
  .spacer {
    flex: 1;
  }
  .progress { flex: 1; height: var(--media-progress-h); overflow: hidden; border-radius: var(--r-pill); background: var(--c-surface-3); }
  .fill { display: block; width: 100%; height: 100%; background: var(--c-accent); transform-origin: left center; }
  .bars {
    display: flex;
    gap: 2px;
    align-items: center;
    height: 14px;
  }
  /* Simulated waves until the real visualizer (F15, P3). Runs only while playing. */
  .bars i {
    width: 3px;
    height: 14px;
    border-radius: 2px;
    background: var(--c-accent);
    transform-origin: 50% 50%;
    animation: wave 900ms ease-in-out infinite;
  }
  .bars i:nth-child(2) {
    animation-delay: -300ms;
  }
  .bars i:nth-child(3) {
    animation-delay: -600ms;
  }
  .bars i:nth-child(4) {
    animation-delay: -150ms;
  }
  .bars.real i {
    animation: none;
    transition: transform 60ms linear;
  }
  .bars.paused i {
    animation: none;
    transform: scaleY(0.25);
    background: var(--c-text-3);
  }
  @keyframes wave {
    0%,
    100% {
      transform: scaleY(0.3);
    }
    50% {
      transform: scaleY(1);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .bars i {
      animation: none;
      transform: scaleY(0.6);
    }
  }
</style>
