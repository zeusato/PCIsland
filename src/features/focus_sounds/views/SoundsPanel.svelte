<script lang="ts">
  // Settings › Pomodoro › focus music (docs/modules/focus-sounds.md §3).
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import Toggle from '$lib/components/form/Toggle.svelte';
  import { t, getLocale } from '$lib/i18n/index.svelte';
  import { live } from '$lib/stores/live.svelte';
  import type { PackName } from '$lib/ipc/bindings/PackName';
  import { cancelDownload, downloadPacks, focusSounds, loadCatalog, loadStatus, mib, removePack, usePack } from '../store.svelte';

  // Compact: island Pomodoro panel, no hint/delete/license (managed from main).
  let { compact = false }: { compact?: boolean } = $props();
  onMount(() => { void loadStatus(); });
  const name = (n: PackName) => (getLocale() === 'en' ? n.en : n.vi);
  const selected = $derived(live.settings?.pomodoro.focusSoundPacks ?? []);
  const installed = $derived(focusSounds.status?.packs ?? []);
  const p = $derived(focusSounds.progress);
</script>

<div class="sounds" class:compact>
  {#if !compact}<p class="muted">{t('focusSounds.hint')}</p>{/if}
  {#each installed as pack (pack.id)}
    <div class="row">
      <span class="name">{name(pack.name)}<small>{t(pack.role === 'focus' ? 'focusSounds.roleFocus' : 'focusSounds.roleBreak')} · {t('focusSounds.tracks', { count: pack.tracks })} · {mib(pack.bytes)}{pack.complete ? '' : ` · ${t('focusSounds.incomplete')}`}</small></span>
      <Toggle label={t('focusSounds.use')} checked={selected.includes(pack.id)} onchange={(on) => usePack(pack.id, on)} />
      {#if !compact}<Button variant="ghost" disabled={focusSounds.busy} onclick={() => removePack(pack.id)}>{t('common.delete')}</Button>{/if}
    </div>
  {/each}

  {#if focusSounds.catalog}
    {#each focusSounds.catalog.packs.filter((c) => c.state !== 'installed') as pack (pack.id)}
      <div class="row">
        <span class="name">{name(pack.name)}<small>{t(pack.role === 'focus' ? 'focusSounds.roleFocus' : 'focusSounds.roleBreak')} · {t('focusSounds.tracks', { count: pack.tracks })} · {mib(pack.bytes)}</small></span>
        <Button variant="primary" disabled={focusSounds.busy} onclick={() => downloadPacks([pack.id])}>{t(pack.state === 'updateAvailable' ? 'focusSounds.update' : 'focusSounds.download')}</Button>
      </div>
    {:else}
      <p class="muted">{t('focusSounds.allInstalled')}</p>
    {/each}
    {#if focusSounds.catalog.license && !compact}<p class="muted">{focusSounds.catalog.license}</p>{/if}
  {:else}
    <Button disabled={focusSounds.loadingCatalog} onclick={loadCatalog}>{t(focusSounds.loadingCatalog ? 'common.loading' : 'focusSounds.browse')}</Button>
  {/if}
  {#if focusSounds.catalogError}<p role="alert">{t('focusSounds.catalogError')}</p>{/if}

  {#if focusSounds.busy}
    <div class="progress" role="status">
      <progress max={p?.totalBytes || 1} value={p?.doneBytes ?? 0}></progress>
      <span>{p ? `${p.file} · ${p.doneFiles}/${p.totalFiles} · ${mib(p.doneBytes)} / ${mib(p.totalBytes)}` : t('common.loading')}</span>
      <Button variant="ghost" onclick={cancelDownload}>{t('common.cancel')}</Button>
    </div>
  {/if}
  {#if focusSounds.error}<p role="alert">{t('focusSounds.downloadError', { error: focusSounds.error })}</p>{/if}
</div>

<style>
  .sounds { display: flex; flex-direction: column; gap: var(--sp-4); }
  .compact { gap: var(--sp-3); font-size: var(--fs-small); }
  .compact .progress { flex-wrap: wrap; }
  .row { display: flex; align-items: center; gap: var(--sp-4); }
  .name { flex: 1; min-width: 0; }
  small { display: block; color: var(--c-text-3); font-size: var(--fs-caption); }
  .progress { display: flex; align-items: center; gap: var(--sp-4); font-size: var(--fs-caption); color: var(--c-text-2); }
  progress { flex: 1; accent-color: var(--c-accent); }
  .muted { margin: 0; color: var(--c-text-2); font-size: var(--fs-small); }
  [role='alert'] { margin: 0; color: var(--c-danger); }
</style>
