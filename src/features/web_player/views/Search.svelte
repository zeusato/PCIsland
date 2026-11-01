<script lang="ts">
  import { webPlayerCommands } from '$lib/ipc/commands';
  import { t } from '$lib/i18n/index.svelte';
  import type { WebQueueItem } from '$lib/ipc/bindings/WebQueueItem';
  let { onplay }: { onplay?: () => void } = $props();
  let site = $state('zing_mp3');
  let query = $state('');
  let busy = $state(false);
  let error = $state(false);
  let results = $state<WebQueueItem[]>([]);
  async function search() {
    if (busy || !query.trim()) return;
    busy = true; error = false; results = [];
    try { results = await webPlayerCommands.search(site, query.trim()); }
    catch { error = true; }
    finally { busy = false; }
  }
  async function play(item: WebQueueItem) {
    busy = true; error = false;
    try { await webPlayerCommands.playResult(site, item.url); onplay?.(); }
    catch { error = true; }
    finally { busy = false; }
  }
</script>
<form onsubmit={(event) => { event.preventDefault(); void search(); }}>
  <select aria-label={t('webPlayer.searchSource')} bind:value={site} disabled={busy} onchange={() => { results = []; error = false; }}>
    <option value="zing_mp3">Zing MP3</option><option value="spotify">Spotify</option>
  </select>
  <div class="input-row"><input aria-label={t('webPlayer.search')} placeholder={t('webPlayer.search')} bind:value={query} maxlength="160" disabled={busy} /><button disabled={busy || !query.trim()}>{busy ? t('common.loading') : t('webPlayer.searchButton')}</button></div>
</form>
{#if error}<p role="alert">{t('webPlayer.searchError')}</p><button onclick={() => webPlayerCommands.open(site)}>{t('webPlayer.openHome')}</button>{/if}
<div class="results">{#each results as item (item.url)}<button class="result" disabled={busy} onclick={() => play(item)} title={item.title}>{item.title}</button>{/each}</div>
<style>
  form { display: flex; flex-direction: column; gap: var(--sp-2); margin-bottom: var(--sp-3); }
  .input-row { display: flex; gap: var(--sp-2); }
  input, select, button { font: inherit; font-size: var(--fs-small); color: var(--c-text-1); background: var(--c-surface-2); border: 1px solid var(--c-border-strong); border-radius: var(--r-control); padding: var(--sp-3); }
  input { min-width: 0; flex: 1; }
  button { cursor: pointer; }
  .results { display: flex; flex-direction: column; gap: var(--sp-2); max-height: var(--island-window-h); overflow-y: auto; }
  .result { text-align: left; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .result:hover { color: var(--c-accent-bright); border-color: var(--c-accent); }
  p { font-size: var(--fs-small); color: var(--c-text-2); }
</style>
