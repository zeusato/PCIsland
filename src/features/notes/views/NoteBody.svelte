<script lang="ts">
  import { checklistLine } from '../checklist';
  import { t, getLocale } from '$lib/i18n/index.svelte';
  import { ceilSeconds, formatDuration } from '$lib/format';
  import Bell from '@lucide/svelte/icons/bell';
  import Check from '@lucide/svelte/icons/check';
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
  let now = $state(Date.now());
  $effect(() => {
    if (!tasks.some((task) => task.alarm && task.alarm.mode !== 'at' && task.dueAt !== null && !task.checked && !task.firedAt)) return;
    now = Date.now();
    const timer = setInterval(() => now = Date.now(), 1000);
    return () => clearInterval(timer);
  });
  function alarmLabel(task: NoteTask) {
    if (task.firedAt) return t('notes.alarmDue');
    if (task.dueAt === null) return `${t('notes.waitingShort')} · ${formatDuration(task.alarm?.delayMs ?? 0)}`;
    if (task.alarm?.mode !== 'at') return formatDuration(ceilSeconds(task.dueAt - now));
    return new Date(task.dueAt).toLocaleString(getLocale() === 'vi' ? 'vi-VN' : 'en-GB', {
      day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false,
    });
  }
  $effect(() => {
    const id = noteId;
    void text;
    tasks = [];
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
        <span class="box" aria-hidden="true">{#if task.checked}<Check strokeWidth={3} />{/if}</span><span>{task.text}</span>
      </button>
      {#if saved?.alarm && !task.checked}<span class="alarm-time tabular" title={saved.dueAt !== null ? new Date(saved.dueAt).toLocaleString() : t('notes.alarmWaiting')}>{alarmLabel(saved)}</span>{/if}
      {#if saved}<button class="alarm" class:armed={!!saved.alarm} aria-label={t('notes.alarm')} title={t('notes.alarm')} disabled={busy || disabled || (task.checked && !saved.alarm)} onclick={(event) => { event.stopPropagation(); editingAlarm = editingAlarm === saved.id ? null : saved.id; }}><Bell size={14} /></button>{/if}
      </div>
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
  .alarm-time { flex: none; max-width: 45%; padding-top: var(--sp-1); color: var(--c-accent-bright); font-size: var(--fs-caption); text-align: right; }
  /* Scales with the note font (desktop note zoom) and stays centred on the first text line. */
  .box { --box: max(var(--sp-6), 1.1em); flex: none; display: grid; place-items: center; width: var(--box); height: var(--box); margin-top: max(0px, calc((1lh - var(--box)) / 2)); border: 1px solid var(--c-border-strong); border-radius: var(--sp-2); color: var(--c-accent); }
  .box :global(svg) { width: 75%; height: 75%; }
  .done { color: var(--c-text-3); text-decoration: line-through; }
</style>
