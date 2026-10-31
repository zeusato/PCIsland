<script lang="ts">
  import { feedback } from '$lib/stores/feedback.svelte';
  import { t } from '$lib/i18n/index.svelte';
  let { floating = false }: { floating?: boolean } = $props();
  $effect(() => {
    void feedback.revision;
    if (!feedback.message) return;
    const timer = setTimeout(() => feedback.message = '', 10000);
    return () => clearTimeout(timer);
  });
</script>

{#if feedback.message}
  <aside class="toast" class:floating aria-live="polite" aria-atomic="true">
    <p role="status">{feedback.message}</p>
    <button aria-label={t('common.close')} onclick={(event) => { event.stopPropagation(); feedback.message = ''; }} ondblclick={(event) => event.stopPropagation()}>×</button>
  </aside>
{/if}

<style>
  .toast { display: flex; align-items: flex-start; gap: var(--sp-4); padding: var(--sp-5); border: 1px solid var(--c-accent); border-radius: var(--r-control); background: var(--c-surface-2); color: var(--c-text-1); font-size: var(--fs-small); box-shadow: var(--shadow-peek); }
  .floating { position: absolute; inset: var(--island-switcher-h) var(--sp-5) var(--sp-5); z-index: 2; overflow-y: auto; }
  p { margin: 0; flex: 1; white-space: pre-line; }
  button { flex: none; border: 0; border-radius: var(--r-control); background: var(--c-surface-3); color: var(--c-text-1); cursor: pointer; font: inherit; padding: var(--sp-2) var(--sp-4); }
</style>
