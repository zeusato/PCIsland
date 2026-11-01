<script lang="ts">
  import type { Activity } from '$lib/ipc/bindings/Activity';
  import type { NowPlayingDto } from '$lib/ipc/bindings/NowPlayingDto';
  import Music from '@lucide/svelte/icons/music';
  import { payload } from '$lib/registry/payloads';
  import Compact from './Compact.svelte';
  import Player from './Player.svelte';

  let { activity, variant }: { activity: Activity; variant: 'compact' | 'peek' | 'alert' | 'bubble' } = $props();
  const np: NowPlayingDto = $derived(payload(activity, 'media.now_playing'));
  let failedArtwork = $state('');
</script>

{#if variant === 'bubble'}
  <div class="bubble" class:playing={np.status === 'playing'}>
    {#if np.artworkUrl && np.artworkUrl !== failedArtwork}<img src={np.artworkUrl} alt="" onerror={(event) => failedArtwork = event.currentTarget.getAttribute('src') ?? ''} />{:else}<Music size={14} />{/if}
  </div>
{:else if variant === 'alert'}
  <div class="alert">
    {#if np.artworkUrl && np.artworkUrl !== failedArtwork}<img src={np.artworkUrl} alt="" onerror={(event) => failedArtwork = event.currentTarget.getAttribute('src') ?? ''} />{:else}<Music size={14} />{/if}
    <span class="title">{np.title}{#if np.artist} – {np.artist}{/if}</span>
  </div>
{:else if variant === 'compact'}
  <Compact {np} />
{:else}
  <Player {np} />
{/if}

<style>
  .bubble {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
  }
  .bubble img {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    object-fit: cover;
  }
  .bubble.playing img {
    box-shadow: 0 0 0 1.5px var(--c-accent);
  }
  .alert {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    width: 100%;
    padding: 0 var(--sp-6);
    font-size: var(--fs-body-island);
  }
  .alert img {
    width: 22px;
    height: 22px;
    border-radius: 6px;
    object-fit: cover;
  }
  .title {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
</style>
