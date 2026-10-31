<script lang="ts">
  import { File, Folder, FolderOpen, X, Trash2 } from '@lucide/svelte';
  import Card from '$lib/components/Card.svelte';
  import Button from '$lib/components/Button.svelte';
  import { inTauri, shelfCommands } from '$lib/ipc/commands';
  import { live } from '$lib/stores/live.svelte';
  import { formatWhen } from '$lib/format';
  import { t } from '$lib/i18n/index.svelte';

  $effect(() => {
    if (inTauri) shelfCommands.list().then((items) => (live.shelf = items));
  });
</script>

<h1>{t('main.nav.shelf')}</h1>

<Card>
  {#snippet actions()}
    <Button variant="danger" onclick={() => shelfCommands.clear()} disabled={!live.shelf.length}><Trash2 size={14} />{t('shelf.action.clear')}</Button>
  {/snippet}
  <p class="muted">{t('shelf.hint')}</p>
  {#each live.shelf as item (item.id)}
    <div class="item" class:missing={item.missing}>
      {#if item.isDir}<Folder size={16} />{:else}<File size={16} />{/if}
      <button class="name" onclick={() => shelfCommands.open(item.path)}>{item.name}</button>
      <span class="muted">{formatWhen(item.addedAt)}</span>
      <Button variant="ghost" onclick={() => shelfCommands.open(item.path, true)}><FolderOpen size={14} /></Button>
      <Button variant="ghost" onclick={() => shelfCommands.remove(item.id)}><X size={14} /></Button>
    </div>
  {:else}
    <p class="muted">{t('shelf.empty')}</p>
  {/each}
</Card>

<style>
  h1 {
    margin: 0 0 var(--sp-6);
    font-size: var(--fs-title);
  }
  .item {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    padding: var(--sp-3) 0;
    border-bottom: 1px solid var(--c-border);
  }
  .missing {
    opacity: 0.4;
  }
  .name {
    flex: 1;
    border: 0;
    background: none;
    text-align: left;
    cursor: pointer;
  }
  .name:hover {
    color: var(--c-accent-bright);
  }
  .muted {
    color: var(--c-text-3);
    font-size: var(--fs-small);
  }
</style>
