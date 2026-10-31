<script lang="ts">
  import { checklistLine } from '../checklist';
  import { t } from '$lib/i18n/index.svelte';

  let { text, limit, ontoggle, onedit, disabled = false }: {
    text: string;
    limit?: number;
    ontoggle: (line: number) => Promise<unknown>;
    onedit?: () => void;
    disabled?: boolean;
  } = $props();
  let busy = $state(false);
  const lines = $derived(text.split('\n').slice(0, limit));
  async function toggle(line: number) {
    if (busy) return;
    busy = true;
    try { await ontoggle(line); } finally { busy = false; }
  }
</script>

<div class="body">
  {#each lines as line, i (i)}
    {@const task = checklistLine(line)}
    {#if task}
      <button class="task" class:done={task.checked} role="checkbox" aria-checked={task.checked} aria-label={task.text || t('notes.checkbox')} disabled={busy || disabled} onclick={(event) => { event.stopPropagation(); void toggle(i); }}>
        <span class="box" aria-hidden="true">{task.checked ? '✓' : ''}</span><span>{task.text}</span>
      </button>
    {:else if onedit}
      <button class="line" onclick={onedit}>{line || '\u00a0'}</button>
    {:else}
      <p class="line">{line || '\u00a0'}</p>
    {/if}
  {/each}
</div>

<style>
  .body { display: flex; flex-direction: column; gap: var(--sp-2); min-width: 0; }
  .line, .task { margin: 0; padding: 0; border: 0; background: none; text-align: left; font: inherit; color: inherit; white-space: pre-wrap; overflow-wrap: anywhere; }
  button { cursor: pointer; }
  .task { display: flex; align-items: flex-start; gap: var(--sp-4); }
  .box { flex: none; display: grid; place-items: center; width: var(--sp-6); height: var(--sp-6); border: 1px solid var(--c-border-strong); border-radius: var(--sp-2); color: var(--c-accent); font-size: var(--fs-caption); }
  .done { color: var(--c-text-3); text-decoration: line-through; }
</style>
