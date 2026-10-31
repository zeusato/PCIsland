<script lang="ts">
  // Quick note capture: Enter saves, Shift+Enter new line, Esc cancels (docs/modules/quick-notes.md §3).
  import { islandCommands, noteCommands } from '$lib/ipc/commands';
  import { t } from '$lib/i18n/index.svelte';

  let { close }: { close: () => void } = $props();
  let text = $state('');
  let area: HTMLTextAreaElement | undefined = $state();

  $effect(() => {
    islandCommands.requestFocus().then(() => area?.focus());
    return () => {
      islandCommands.releaseFocus();
    };
  });

  async function save() {
    if (text.trim()) await noteCommands.save(text);
    close();
  }
</script>

<textarea
  bind:this={area}
  bind:value={text}
  rows="3"
  placeholder={t('notes.placeholder')}
  onkeydown={(e) => {
    e.stopPropagation();
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      save();
    }
    if (e.key === 'Escape') close();
  }}
></textarea>

<style>
  textarea {
    width: 100%;
    resize: none;
    border: 1px solid var(--c-border-strong);
    border-radius: var(--r-control);
    background: var(--c-surface-2);
    color: var(--c-text-1);
    padding: var(--sp-4) var(--sp-5);
    font: inherit;
    outline: none;
    user-select: text;
  }
  textarea:focus {
    border-color: var(--c-accent);
  }
</style>
