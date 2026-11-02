<script lang="ts">
  import Button from '$lib/components/Button.svelte';
  import { t } from '$lib/i18n/index.svelte';
  import { bytes, type createMaintenance } from '../controller.svelte';
  import ConfirmRow from './ConfirmRow.svelte';
  let { controller }: { controller: ReturnType<typeof createMaintenance> } = $props();
  const s = $derived(controller.state);
</script>

<div class="apps" role="list" aria-label={t('maintenance.apps')}>
  {#each s.apps as app (app.name)}
    <div class="app" role="listitem">
      <div class="row">
        <span class="info">
          <b>{app.name}</b>
          <small>{t('maintenance.appMemory', { count: app.processes.length, ws: bytes(app.workingSet) })}</small>
          {#if app.audio}<small class="badge">{t('maintenance.appAudio')}</small>{/if}
          {#if !app.windows}<small>{t('maintenance.appBackground')}</small>{/if}
        </span>
        <Button disabled={s.busy} onclick={() => controller.ask({ kind: 'close', name: app.name, audio: app.audio })}>{t('maintenance.closeApp')}</Button>
      </div>
      {#if (s.confirm?.kind === 'close' || s.confirm?.kind === 'force') && s.confirm.name === app.name}
        <ConfirmRow
          busy={s.busy}
          message={t(s.confirm.kind === 'force' ? 'maintenance.forceConfirm' : s.confirm.audio ? 'maintenance.closeConfirmAudio' : 'maintenance.closeConfirm', { name: app.name })}
          action={t(s.confirm.kind === 'force' ? 'maintenance.forceAction' : 'maintenance.closeConfirmAction')}
          onconfirm={controller.closeApp}
          oncancel={controller.cancel} />
      {/if}
    </div>
  {:else}<p>{t('maintenance.appsEmpty')}</p>{/each}
</div>
{#if s.closed}
  {@const c = s.closed}
  <div role="status" class="result">
    <p>{t(`maintenance.closeStatus.${c.result.status}`, { name: c.name, count: c.result.remaining.length })}</p>
    {#if (c.result.status === 'pending' || c.result.status === 'no_window') && c.result.remaining.length && s.confirm?.kind !== 'force'}
      <div><Button variant="danger" disabled={s.busy} onclick={() => controller.ask({ kind: 'force', name: c.name, audio: false })}>{t('maintenance.forceAction')}</Button></div>
    {/if}
    {#if s.confirm?.kind === 'force' && s.confirm.name === c.name && !s.apps.some(a => a.name === c.name)}
      <ConfirmRow busy={s.busy} message={t('maintenance.forceConfirm', { name: c.name })} action={t('maintenance.forceAction')} onconfirm={controller.closeApp} oncancel={controller.cancel} />
    {/if}
  </div>
{/if}

<style>
  p { margin: 0; color: var(--c-text-2); line-height: 1.5; }
  .apps { max-height: var(--island-list-max-h); overflow: auto; border: 1px solid var(--c-border); border-radius: var(--r-control); }
  .app { display: grid; gap: var(--sp-4); padding: var(--sp-4); border-bottom: 1px solid var(--c-border); }
  .row { display: flex; align-items: center; justify-content: space-between; gap: var(--sp-4); }
  .info { min-width: 0; overflow-wrap: anywhere; }
  small { display: block; color: var(--c-text-2); font-size: var(--fs-caption); line-height: 1.5; }
  .badge { color: var(--c-warning); }
  .result { display: grid; gap: var(--sp-4); }
</style>
