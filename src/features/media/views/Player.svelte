<script lang="ts">
  import type { NowPlayingDto } from '$lib/ipc/bindings/NowPlayingDto';
  import { nowPlaying } from '../store.svelte';
  import { t } from '$lib/i18n/index.svelte';
  import Peek from './Peek.svelte';
  import Queue from './Queue.svelte';
  let { np, source }: { np?: NowPlayingDto; source?: string } = $props();
  let sourceReady = $state(false);
  $effect(() => { if (nowPlaying()?.sourceId === source) sourceReady = true; });
  const current = $derived(np ?? (sourceReady || nowPlaying()?.sourceId === source ? nowPlaying() : null));
  let showQueue = $state(false);
</script>

{#if showQueue && current}
  <Queue expand source={current.sourceId} close={() => showQueue = false} />
{:else if current}
  <div class="player"><Peek np={current} /><button class="queue-toggle" onclick={(event) => { event.stopPropagation(); showQueue = true; }}>{t('music.queue')}</button></div>
{:else}
  <div class="loading" role="status">{t('common.loading')}</div>
{/if}

<style>
  .player { position: relative; height: 100%; }
  .loading { display: grid; place-items: center; color: var(--c-text-2); font-size: var(--fs-small); }
  .queue-toggle { position: absolute; right: var(--sp-5); bottom: var(--sp-3); background: var(--c-surface-2); border: 0; border-radius: var(--r-control); color: var(--c-text-2); padding: var(--sp-2) var(--sp-3); font-size: var(--fs-caption); cursor: pointer; }
</style>
