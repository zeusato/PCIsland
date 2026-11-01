<script lang="ts">
  import type { MemoryProcess } from '$lib/ipc/bindings/MemoryProcess';
  import { t } from '$lib/i18n/index.svelte';
  import { bytes } from '../controller.svelte';
  let { rows, selected, busy, onselect }: { rows: MemoryProcess[]; selected: number[]; busy: boolean; onselect: (pid: number, checked: boolean) => void } = $props();
</script>
<p>{t('maintenance.processHint')}</p>
<div class="processes" role="group" aria-label={t('maintenance.processes')}>
  {#each rows as row (row.target.pid + row.target.started)}
    <label>
      <input type="checkbox" disabled={busy || row.reason !== 'candidate'} checked={selected.includes(row.target.pid)} onchange={(e) => onselect(row.target.pid, e.currentTarget.checked)} />
      <span><b>{row.name}</b> · {row.target.pid}<small>{t('maintenance.processMemory', { ws: bytes(row.workingSet), private: bytes(row.privateBytes) })}</small><small>{t(`maintenance.reason.${row.reason}`)}</small></span>
    </label>
  {:else}<p>{t('maintenance.empty')}</p>{/each}
</div>
<style>
  p { margin: 0; color: var(--c-text-2); }
  .processes { max-height: var(--island-list-max-h); overflow: auto; border: 1px solid var(--c-border); border-radius: var(--r-control); }
  label { display: flex; align-items: start; gap: var(--sp-4); padding: var(--sp-4); border-bottom: 1px solid var(--c-border); }
  input { accent-color: var(--c-accent); flex-shrink: 0; }
  span { min-width: 0; overflow-wrap: anywhere; } small { display: block; color: var(--c-text-2); font-size: var(--fs-caption); line-height: 1.5; }
</style>
