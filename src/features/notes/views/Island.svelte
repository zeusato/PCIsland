<script lang="ts">
  import type { Activity } from '$lib/ipc/bindings/Activity';
  import type { Note } from '$lib/ipc/bindings/Note';
  import { Pin, Check } from '@lucide/svelte';
  import ActionButtons from '$lib/components/ActionButtons.svelte';
  import { noteCommands } from '$lib/ipc/commands';
  import { t } from '$lib/i18n/index.svelte';

  let { activity, variant }: { activity: Activity; variant: string } = $props();
  const note = $derived(activity.payload as Note);
  const lines = $derived(note?.text?.split('\n').slice(0, 6) ?? []);
</script>

{#if activity.kind === 'notes.saved'}
  <div class="row"><Check size={16} /><span>{t('notes.saved')}</span></div>
{:else if variant === 'compact' || variant === 'bubble'}
  <div class="row compact"><Pin size={14} /><span class="snippet">{note.text.split('\n')[0]?.slice(0, 24)}</span></div>
{:else}
  <div class="peek">
    {#each lines as line, i (i)}
      {@const check = line.trimStart().match(/^\[( |x)\]\s?(.*)$/)}
      {#if check}
        <button class="check" class:done={check[1] === 'x'} onclick={(e) => { e.stopPropagation(); noteCommands.toggleCheck(note.id, i); }}>
          <span class="box">{check[1] === 'x' ? '✓' : ''}</span>{check[2]}
        </button>
      {:else}
        <p>{line}</p>
      {/if}
    {/each}
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
    overflow: hidden;
  }
  p {
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .check {
    display: flex;
    gap: var(--sp-4);
    align-items: center;
    border: 0;
    background: none;
    padding: 0;
    cursor: pointer;
    text-align: left;
  }
  .box {
    display: grid;
    place-items: center;
    width: 14px;
    height: 14px;
    border: 1px solid var(--c-border-strong);
    border-radius: 4px;
    font-size: 10px;
    color: var(--c-accent);
  }
  .done {
    color: var(--c-text-3);
    text-decoration: line-through;
  }
</style>
