<script lang="ts">
  import type { NowPlayingDto } from '$lib/ipc/bindings/NowPlayingDto';
  import Music from '@lucide/svelte/icons/music';
  import { bars, fresh, viz } from '$lib/stores/visualizer.svelte';

  let { np }: { np: NowPlayingDto } = $props();
  const playing = $derived(np.status === 'playing');
  // Real levels while the analyzer runs (F15), CSS waves otherwise.
  const real = $derived(playing && fresh(viz.at + 1) ? bars(4) : null);
</script>

<div class="compact">
  {#if np.artworkUrl}
    <img class="art" src={np.artworkUrl} alt="" />
  {:else}
    <span class="art placeholder"><Music size={12} /></span>
  {/if}
  <span class="spacer"></span>
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
