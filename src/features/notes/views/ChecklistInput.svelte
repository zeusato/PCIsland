<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '$lib/i18n/index.svelte';
  let { onadd, oncancel, busy = false }: { onadd: (label: string) => Promise<unknown>; oncancel: () => void; busy?: boolean } = $props();
  let label = $state('');
  let input: HTMLInputElement | undefined = $state();
  export function focus() { input?.focus(); }
  onMount(focus);
  async function add() {
    if (!busy && label.trim()) await onadd(label.trim());
  }
</script>

<form onsubmit={(event) => { event.preventDefault(); void add(); }}>
  <input bind:this={input} bind:value={label} placeholder={t('notes.checkbox')} aria-label={t('notes.checkbox')} disabled={busy} onkeydown={(event) => {
    if (event.key === 'Enter' && event.isComposing) event.preventDefault();
    if (event.key === 'Escape' && !busy) { event.stopPropagation(); oncancel(); }
  }} />
  <button disabled={busy || !label.trim()} type="submit">{t('common.add')}</button>
  <button disabled={busy} type="button" onclick={oncancel}>{t('common.cancel')}</button>
</form>

<style>
  form { display: flex; gap: var(--sp-3); }
  input, button { border: 1px solid var(--c-border-strong); border-radius: var(--r-control); background: var(--c-surface-2); color: var(--c-text-1); padding: var(--sp-3) var(--sp-4); font: inherit; }
  input { flex: 1; min-width: 0; user-select: text; }
  button { cursor: pointer; }
  button:disabled { opacity: 0.4; cursor: default; }
</style>
