<script lang="ts">
  import type { Activity } from '$lib/ipc/bindings/Activity';
  import type { ShelfItem } from '$lib/ipc/bindings/ShelfItem';
  import { Package, File, Folder, FolderOpen, X } from '@lucide/svelte';
  import ActionButtons from '$lib/components/ActionButtons.svelte';
  import { shelfCommands } from '$lib/ipc/commands';
  import { t } from '$lib/i18n/index.svelte';

  let { activity, variant }: { activity: Activity; variant: string } = $props();
  const data = $derived(activity.payload as { count: number; items?: ShelfItem[] });
</script>

{#if activity.kind === 'shelf.dropped'}
  <div class="row"><Package size={16} /><span>{t('shelf.dropped', { n: data.count })}</span></div>
{:else if variant !== 'peek'}
  <div class="row compact"><Package size={16} /><span class="tabular">{data.count}</span></div>
{:else}
  <div class="peek">
    <ul>
      {#each (data.items ?? []).slice(0, 5) as item (item.id)}
        <li class:missing={item.missing}>
          {#if item.isDir}<Folder size={14} />{:else}<File size={14} />{/if}
          <button class="name" onclick={(e) => { e.stopPropagation(); shelfCommands.open(item.path); }}>{item.name}</button>
          <button aria-label={t('shelf.reveal')} onclick={(e) => { e.stopPropagation(); shelfCommands.open(item.path, true); }}><FolderOpen size={14} /></button>
          <button aria-label={t('shelf.remove')} onclick={(e) => { e.stopPropagation(); shelfCommands.remove(item.id); }}><X size={14} /></button>
        </li>
      {/each}
    </ul>
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
    justify-content: space-between;
    color: var(--c-text-2);
  }
  .peek {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    padding: var(--sp-5) var(--sp-7);
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
  }
  li {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    font-size: var(--fs-small);
  }
  li.missing {
    opacity: 0.4;
  }
  button {
    border: 0;
    background: none;
    padding: var(--sp-1);
    cursor: pointer;
    color: var(--c-text-2);
  }
  button:hover {
    color: var(--c-accent-bright);
  }
  .name {
    flex: 1;
    text-align: left;
    color: var(--c-text-1);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
</style>
