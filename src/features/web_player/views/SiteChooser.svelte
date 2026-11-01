<script lang="ts">
  import { onMount } from 'svelte';
  import { inTauri, webPlayerCommands } from '$lib/ipc/commands';
  import { onWebPlayerState } from '$lib/ipc/events';
  import type { SiteDto } from '$lib/ipc/bindings/SiteDto';
  import type { WebPlayerSnapshot } from '$lib/ipc/bindings/WebPlayerSnapshot';
  import { t } from '$lib/i18n/index.svelte';
  import Search from './Search.svelte';
  import { media, nowPlaying } from '../../media/store.svelte';
  let searching = $state(false);
  let changing = $state(false);
  let { onchoose, onplay = onchoose, island = false }: { onchoose?: () => void; onplay?: () => void; island?: boolean } = $props();
  let sites = $state<SiteDto[]>([]);
  let snapshot = $state<WebPlayerSnapshot | null>(null);
  let selected = $state<string | null>(null);
  let busy = $state(false);
  let loading = $state(true);
  let error = $state(false);
  const saved = $derived(snapshot?.history.find((item) => item.siteId === selected));
  const currentSite = $derived(sites.find((site) => site.id === snapshot?.siteId));
  const currentTrack = $derived(nowPlaying()?.sourceId === 'web' ? nowPlaying() : null);
  const isPlaying = $derived(media.snapshot?.sources.some((source) => source.sourceId === 'web' && source.status === 'playing'));
  const currentTitle = $derived(currentTrack?.title || snapshot?.history.find((item) => item.siteId === snapshot?.siteId)?.title);
  const hasCurrent = $derived(island && currentSite && snapshot?.lifecycle !== 'closed');
  async function show() { try { await webPlayerCommands.show(); } catch { error = true; } }
  onMount(() => {
    if (!inTauri) { loading = false; return; }
    void Promise.all([webPlayerCommands.listSites(), webPlayerCommands.getState()])
      .then(([list, value]) => { sites = list; snapshot = value; })
      .catch(() => error = true).finally(() => loading = false);
    const sub = onWebPlayerState((value) => snapshot = value);
    return () => void sub.then((stop) => stop());
  });
  async function open(id: string, resume = false) {
    if (busy) return;
    busy = true; error = false;
    try {
      if (resume) { await webPlayerCommands.resume(id); onplay?.(); }
      else { await webPlayerCommands.open(id); onchoose?.(); }
    }
    catch { error = true; }
    finally { busy = false; }
  }
</script>

<div class="chooser">
  {#if hasCurrent && !changing && !searching}
    <div class="current">
      <strong>{currentSite?.name}</strong>
      <span class="status">{t(isPlaying ? 'webPlayer.playing' : 'webPlayer.currentSource')}</span>
      {#if currentTitle}<p class="saved" title={currentTitle}>{currentTitle}</p>{/if}
      <div class="sites">
        <button onclick={show}>{t('webPlayer.openCurrent')}</button>
        <button onclick={() => changing = true}>{t('webPlayer.changeSite')}</button>
        <button onclick={() => searching = true}>{t('webPlayer.search')}</button>
      </div>
    </div>
  {:else}
  {#if hasCurrent}<button class="search-toggle" onclick={() => { searching = false; changing = false; }}>{t('webPlayer.backToCurrent')}</button>{/if}
  <button class="search-toggle" class:active={searching} onclick={() => searching = !searching}>{searching ? t('webPlayer.sites') : t('webPlayer.search')}</button>
  {#if searching}<Search {onplay} />{:else}
  <div class="sites">{#each sites as site (site.id)}
    <button class:active={selected === site.id} disabled={busy || loading} onclick={(event) => { event.stopPropagation(); if (snapshot?.history.some((item) => item.siteId === site.id)) selected = site.id; else void open(site.id); }}>{site.name}</button>
  {/each}</div>
  {#if loading}<p>{t('common.loading')}</p>{/if}
  {#if selected && saved}
    <p class="saved">{saved.title}</p>
    <div class="sites">
      <button disabled={busy} onclick={(event) => { event.stopPropagation(); void open(selected!, true); }}>{t('webPlayer.resume')}</button>
      <button disabled={busy} onclick={(event) => { event.stopPropagation(); void open(selected!); }}>{t('webPlayer.openHome')}</button>
    </div>
  {/if}
  {/if}
  {/if}
  {#if error}<p role="alert">{t('errors.unknown')}</p>{/if}
</div>

<style>
  .chooser { flex-shrink: 0; min-height: 0; }
  .search-toggle { margin-bottom: var(--sp-3); }
  .current strong { color: var(--c-accent-bright); }
  .status { margin-left: var(--sp-3); color: var(--c-text-3); font-size: var(--fs-small); }
  .sites { display: flex; flex-wrap: wrap; gap: var(--sp-3); }
  button { border: 1px solid var(--c-border-strong); border-radius: var(--r-control); padding: var(--sp-3) var(--sp-4); background: var(--c-surface-2); color: var(--c-text-1); font-size: var(--fs-small); cursor: pointer; }
  .active { color: var(--c-accent-bright); border-color: var(--c-accent); }
  .saved { margin: var(--sp-3) 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--fs-small); }
</style>
