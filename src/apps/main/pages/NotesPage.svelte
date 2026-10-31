<script lang="ts">
  import Pin from '@lucide/svelte/icons/pin';
  import PinOff from '@lucide/svelte/icons/pin-off';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Plus from '@lucide/svelte/icons/plus';
  import Download from '@lucide/svelte/icons/download';
  import Pencil from '@lucide/svelte/icons/pencil';
  import { tick } from 'svelte';
  import NoteBody from '../../../features/notes/views/NoteBody.svelte';
  import ChecklistInput from '../../../features/notes/views/ChecklistInput.svelte';
  import { appendCheckbox } from '../../../features/notes/checklist';
  import { saveMarkdown } from '$lib/ipc/dialog';
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
  let creating = $state(false);
  let addingCheck = $state<string | null>(null);
  let savingCheck = $state(false);
  let checkError = $state(false);
  const recentNotes = $derived([...notes].sort((a, b) => b.updatedAt - a.updatedAt));
  let editing = $state<string | null>(null);
  let editText = $state('');
  let draftArea: HTMLTextAreaElement | undefined = $state();
  let editArea: HTMLTextAreaElement | undefined = $state();
  async function beginCreate() {
    creating = true;
    await tick();
    draftArea?.focus();
  }
  async function addCheck(note: Note, label: string) {
    if (savingCheck) return;
    savingCheck = true;
    checkError = false;
    try {
      await noteCommands.save(appendCheckbox(note.text) + label, note.id);
      addingCheck = null;
      await load();
    } catch { checkError = true; }
    finally { savingCheck = false; }
  }
  async function beginEdit(note: Note) {
    addingCheck = null;
    editing = note.id;
    editText = note.text;
    await tick();
    editArea?.focus();
  }
  async function insertCheckbox() {
    draft = appendCheckbox(draft);
    await tick();
    draftArea?.focus();
    draftArea?.setSelectionRange(draft.length, draft.length);
  }
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
    creating = false;
    load();
  }
  async function saveEdit(id: string) {
    if (editText.trim()) await noteCommands.save(editText, id);
    editing = null;
    load();
  }
  let exported = $state<number | null>(null);
  async function exportAll() {
    const dest = await saveMarkdown('pcisland-notes');
    if (dest) exported = await noteCommands.exportMarkdown(dest);
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
  <div class="bar">
    <input placeholder={t('notes.search')} bind:value={search} />
    <Button variant="ghost" title={t('notes.export')} onclick={exportAll} disabled={!notes.length}><Download size={14} />.md</Button>
    <Button variant="primary" onclick={beginCreate}><Plus size={14} />{t('notes.new')}</Button>
  </div>
  {#if creating}
    <textarea bind:this={draftArea} rows="3" aria-label={t('notes.new')} placeholder={t('notes.placeholder')} bind:value={draft} onkeydown={(event) => {
      if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) { event.preventDefault(); void create(); }
      if (event.key === 'Escape') creating = false;
    }}></textarea>
    <div class="bar">
      <Button variant="ghost" onclick={insertCheckbox}>{t('notes.addCheckbox')}</Button>
      <Button variant="primary" onclick={create} disabled={!draft.trim()}>{t('common.save')}</Button>
      <Button variant="ghost" onclick={() => creating = false}>{t('common.cancel')}</Button>
    </div>
  {/if}
  {#if exported !== null}<p class="undo">{t('notes.exported', { n: exported })}</p>{/if}
  {#if undo}
    <p class="undo">{t('notes.deleted')} <button onclick={async () => { if (undo) { await noteCommands.restore(undo.id); undo = null; load(); } }}>{t('common.undo')}</button></p>
  {/if}
</Card>

{#each recentNotes as n (n.id)}
  <Card>
    {#snippet actions()}
      <Button variant="ghost" disabled={savingCheck} onclick={() => { addingCheck = n.id; editing = null; checkError = false; }}>{t('notes.addCheckbox')}</Button>
      <Button variant="ghost" title={t('common.edit')} onclick={() => beginEdit(n)}><Pencil size={14} /></Button>
      <Button variant="ghost" title={n.pinned ? t('notes.action.unpin') : t('notes.pin')} onclick={() => noteCommands.pin(n.pinned ? null : n.id).then(load)}>
        {#if n.pinned}<PinOff size={14} />{:else}<Pin size={14} />{/if}
      </Button>
      <Button variant="danger" title={t('common.delete')} onclick={() => remove(n.id)}><Trash2 size={14} /></Button>
    {/snippet}
    {#if editing === n.id}
      <textarea bind:this={editArea} rows="5" bind:value={editText} aria-label={t('common.edit')} onkeydown={(event) => {
        if (event.key === 'Escape') editing = null;
        if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) { event.preventDefault(); void saveEdit(n.id); }
      }}></textarea>
      <div class="bar">
        <Button variant="primary" onclick={() => saveEdit(n.id)} disabled={!editText.trim()}>{t('common.save')}</Button>
        <Button variant="ghost" onclick={() => editing = null}>{t('common.cancel')}</Button>
      </div>
    {:else}
      <div class:pinned={n.pinned}>
        <NoteBody text={n.text} disabled={savingCheck} ontoggle={async (line) => { await noteCommands.toggleCheck(n.id, line); await load(); }} />
      </div>
      {#if addingCheck === n.id}
        <ChecklistInput busy={savingCheck} onadd={(label) => addCheck(n, label)} oncancel={() => addingCheck = null} />
        {#if checkError}<p role="alert">{t('notes.failed')}</p>{/if}
      {/if}
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
