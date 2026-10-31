<script lang="ts">
  // Quick reminder with Vietnamese natural-language preview (docs/modules/reminders.md §3, F25).
  import { islandCommands, reminderCommands } from '$lib/ipc/commands';
  import type { ParsedPreview } from '$lib/ipc/bindings/ParsedPreview';
  import { formatWhen } from '$lib/format';
  import { t } from '$lib/i18n/index.svelte';

  let { close }: { close: () => void } = $props();
  let text = $state('');
  let preview = $state<ParsedPreview | null>(null);
  let input: HTMLInputElement | undefined = $state();
  let error = $state('');

  $effect(() => {
    islandCommands.requestFocus().then(() => input?.focus());
    return () => {
      islandCommands.releaseFocus();
    };
  });

  $effect(() => {
    const value = text;
    if (!value.trim()) {
      preview = null;
      return;
    }
    const id = setTimeout(async () => (preview = await reminderCommands.parse(value)), 150);
    return () => clearTimeout(id);
  });

  async function save() {
    if (!preview?.title || preview.dueAt === null) {
      error = t('reminders.needTime');
      return;
    }
    await reminderCommands.create({ title: preview.title, note: null, dueAt: preview.dueAt, repeat: preview.repeat, leadMin: 0 });
    close();
  }
</script>

<div class="panel">
  <input
    bind:this={input}
    bind:value={text}
    placeholder={t('reminders.placeholder')}
    onkeydown={(e) => {
      e.stopPropagation();
      if (e.key === 'Enter') save();
      if (e.key === 'Escape') close();
    }}
  />
  <div class="preview" class:muted={!preview?.dueAt}>
    {#if preview}
      <strong>{preview.title || '…'}</strong> ·
      {preview.dueAt ? formatWhen(preview.dueAt) : t('reminders.noTime')}
      {#if preview.repeat.kind !== 'none'} · {t(`reminders.repeat.${preview.repeat.kind}`)}{/if}
    {:else}
      {t('reminders.hint')}
    {/if}
  </div>
  {#if error}<div class="error">{error}</div>{/if}
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }
  input {
    width: 100%;
    border: 1px solid var(--c-border-strong);
    border-radius: var(--r-control);
    background: var(--c-surface-2);
    color: var(--c-text-1);
    padding: var(--sp-4) var(--sp-5);
    font: inherit;
    outline: none;
    user-select: text;
  }
  input:focus {
    border-color: var(--c-accent);
  }
  .preview {
    font-size: var(--fs-small);
    color: var(--c-accent-bright);
  }
  .muted {
    color: var(--c-text-3);
  }
  .error {
    font-size: var(--fs-small);
    color: var(--c-warning);
  }
</style>
