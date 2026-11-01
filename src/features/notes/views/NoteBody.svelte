<script lang="ts">
  import { checklistLine } from '../checklist';
  import { t } from '$lib/i18n/index.svelte';
  import Bell from '@lucide/svelte/icons/bell';
  import { inTauri, noteCommands } from '$lib/ipc/commands';
  import { onNotesState } from '$lib/ipc/events';
  import type { NoteTask } from '$lib/ipc/bindings/NoteTask';
  import TaskAlarmEditor from './TaskAlarmEditor.svelte';

  let { text, limit, ontoggle, onedit, disabled = false, noteId, island = false }: {
    text: string;
    limit?: number;
    ontoggle: (line: number) => Promise<unknown>;
    onedit?: () => void;
    disabled?: boolean;
    noteId?: string;
    island?: boolean;
  } = $props();
  let busy = $state(false);
  let tasks: NoteTask[] = $state([]);
  let editingAlarm: string | null = $state(null);
  $effect(() => {
    const id = noteId;
    void text;
    if (!id || !inTauri) return;
    let active = true;
    let revision = 0;
    async function load() {
      const request = ++revision;
      try { const result = await noteCommands.tasks(id!); if (active && revision === request) tasks = result; } catch { /* Retry on the next notes update. */ }
    }
    void load();
    const listener = onNotesState(() => void load());
    return () => { active = false; void listener.then((off) => off()); };
  });
  const lines = $derived(text.split('\n').slice(0, limit));
  async function toggle(line: number) {
    if (busy) return;
    busy = true;
    try { await ontoggle(line); } finally { busy = false; }
  }
</script>

<div class="body">
  {#each lines as line, i (i)}
    {@const task = checklistLine(line)}
    {#if task}
      {@const saved = tasks.find((entry) => entry.lineIndex === i)}
      <div class="task-row">
      <button class="task" class:done={task.checked} role="checkbox" aria-checked={task.checked} aria-label={task.text || t('notes.checkbox')} disabled={busy || disabled} onclick={(event) => { event.stopPropagation(); void toggle(i); }}>
        <span class="box" aria-hidden="true">{task.checked ? '✓' : ''}</span><span>{task.text}</span>
      </button>
      {#if saved}<button class="alarm" class:armed={!!saved.alarm} aria-label={t('notes.alarm')} title={t('notes.alarm')} disabled={busy || disabled || (task.checked && !saved.alarm)} onclick={(event) => { event.stopPropagation(); editingAlarm = editingAlarm === saved.id ? null : saved.id; }}><Bell size={14} /></button>{/if}
      </div>
      {#if saved?.alarm && !task.checked}<small>{saved.firedAt ? t('notes.alarmDue') : saved.dueAt ? new Date(saved.dueAt).toLocaleString() : t('notes.alarmWaiting')}</small>{/if}
      {#if saved && editingAlarm === saved.id}<TaskAlarmEditor task={saved} hasPrevious={tasks.some((entry) => entry.lineIndex < i)} {island} close={() => editingAlarm = null} />{/if}
    {:else if onedit}
      <button class="line" onclick={onedit}>{line || '\u00a0'}</button>
    {:else}
      <p class="line">{line || '\u00a0'}</p>
    {/if}
  {/each}
</div>

<style>
  .body { display: flex; flex-direction: column; gap: var(--sp-2); min-width: 0; }
  .line, .task { margin: 0; padding: 0; border: 0; background: none; text-align: left; font: inherit; color: inherit; white-space: pre-wrap; overflow-wrap: anywhere; }
  button { cursor: pointer; }
  .task-row { display: flex; gap: var(--sp-3); align-items: flex-start; }
  .task { display: flex; align-items: flex-start; gap: var(--sp-4); flex: 1; min-width: 0; }
  .alarm { display: grid; place-items: center; border: 0; background: none; color: var(--c-text-3); padding: var(--sp-2); }
  .alarm.armed { color: var(--c-accent); }
  small { color: var(--c-text-3); font-size: var(--fs-caption); }
  .box { flex: none; display: grid; place-items: center; width: var(--sp-6); height: var(--sp-6); border: 1px solid var(--c-border-strong); border-radius: var(--sp-2); color: var(--c-accent); font-size: var(--fs-caption); }
  .done { color: var(--c-text-3); text-decoration: line-through; }
</style>
