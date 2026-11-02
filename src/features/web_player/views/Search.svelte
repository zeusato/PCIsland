<script lang="ts">
  import { webPlayerCommands } from '$lib/ipc/commands';
  import { t } from '$lib/i18n/index.svelte';
  import type { WebQueueItem } from '$lib/ipc/bindings/WebQueueItem';
  let { onplay }: { onplay?: () => void } = $props();
  let site = $state('zing_mp3');
  let query = $state('');
  let busy = $state(false);
  let error = $state(false);
  let playError = $state(false);
  let playingTitle = $state('');
  let results = $state<WebQueueItem[]>([]);
  async function search() {
    if (busy || !query.trim()) return;
    busy = true; error = false; playError = false; results = [];
    try { results = await webPlayerCommands.search(site, query.trim()); }
    catch { error = true; }
    finally { busy = false; }
  }
  async function play(item: WebQueueItem) {
    if (busy) return;
    busy = true; error = false; playError = false; playingTitle = item.title;
    try { await webPlayerCommands.playResult(site, item.url); onplay?.(); }
    catch { playError = true; }
    finally { busy = false; playingTitle = ''; }
  }
</script>
<form onsubmit={(event) => { event.preventDefault(); void search(); }}>
  <select aria-label={t('webPlayer.searchSource')} bind:value={site} disabled={busy} onchange={() => { results = []; error = false; }}>
    <option value="zing_mp3">Zing MP3</option><option value="spotify">Spotify</option><option value="soundcloud">SoundCloud</option>
  </select>
  <div class="input-row"><input aria-label={t('webPlayer.search')} placeholder={t('webPlayer.search')} bind:value={query} maxlength="160" disabled={busy} /><button disabled={busy || !query.trim()}>{busy ? t('common.loading') : t('webPlayer.searchButton')}</button></div>
</form>
{#if error}<p role="alert">{t('webPlayer.searchError')}</p><button onclick={() => webPlayerCommands.open(site)}>{t('webPlayer.openHome')}</button>{/if}
{#if playingTitle}<p role="status">{t('webPlayer.startingTrack', { title: playingTitle })}</p>{/if}
{#if playError}<p role="alert">{t('webPlayer.playError')}</p><button onclick={() => webPlayerCommands.show()}>{t('webPlayer.openCurrent')}</button>{/if}
<div class="results">{#each results as item (item.url)}
  <button class="result" disabled={busy} onclick={() => play(item)} title={item.title}>
    <span class="result-title">{item.title}</span>
    <span class="credit">{t('webPlayer.artist')}: {item.artist || t('webPlayer.creditUnavailable')}</span>
    {#if item.uploader}<span class="credit">{t('webPlayer.uploader')}: {item.uploader}</span>{/if}
  </button>
{/each}</div>
<style>
  form { display: flex; flex-shrink: 0; flex-direction: column; gap: var(--sp-2); margin-bottom: var(--sp-3); }
  .input-row { display: flex; gap: var(--sp-2); }
  input, select, button { font: inherit; font-size: var(--fs-small); color: var(--c-text-1); background: var(--c-surface-2); border: 1px solid var(--c-border-strong); border-radius: var(--r-control); padding: var(--sp-3); }
  input { min-width: 0; flex: 1; }
  button { cursor: pointer; }
  .results { display: flex; flex-direction: column; gap: var(--sp-2); max-height: var(--island-list-max-h); overflow-y: auto; min-height: 0; scrollbar-gutter: stable; }
  .result { flex: 0 0 auto; min-height: var(--sp-10); text-align: left; white-space: normal; overflow-wrap: anywhere; line-height: 1.5; }
  .result:hover { color: var(--c-accent-bright); border-color: var(--c-accent); }
  .result-title { display: block; font-weight: var(--fw-medium); }
  .credit { display: block; color: var(--c-text-2); font-size: var(--fs-caption); margin-top: var(--sp-1); }
  p { font-size: var(--fs-small); color: var(--c-text-2); }
</style>
