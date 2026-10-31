<script lang="ts">
  // Internet radio stations (F13, docs/modules/local-player.md §3).
  import Play from '@lucide/svelte/icons/play';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Radio from '@lucide/svelte/icons/radio';
  import Button from '$lib/components/Button.svelte';
  import { inTauri, playerCommands } from '$lib/ipc/commands';
  import type { Station } from '$lib/ipc/bindings/Station';
  import { t } from '$lib/i18n/index.svelte';

  let stations = $state<Station[]>([]);
  let name = $state('');
  let url = $state('');
  let error = $state('');

  $effect(() => {
    if (inTauri) playerCommands.stations().then((s) => (stations = s));
  });

  async function add() {
    error = '';
    try {
      stations = await playerCommands.addStation(name || url, url);
      name = '';
      url = '';
    } catch {
      error = t('radio.badUrl');
    }
  }
</script>

<div class="add">
  <input placeholder={t('radio.name')} bind:value={name} />
  <input class="url" placeholder="https://…" bind:value={url} onkeydown={(e) => e.key === 'Enter' && add()} />
  <Button variant="primary" onclick={add} disabled={!url.trim()}>{t('common.add')}</Button>
</div>
{#if error}<p class="error">{error}</p>{/if}
<p class="muted">{t('radio.hint')}</p>
{#each stations as s (s.id)}
  <div class="row">
    <Radio size={14} />
    <span class="grow">{s.name}</span>
    <Button variant="ghost" onclick={() => playerCommands.playStation(s.id)}><Play size={14} /></Button>
    <Button variant="danger" onclick={async () => (stations = await playerCommands.removeStation(s.id))}><Trash2 size={14} /></Button>
  </div>
{:else}
  <p class="muted">{t('radio.empty')}</p>
{/each}

<style>
  .add {
    display: flex;
    gap: var(--sp-4);
  }
  input {
    border: 1px solid var(--c-border-strong);
    border-radius: var(--r-control);
    background: var(--c-surface-3);
    color: var(--c-text-1);
    padding: var(--sp-3) var(--sp-4);
    font: inherit;
    user-select: text;
  }
  .url {
    flex: 1;
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    padding: var(--sp-3) 0;
    border-bottom: 1px solid var(--c-border);
  }
  .grow {
    flex: 1;
  }
  .muted {
    color: var(--c-text-3);
    font-size: var(--fs-small);
  }
  .error {
    color: var(--c-warning);
    font-size: var(--fs-small);
  }
</style>
