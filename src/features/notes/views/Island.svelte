<script lang="ts">
  import type { Activity } from '$lib/ipc/bindings/Activity';
  import type { Note } from '$lib/ipc/bindings/Note';
  import Pin from '@lucide/svelte/icons/pin';
  import Check from '@lucide/svelte/icons/check';
  import ActionButtons from '$lib/components/ActionButtons.svelte';
  import { payload } from '$lib/registry/payloads';
  import { noteCommands } from '$lib/ipc/commands';
  import { t } from '$lib/i18n/index.svelte';
  import NoteBody from './NoteBody.svelte';

  let { activity, variant }: { activity: Activity; variant: string } = $props();
  const note: Note = $derived(payload(activity, 'notes.pinned'));
</script>

{#if activity.kind === 'notes.task_due'}
  <div class="peek"><span>{payload(activity, 'notes.task_due').label}</span><ActionButtons {activity} /></div>
{:else if activity.kind === 'notes.saved'}
  <div class="row"><Check size={16} /><span>{t('notes.saved')}</span></div>
{:else if variant === 'compact' || variant === 'bubble'}
  <div class="row compact"><Pin size={14} /><span class="snippet">{note.text.split('\n')[0]?.slice(0, 24)}</span></div>
{:else}
  <div class="peek">
    <NoteBody noteId={note.id} island text={note.text} limit={6} ontoggle={(line) => noteCommands.toggleCheck(note.id, line)} />
    <ActionButtons {activity} />
  </div>
{/if}

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--sp-4);
    width: 100%;
    padding: 0 var(--sp-5);
    font-size: var(--fs-body-island);
  }
  .compact {
    justify-content: flex-start;
    color: var(--c-text-2);
  }
  .snippet {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .peek {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    padding: var(--sp-6) var(--sp-7);
    font-size: var(--fs-body-island);
    min-height: 0;
    overflow: auto;
  }
</style>
