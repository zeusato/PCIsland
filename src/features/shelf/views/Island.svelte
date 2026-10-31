<script lang="ts">
  import type { Activity } from '$lib/ipc/bindings/Activity';
  import Package from '@lucide/svelte/icons/package';
  import File from '@lucide/svelte/icons/file';
  import Folder from '@lucide/svelte/icons/folder';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import X from '@lucide/svelte/icons/x';
  import ActionButtons from '$lib/components/ActionButtons.svelte';
  import { shelfCommands } from '$lib/ipc/commands';
  import { dragFilesOut } from '$lib/ipc/drag';
  import { payloadOf } from '$lib/registry/payloads';
  import { t } from '$lib/i18n/index.svelte';

  let { activity, variant }: { activity: Activity; variant: string } = $props();
  const items = $derived(payloadOf(activity, 'shelf.items'));
  const dropped = $derived(payloadOf(activity, 'shelf.dropped'));
</script>

{#if dropped}
  <div class="row"><Package size={16} /><span>{t('shelf.dropped', { n: dropped.count })}</span></div>
{:else if variant !== 'peek'}
  <div class="row compact"><Package size={16} /><span class="tabular">{items?.count ?? 0}</span></div>
{:else if items}
  <div class="peek">
    <div class="grid">
      {#each items.items.slice(0, 8) as item (item.id)}
        <div
          class="tile"
          class:missing={item.missing}
          title={item.name}
          role="button"
          tabindex="-1"
          onmousedown={(e) => {
            if (e.button === 0 && !item.missing) {
              e.stopPropagation();
              dragFilesOut([item.path]);
            }
          }}
          ondblclick={(e) => {
            e.stopPropagation();
            shelfCommands.open(item.path);
          }}
        >
          {#if item.thumbnailUrl}
            <img src={item.thumbnailUrl} alt="" draggable="false" />
          {:else if item.isDir}<Folder size={22} />{:else}<File size={22} />{/if}
          <span class="name">{item.name}</span>
          <span class="tools">
            <button aria-label={t('shelf.reveal')} onmousedown={(e) => e.stopPropagation()} onclick={(e) => { e.stopPropagation(); shelfCommands.open(item.path, true); }}><FolderOpen size={12} /></button>
            <button aria-label={t('shelf.remove')} onmousedown={(e) => e.stopPropagation()} onclick={(e) => { e.stopPropagation(); shelfCommands.remove(item.id); }}><X size={12} /></button>
          </span>
        </div>
      {/each}
    </div>
    <div class="hint">{t('shelf.dragHint')}</div>
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
    gap: var(--sp-3);
    padding: var(--sp-5) var(--sp-6);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--sp-3);
  }
  .tile {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sp-1);
    padding: var(--sp-3) var(--sp-2);
    border-radius: var(--r-control);
    background: var(--c-surface-2);
    color: var(--c-text-2);
    cursor: grab;
  }
  .tile:hover {
    background: var(--c-surface-3);
  }
  .tile.missing {
    opacity: 0.4;
    cursor: default;
  }
  img {
    width: 28px;
    height: 28px;
    object-fit: contain;
  }
  .name {
    max-width: 100%;
    font-size: var(--fs-caption);
    color: var(--c-text-1);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tools {
    position: absolute;
    top: 2px;
    right: 2px;
    display: none;
  }
  .tile:hover .tools {
    display: flex;
  }
  .tools button {
    border: 0;
    background: var(--c-surface-1);
    border-radius: 4px;
    padding: 1px;
    cursor: pointer;
    color: var(--c-text-2);
  }
  .hint {
    font-size: var(--fs-caption);
    color: var(--c-text-3);
    text-align: center;
  }
</style>
