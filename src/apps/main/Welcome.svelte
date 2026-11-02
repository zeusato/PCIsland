<script lang="ts">
  // First-run welcome: optional downloads in one place (docs/modules/focus-sounds.md §3,
  // video-player §5.3). Nothing downloads until the user confirms.
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import { t, getLocale } from '$lib/i18n/index.svelte';
  import { inTauri } from '$lib/ipc/commands';
  import { videoCommands } from '$lib/ipc/video';
  import { updateSettings } from '$lib/stores/live.svelte';
  import { downloadPacks, focusSounds, loadCatalog, mib } from '../../features/focus_sounds/store.svelte';

  let toolsInstalled = $state(false);
  let wantTools = $state(true);
  let packs = $state<string[]>([]);
  let phase = $state<'choose' | 'working' | 'done'>('choose');
  let toolsError = $state(false);
  let soundsError = $state(false);

  onMount(() => {
    if (!inTauri) return;
    void videoCommands.toolsStatus().then((s) => { toolsInstalled = s.installed; }).catch(() => {});
    void loadCatalog().then(() => { packs = focusSounds.catalog?.packs.filter((p) => p.state !== 'installed').map((p) => p.id) ?? []; });
  });
  const offered = $derived(focusSounds.catalog?.packs.filter((p) => p.state !== 'installed') ?? []);
  const soundBytes = $derived(offered.filter((p) => packs.includes(p.id)).reduce((n, p) => n + p.bytes, 0));
  const nothing = $derived((toolsInstalled || !wantTools) && !packs.length);

  const finish = () => updateSettings('app', { welcomeDone: true });
  async function start() {
    if (nothing) return finish();
    phase = 'working';
    if (wantTools && !toolsInstalled) {
      try { await videoCommands.installTools(); toolsInstalled = true; } catch { toolsError = true; }
    }
    if (packs.length) soundsError = !(await downloadPacks(packs));
    phase = 'done';
  }
</script>

<div class="backdrop">
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="welcome-title">
    <h2 id="welcome-title">{t('welcome.title')}</h2>
    <p>{t('welcome.intro')}</p>

    {#if phase === 'choose'}
      <label class="option">
        <input type="checkbox" bind:checked={wantTools} disabled={toolsInstalled} />
        <span>{t('welcome.tools')}<small>{toolsInstalled ? t('welcome.alreadyInstalled') : t('welcome.toolsHint')}</small></span>
      </label>
      <h3>{t('focusSounds.title')}</h3>
      {#if focusSounds.loadingCatalog}<p class="muted">{t('common.loading')}</p>
      {:else if focusSounds.catalogError}<p class="muted">{t('welcome.soundsUnavailable')}</p>
      {:else}
        {#each offered as pack (pack.id)}
          <label class="option">
            <input type="checkbox" checked={packs.includes(pack.id)} onchange={(e) => { packs = (e.currentTarget as HTMLInputElement).checked ? [...packs, pack.id] : packs.filter((x) => x !== pack.id); }} />
            <span>{getLocale() === 'en' ? pack.name.en : pack.name.vi}<small>{t(pack.role === 'focus' ? 'focusSounds.roleFocus' : 'focusSounds.roleBreak')} · {t('focusSounds.tracks', { count: pack.tracks })} · {mib(pack.bytes)}</small></span>
          </label>
        {:else}<p class="muted">{t('focusSounds.allInstalled')}</p>{/each}
      {/if}
      <p class="muted">{t('welcome.later')}</p>
      <div class="actions">
        <Button variant="ghost" onclick={finish}>{t('welcome.skip')}</Button>
        <Button variant="primary" onclick={start}>{nothing ? t('welcome.continue') : t('welcome.download', { size: mib(soundBytes) })}</Button>
      </div>
    {:else if phase === 'working'}
      <p role="status">{focusSounds.progress ? `${focusSounds.progress.file} · ${focusSounds.progress.doneFiles}/${focusSounds.progress.totalFiles}` : t('welcome.working')}</p>
      <progress max={focusSounds.progress?.totalBytes || 1} value={focusSounds.progress?.doneBytes ?? 0}></progress>
    {:else}
      <p role="status">{toolsError || soundsError ? t('welcome.partial') : t('welcome.done')}</p>
      {#if focusSounds.error}<p role="alert">{focusSounds.error}</p>{/if}
      <div class="actions"><Button variant="primary" onclick={finish}>{t('welcome.close')}</Button></div>
    {/if}
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; display: grid; place-items: center; background: var(--c-scrim); z-index: 50; }
  .dialog { width: min(520px, calc(100vw - 2 * var(--sp-6))); max-height: calc(100vh - 2 * var(--sp-6)); overflow: auto; display: flex; flex-direction: column; gap: var(--sp-4); padding: var(--sp-7); border: 1px solid var(--c-border-strong); border-radius: var(--r-card); background: var(--c-surface-1); }
  h2, h3, p { margin: 0; }
  h3 { font-size: var(--fs-body); }
  .option { display: flex; gap: var(--sp-4); align-items: flex-start; cursor: pointer; }
  .option input { accent-color: var(--c-accent); margin-top: var(--sp-1); }
  small { display: block; color: var(--c-text-3); font-size: var(--fs-caption); }
  .muted { color: var(--c-text-2); font-size: var(--fs-small); }
  .actions { display: flex; justify-content: flex-end; gap: var(--sp-4); }
  progress { width: 100%; accent-color: var(--c-accent); }
  [role='alert'] { color: var(--c-danger); }
</style>
