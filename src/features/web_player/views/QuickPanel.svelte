<script lang="ts">
  import SiteChooser from './SiteChooser.svelte';
  import Queue from '../../media/views/Queue.svelte';
  import { t } from '$lib/i18n/index.svelte';
  import { pickFolder } from '$lib/ipc/dialog';
  import { mediaCommands } from '$lib/ipc/commands';
  import { playFolder } from '../../local_player/folder';
  import { islandInteraction } from '$lib/components/islandInteraction';
  const { showPlayer } = islandInteraction();
  let { close }: { close: () => void } = $props();
  let local = $state(false);
  let error = $state(false);
  let busy = $state(false);
  let empty = $state(false);
  function resumePlayer() {
    void mediaCommands.selectSource('web').catch(() => {});
    showPlayer('web');
  }
  async function folder() {
    if (busy) return;
    busy = true; error = false; empty = false;
    try {
      const path = await pickFolder();
      if (!path) return;
      if (!await playFolder(path)) { empty = true; return; }
      await mediaCommands.selectSource('local');
      showPlayer('local');
    }
    catch { error = true; }
    finally { busy = false; }
  }
</script>
<div class="tabs"><button class:active={!local} onclick={() => local = false}>{t('main.nav.online')}</button><button class:active={local} onclick={() => local = true}>{t('music.library')}</button></div>
{#if local}
  <button onclick={folder} disabled={busy}>{t(busy ? 'music.scanning' : 'music.chooseFolder')}</button>
  {#if empty}<p role="status">{t('music.folderEmpty')}</p>{/if}
  <div class="queue"><Queue source="local" {close} /></div>
{:else}<SiteChooser island onchoose={close} onplay={resumePlayer} />{/if}
{#if error}<p role="alert">{t('errors.unknown')}</p>{/if}
<style>
  .tabs { display: flex; gap: var(--sp-3); }
  button { border: 1px solid var(--c-border-strong); border-radius: var(--r-control); padding: var(--sp-2) var(--sp-4); background: var(--c-surface-2); color: var(--c-text-1); font-size: var(--fs-small); cursor: pointer; }
  .active { color: var(--c-accent-bright); border-color: var(--c-accent); }
  .queue { min-height: 0; flex: 1; }
  p { font-size: var(--fs-small); }
</style>
