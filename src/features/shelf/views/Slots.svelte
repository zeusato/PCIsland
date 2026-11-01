<script lang="ts">
  import Plus from '@lucide/svelte/icons/plus';
  import File from '@lucide/svelte/icons/file';
  import Folder from '@lucide/svelte/icons/folder';
  import X from '@lucide/svelte/icons/x';
  import type { ShelfItem } from '$lib/ipc/bindings/ShelfItem';
  import { shelfCommands } from '$lib/ipc/commands';
  import { dragFilesOut } from '$lib/ipc/drag';
  import { t } from '$lib/i18n/index.svelte';
  let { items, receiving = false }: { items: ShelfItem[]; receiving?: boolean } = $props();
</script>

<div class="shelf">
  <div class="slots">
    {#each items.slice(0, 3) as item (item.id)}
      <div class="slot" class:missing={item.missing}>
        <button class="file" title={item.name} disabled={item.missing} onmousedown={(event) => { if (event.button === 0 && !receiving) { event.stopPropagation(); void dragFilesOut([item.path]); } }} ondblclick={(event) => { event.stopPropagation(); void shelfCommands.open(item.path); }}>
          {#if item.thumbnailUrl}<img src={item.thumbnailUrl} alt="" draggable="false" />{:else if item.isDir}<Folder size={32} />{:else}<File size={32} />{/if}
          <span>{item.name}</span>
        </button>
        <button class="remove" aria-label={t('shelf.remove')} onclick={(event) => { event.stopPropagation(); void shelfCommands.remove(item.id); }}><X size={14} /></button>
      </div>
    {/each}
    {#if items.length < 3}<div class="slot empty" class:receiving><Plus size={32} /><span>{t('shelf.dropHere')}</span></div>{/if}
  </div>
  <p>{t(items.length >= 3 ? 'shelf.full' : 'shelf.dragHint')}</p>
</div>

<style>
  .shelf { min-width: 0; padding: var(--sp-5); }
  .slots { display: flex; justify-content: center; gap: var(--sp-4); }
  .slot { width: var(--shelf-slot); height: var(--shelf-slot); position: relative; border: 1px solid var(--c-border-strong); border-radius: var(--r-control); background: var(--c-surface-2); min-width: 0; }
  .file { display: flex; align-items: center; justify-content: center; flex-direction: column; gap: var(--sp-3); width: 100%; height: 100%; background: none; border: 0; cursor: grab; padding: var(--sp-3); }
  .file span { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--fs-caption); }
  img { width: var(--shelf-thumb); height: var(--shelf-thumb); object-fit: contain; }
  .empty { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: var(--sp-3); border-style: dashed; color: var(--c-text-2); font-size: var(--fs-caption); }
  .receiving { border-color: var(--c-accent); color: var(--c-accent-bright); background: var(--c-accent-soft); }
  .remove { position: absolute; right: var(--sp-1); top: var(--sp-1); border: 0; background: var(--c-surface-3); border-radius: var(--r-control); cursor: pointer; }
  .missing { opacity: 0.5; }
  p { text-align: center; color: var(--c-text-2); font-size: var(--fs-caption); margin: var(--sp-4) 0 0; }
</style>
