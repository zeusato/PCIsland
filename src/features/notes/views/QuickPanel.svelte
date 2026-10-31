<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { inTauri, islandCommands, noteCommands } from '$lib/ipc/commands';
  import { onNotesState } from '$lib/ipc/events';
  import type { Note } from '$lib/ipc/bindings/Note';
  import { t } from '$lib/i18n/index.svelte';
  import { appendCheckbox } from '../checklist';
  import NoteBody from './NoteBody.svelte';
  import ChecklistInput from './ChecklistInput.svelte';

  let { close }: { close: () => void } = $props();
  let notes = $state<Note[]>([]);
  let selected = $state('');
  const note = $derived(notes.find((item) => item.id === selected));
  let mode = $state<'view' | 'create' | 'edit' | 'check'>('view');
  let text = $state('');
  let busy = $state(false);
  let loading = $state(true);
  let error = $state(false);
  let area: HTMLTextAreaElement | undefined = $state();
  let revision = 0;

  async function load() {
    const request = ++revision;
    try {
      const result = await noteCommands.list();
      if (request !== revision) return;
      notes = result.sort((a, b) => b.updatedAt - a.updatedAt);
      if (!notes.some((item) => item.id === selected)) selected = notes[0]?.id ?? '';
      error = false;
    } catch { if (request === revision) error = true; }
    finally { if (request === revision) loading = false; }
  }
  onMount(() => {
    void load();
    const sub = inTauri ? onNotesState(() => void load()) : null;
    return () => { revision++; void sub?.then((unlisten) => unlisten()); };
  });
  $effect(() => {
    if (mode === 'view' || !inTauri) return;
    let active = true;
    void islandCommands.requestFocus().then(() => {
      if (active) { if (mode === 'check') checkInput?.focus(); else area?.focus(); }
    });
    return () => { active = false; void islandCommands.releaseFocus(); };
  });
  let checkInput: ChecklistInput | undefined = $state();

  async function begin(next: 'create' | 'edit') {
    text = next === 'edit' ? note?.text ?? '' : '';
    mode = next;
    await tick();
    area?.focus();
  }
  async function mutate(action: () => Promise<Note>) {
    if (busy) return;
    busy = true;
    error = false;
    try {
      const saved = await action();
      selected = saved.id;
      await load();
      mode = 'view';
    } catch { error = true; }
    finally { busy = false; }
  }
  async function save() {
    if (!text.trim()) return;
    const id = mode === 'edit' ? selected : undefined;
    await mutate(() => noteCommands.save(text, id));
  }
</script>

<div class="panel" role="dialog" tabindex="-1" aria-label={t('main.nav.notes')} onclick={(event) => event.stopPropagation()} ondblclick={(event) => event.stopPropagation()} onkeydown={(event) => {
  event.stopPropagation();
  if (event.key === 'Escape' && !busy) { if (mode === 'view') close(); else mode = 'view'; }
}}>
  {#if mode === 'create' || mode === 'edit'}
    <textarea bind:this={area} bind:value={text} rows="2" aria-label={t(mode === 'create' ? 'notes.new' : 'common.edit')} placeholder={t('notes.placeholder')} disabled={busy} onkeydown={(event) => {
      if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) { event.preventDefault(); void save(); }
    }}></textarea>
    <div class="actions">
      <button disabled={busy} onclick={async () => { text = appendCheckbox(text); await tick(); area?.focus(); area?.setSelectionRange(text.length, text.length); }}>{t('notes.addCheckbox')}</button>
      <button disabled={busy || !text.trim()} onclick={save}>{t('common.save')}</button>
      <button disabled={busy} onclick={() => mode = 'view'}>{t('common.cancel')}</button>
    </div>
  {:else}
    {#if notes.length}
      <select aria-label={t('notes.choose')} bind:value={selected} disabled={busy || mode === 'check'}>
        {#each notes as item (item.id)}<option value={item.id}>{item.text.split('\n').find((line) => line.trim())?.slice(0, 60)}</option>{/each}
      </select>
    {/if}
    <div class="note-content">
      {#if note}
        <NoteBody text={note.text} disabled={busy || mode === 'check'} ontoggle={(line) => mutate(() => noteCommands.toggleCheck(selected, line))} />
      {:else}<p>{t(loading ? 'common.loading' : 'notes.empty')}</p>{/if}
    </div>
    {#if mode === 'check' && note}
      <ChecklistInput bind:this={checkInput} {busy} onadd={(label) => mutate(() => noteCommands.save(appendCheckbox(note!.text) + label, selected))} oncancel={() => mode = 'view'} />
    {:else}
      <div class="actions">
        <button disabled={busy || loading} onclick={() => begin('create')}>{t('notes.new')}</button>
        <button disabled={busy || !note} onclick={() => mode = 'check'}>{t('notes.addCheckbox')}</button>
        <button disabled={busy || !note} onclick={() => begin('edit')}>{t('common.edit')}</button>
      </div>
    {/if}
  {/if}
  {#if error}<p role="alert">{t('notes.failed')}</p>{/if}
</div>

<style>
  .panel { display: flex; flex: 1; min-height: 0; flex-direction: column; gap: var(--sp-3); font-size: var(--fs-body-island); }
  .note-content { flex: 1; min-height: 0; overflow-y: auto; }
  p { margin: 0; color: var(--c-text-2); }
  .actions { display: flex; flex-wrap: wrap; gap: var(--sp-3); }
  button, select, textarea { border: 1px solid var(--c-border-strong); border-radius: var(--r-control); background: var(--c-surface-2); color: var(--c-text-1); font: inherit; padding: var(--sp-2) var(--sp-4); }
  button { cursor: pointer; }
  button:disabled { opacity: 0.4; cursor: default; }
  select { width: 100%; min-width: 0; }
  textarea { width: 100%; flex: 1; min-height: 0; resize: none; user-select: text; }
  textarea:focus { outline-color: var(--c-accent); }
</style>
