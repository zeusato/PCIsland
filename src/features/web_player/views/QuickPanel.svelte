<script lang="ts">
  import SiteChooser from './SiteChooser.svelte';
  import Queue from '../../media/views/Queue.svelte';
  import { t } from '$lib/i18n/index.svelte';
  import { pickAudioFiles } from '$lib/ipc/dialog';
  import { mediaCommands, playerCommands } from '$lib/ipc/commands';
  import { islandInteraction } from '$lib/components/islandInteraction';
  const { showPlayer } = islandInteraction();
  let { close }: { close: () => void } = $props();
  let local = $state(false);
  let error = $state(false);
  function resumePlayer() {
    void mediaCommands.selectSource('web').catch(() => {});
    showPlayer('web');
  }
  async function files() {
    try { const paths = await pickAudioFiles(); if (paths.length) await playerCommands.playFiles(paths); }
    catch { error = true; }
  }
</script>
<div class="tabs"><button class:active={!local} onclick={() => local = false}>{t('main.nav.online')}</button><button class:active={local} onclick={() => local = true}>{t('music.library')}</button></div>
{#if local}
  <button onclick={files}>{t('music.openFiles')}</button>
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
