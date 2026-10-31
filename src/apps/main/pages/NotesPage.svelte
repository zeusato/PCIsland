<script lang="ts">
  import { Pin, PinOff, Trash2, Plus } from '@lucide/svelte';
  import Card from '$lib/components/Card.svelte';
  import Button from '$lib/components/Button.svelte';
  import { inTauri, noteCommands } from '$lib/ipc/commands';
  import { onNotesState } from '$lib/ipc/events';
  import type { Note } from '$lib/ipc/bindings/Note';
  import { formatWhen } from '$lib/format';
  import { t } from '$lib/i18n/index.svelte';

  let notes = $state<Note[]>([]);
  let search = $state('');
  let draft = $state('');
  let editing = $state<string | null>(null);
  let editText = $state('');
  let undo = $state<{ id: string; timer: ReturnType<typeof setTimeout> } | null>(null);

  async function load() {
    if (inTauri) notes = await noteCommands.list(search);
  }
  $effect(() => {
    const q = search;
    const id = setTimeout(() => inTauri && noteCommands.list(q).then((n) => (notes = n)), 150);
    return () => clearTimeout(id);
  });
  $effect(() => {
    if (!inTauri) return;
    const sub = onNotesState(() => load());
    return () => void sub.then((u) => u());
  });

  async function create() {
    if (!draft.trim()) return;
    await noteCommands.save(draft);
    draft = '';
    load();
  }
  async function saveEdit(id: string) {
    if (editText.trim()) await noteCommands.save(editText, id);
    editing = null;
    load();
  }
  async function remove(id: string) {
    await noteCommands.remove(id);
    if (undo) clearTimeout(undo.timer);
    undo = { id, timer: setTimeout(() => (undo = null), 5000) };
    load();
  }
</script>

<h1>{t('main.nav.notes')}</h1>

<Card>
  <textarea rows="3" placeholder={t('notes.placeholder')} bind:value={draft}></textarea>
  <div class="bar">
    <input placeholder={t('notes.search')} bind:value={search} />
    <Button variant="primary" onclick={create} disabled={!draft.trim()}><Plus size={14} />{t('common.add')}</Button>
  </div>
  {#if undo}
    <p class="undo">{t('notes.deleted')} <button onclick={async () => { if (undo) { await noteCommands.restore(undo.id); undo = null; load(); } }}>{t('common.undo')}</button></p>
  {/if}
</Card>

{#each notes as n (n.id)}
  <Card>
    {#snippet actions()}
      <Button variant="ghost" title={n.pinned ? t('notes.action.unpin') : t('notes.pin')} onclick={() => noteCommands.pin(n.pinned ? null : n.id).then(load)}>
        {#if n.pinned}<PinOff size={14} />{:else}<Pin size={14} />{/if}
      </Button>
      <Button variant="danger" onclick={() => remove(n.id)}><Trash2 size={14} /></Button>
    {/snippet}
    {#if editing === n.id}
      <textarea rows="5" bind:value={editText} onblur={() => saveEdit(n.id)}></textarea>
    {:else}
      <button class="text" class:pinned={n.pinned} onclick={() => { editing = n.id; editText = n.text; }}>{n.text}</button>
    {/if}
    <p class="muted">{formatWhen(n.updatedAt)}</p>
  </Card>
{:else}
  <p class="muted">{t('notes.empty')}</p>
{/each}

<style>
  h1 {
    margin: 0 0 var(--sp-6);
    font-size: var(--fs-title);
  }
  textarea,
  input {
    width: 100%;
    border: 1px solid var(--c-border-strong);
    border-radius: var(--r-control);
    background: var(--c-surface-3);
    color: var(--c-text-1);
    padding: var(--sp-4) var(--sp-5);
    font: inherit;
    resize: vertical;
    user-select: text;
  }
  .bar {
    display: flex;
    gap: var(--sp-4);
    margin-top: var(--sp-4);
  }
  .text {
    display: block;
    width: 100%;
    border: 0;
    background: none;
    padding: 0;
    text-align: left;
    white-space: pre-wrap;
    cursor: text;
    user-select: text;
  }
  .pinned {
    border-left: 3px solid var(--c-accent);
    padding-left: var(--sp-4);
  }
  .muted {
    color: var(--c-text-3);
    font-size: var(--fs-small);
    margin: var(--sp-3) 0 0;
  }
  .undo button {
    border: 0;
    background: none;
    color: var(--c-accent-bright);
    cursor: pointer;
  }
</style>
