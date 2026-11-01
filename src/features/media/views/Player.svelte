<script lang="ts">
  import { holdFocus } from '$lib/ipc/focus';
  import type { NowPlayingDto } from '$lib/ipc/bindings/NowPlayingDto';
  import { nowPlaying } from '../store.svelte';
  import { t } from '$lib/i18n/index.svelte';
  import Peek from './Peek.svelte';
  import Queue from './Queue.svelte';
  import RepeatControl from './RepeatControl.svelte';
  import VideoControls from '../../video/views/VideoControls.svelte';
  import Search from '../../web_player/views/Search.svelte';
  import { islandInteraction } from '$lib/components/islandInteraction';
  import { inTauri, mediaCommands } from '$lib/ipc/commands';
  const { fit, showPlayer } = islandInteraction();
  let { np, source, active = true }: { np?: NowPlayingDto; source?: string; active?: boolean } = $props();
  let sourceReady = $state(false);
  let requested: string | undefined;
  $effect(() => {
    if (requested !== source) { requested = source; sourceReady = false; }
    if (nowPlaying()?.sourceId === source) sourceReady = true;
  });
  const current = $derived(np ?? (!source || sourceReady || nowPlaying()?.sourceId === source ? nowPlaying() : null));
  let showQueue = $state(false);
  let searching = $state(false);
  let searchElement: HTMLDivElement | undefined = $state();
  $effect(() => {
    if (!active || !searching || !inTauri) return;
    let mounted = true;
    const focus = holdFocus();
    void focus.ready.then(() => { if (mounted) searchElement?.querySelector('input')?.focus(); });
    return () => { mounted = false; focus.release(); };
  });
  $effect(() => {
    if (!active || !searching || !searchElement) return;
    return fit(searchElement).destroy;
  });
  function played() {
    searching = false;
    void mediaCommands.selectSource('web');
    showPlayer('web');
  }
</script>

{#snippet videoControls()}<VideoControls />{/snippet}

{#if searching}
  <div class="search-panel" bind:this={searchElement} role="dialog" aria-label={t('webPlayer.search')} tabindex="-1" onclick={(event) => event.stopPropagation()} onkeydown={(event) => { event.stopPropagation(); if (event.key === 'Escape') searching = false; }}>
    <button onclick={() => searching = false}>{t('music.backToPlayer')}</button>
    <Search onplay={played} />
  </div>
{:else if showQueue && current}
  <Queue expand={active} source={current.sourceId} close={() => showQueue = false} />
{:else if current}
  <div class="player"><Peek np={current} {active} trailing={current.sourceId === 'video' ? videoControls : undefined} /><div class="repeat"><RepeatControl source={current.sourceId} /></div>{#if current.sourceId !== 'video'}<div class="player-actions"><button onclick={(event) => { event.stopPropagation(); searching = true; }}>{t('webPlayer.searchButton')}</button><button onclick={(event) => { event.stopPropagation(); showQueue = true; }}>{t('music.queue')}</button></div>{/if}</div>
{:else}
  <div class="loading" role="status">{t('common.loading')}</div>
{/if}

<style>
  .player { position: relative; height: 100%; width: 100%; min-width: 0; }
  .repeat { position: absolute; left: var(--sp-5); bottom: var(--sp-3); }
  .loading { display: grid; place-items: center; color: var(--c-text-2); font-size: var(--fs-small); }
  .player-actions { position: absolute; right: var(--sp-5); bottom: var(--sp-3); display: flex; gap: var(--sp-2); }
  button { background: var(--c-surface-2); border: 0; border-radius: var(--r-control); color: var(--c-text-2); padding: var(--sp-2) var(--sp-3); font-size: var(--fs-caption); cursor: pointer; }
  .search-panel { align-self: start; padding: var(--sp-5); display: flex; flex-direction: column; gap: var(--sp-3); max-height: var(--island-panel-max-h); overflow: auto; }
</style>
