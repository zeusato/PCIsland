<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import { t } from '$lib/i18n/index.svelte';
  import { bytes, createMaintenance } from '../controller.svelte';
  import ProcessList from './ProcessList.svelte';
  const controller = createMaintenance();
  const s = controller.state;
  onMount(controller.start);
</script>

<div class="maintenance">
  <section aria-label={t('maintenance.cleanup')}>
    <h3>{t('maintenance.cleanup')}</h3>
    <p>{t('maintenance.cleanupHint')}</p>
    <Button variant="primary" disabled={s.busy} onclick={controller.clean}>{t(s.busy && s.cleanup?.phase !== 'done' ? 'maintenance.busy' : 'maintenance.cleanNow')}</Button>
    {#if s.cleanup}
      {@const p = s.cleanup}
      <div role="status" aria-live="polite">
        {#if p.phase === 'failed'}
          <p>{t('maintenance.cleanupFailed')}</p>
        {:else if p.phase === 'done'}
          <strong>{t(p.skipped || p.busyApps || p.limited ? 'maintenance.cleanupPartial' : p.files ? 'maintenance.cleanupDone' : 'maintenance.cleanupEmpty')}</strong>
          <p>{t('maintenance.cleanupResult', { size: bytes(p.bytes), count: p.files, skipped: p.skipped })}</p>
          {#if p.busyApps}<p>{t('maintenance.busyApps', { count: p.busyApps })}</p>{/if}
          {#if p.limited}<p>{t('maintenance.cleanupLimited')}</p>{/if}
        {:else}
          <p>{t(p.phase === 'scan' ? 'maintenance.cleanupScanning' : 'maintenance.cleanupDeleting', { count: p.phase === 'scan' ? p.scanned : p.processed, total: p.total })} · {t(`maintenance.category.${p.category}`)}</p>
          <progress aria-label={t('maintenance.cleanup')} max={Math.max(1, p.total)} value={p.phase === 'scan' ? undefined : p.processed}></progress>
          <p>{t('maintenance.cleanupRunning', { size: bytes(p.bytes), count: p.files })}</p>
        {/if}
      </div>
    {/if}
  </section>
  <section aria-label={t('maintenance.ram')}>
    <h3>{t('maintenance.ram')}</h3>
    {#if s.resources}
      <strong>{t('maintenance.available', { free: bytes(s.resources.ramAvailable), total: bytes(s.resources.ramTotal) })}</strong>
      <p>{t('maintenance.cache', { size: bytes(s.resources.systemCache) })}</p>
    {/if}
    <p>{t('maintenance.ramHint')}</p>
    <div class="actions"><Button disabled={s.busy} onclick={controller.scan}>{t(s.busy ? 'maintenance.busy' : 'maintenance.scan')}</Button><Button disabled={s.busy} onclick={() => controller.open('task_manager')}>{t('maintenance.taskManager')}</Button></div>
    {#if s.scanned}
      <ProcessList rows={s.processes} selected={s.selected} busy={s.busy} onselect={controller.select} />
      <div class="actions"><Button disabled={s.busy} onclick={controller.selectCandidates}>{t('maintenance.selectCandidates')}</Button><Button variant="primary" disabled={s.busy || !s.selected.length} onclick={controller.reclaim}>{t('maintenance.reclaim', { count: s.selected.length })}</Button></div>
    {/if}
    {#if s.reclaim}
      <div role="status">
        <p>{t('maintenance.ramResult', { count: s.reclaim.processed, skipped: s.reclaim.skipped })}</p>
        <p>{t('maintenance.workingSet', { before: bytes(s.reclaim.workingSetBefore), after: bytes(s.reclaim.workingSetAfter) })}</p>
        <p>{t('maintenance.private', { before: bytes(s.reclaim.privateBefore), after: bytes(s.reclaim.privateAfter) })}</p>
        {#if s.ramBefore !== null && s.ramAfter !== null}<p>{t('maintenance.availableChange', { before: bytes(s.ramBefore), after: bytes(s.ramAfter) })}</p>{/if}
        {#if s.waiting}<p>{t('maintenance.wait30')}</p>{/if}
        {#if s.later}<p>{t('maintenance.after30', { count: s.later.count, ws: bytes(s.later.workingSet), private: bytes(s.later.privateBytes) })}</p>{/if}
      </div>
    {/if}
  </section>
  <section aria-label={t('maintenance.disk')}>
    <h3>{t('maintenance.disk')}</h3>
    {#if s.resources}<strong>{t('maintenance.available', { free: bytes(s.resources.diskFree), total: bytes(s.resources.diskTotal) })}</strong>{/if}
    <p>{t('maintenance.storageHint')}</p>
    <div class="actions"><Button disabled={s.busy} onclick={() => controller.open('storage')}>{t('maintenance.storage')}</Button><Button disabled={s.busy} onclick={controller.refresh}>{t('maintenance.refresh')}</Button></div>
    {#if s.diskBefore !== null && s.resources}<p>{t('maintenance.diskChange', { before: bytes(s.diskBefore), after: bytes(s.resources.diskFree) })}</p>{/if}
  </section>
  <section aria-label={t('maintenance.logs')}>
    <h3>{t('maintenance.logs')}</h3><p>{t('maintenance.logsHint')}</p>
    <div class="actions"><Button disabled={s.busy} onclick={controller.scanLogs}>{t('maintenance.scanLogs')}</Button><Button disabled={s.busy || !s.logs?.files} onclick={controller.cleanLogs}>{t('maintenance.cleanLogs')}</Button></div>
    {#if s.logs}<p>{t('maintenance.logEstimate', { size: bytes(s.logs.bytes), count: s.logs.files, skipped: s.logs.skipped })}</p>{/if}
    {#if s.cleaned}<p role="status">{t('maintenance.logResult', { size: bytes(s.cleaned.bytes), count: s.cleaned.files, skipped: s.cleaned.skipped })}</p>{/if}
  </section>
  {#if s.error || s.samplingError}<p role="alert">{t('maintenance.error')}</p>{/if}
</div>

<style>
  .maintenance { display: grid; gap: var(--sp-6); font-size: var(--fs-small); }
  section { display: grid; gap: var(--sp-4); padding: var(--sp-5); background: var(--c-surface-2); border-radius: var(--r-card); min-width: 0; }
  h3, p { margin: 0; } h3 { font-size: var(--fs-body); } p { color: var(--c-text-2); line-height: 1.5; }
  strong { color: var(--c-accent); font-variant-numeric: tabular-nums; }
  .actions { display: flex; flex-wrap: wrap; gap: var(--sp-4); }
  progress { width: 100%; accent-color: var(--c-accent); }
  [role='alert'] { color: var(--c-danger); }
</style>
