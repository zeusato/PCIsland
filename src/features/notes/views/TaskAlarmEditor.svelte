<script lang="ts">
  import { tick, untrack } from 'svelte';
  import type { NoteTask } from '$lib/ipc/bindings/NoteTask';
  import { inTauri, islandCommands, noteCommands } from '$lib/ipc/commands';
  import { t } from '$lib/i18n/index.svelte';
  let { task, hasPrevious, island = false, close }: { task: NoteTask; hasPrevious: boolean; island?: boolean; close: () => void } = $props();
  let mode = $state(untrack(() => task.alarm?.mode ?? 'afterNow'));
  let amount = $state(untrack(() => (task.alarm?.delayMs ?? 60000) / 1000));
  let unit = $state(1);
  const localDate = (value: number) => {
    const date = new Date(value); return new Date(value - date.getTimezoneOffset() * 60000).toISOString().slice(0,19);
  };
  let at = $state(untrack(() => localDate(task.alarm?.at ?? Date.now() + 60000)));
  let busy = $state(false);
  let error = $state(false);
  let form: HTMLFormElement | undefined = $state();
  $effect(() => {
    if (!island || !inTauri) return;
    let active = true;
    void islandCommands.requestFocus().then(async () => { await tick(); if (active) form?.querySelector<HTMLInputElement>('input')?.focus(); });
    return () => { active = false; void islandCommands.releaseFocus(); };
  });
  async function save(remove = false) {
    if (busy) return;
    busy = true; error = false;
    try {
      const due = new Date(at).getTime();
      const delayMs = Math.round(amount * unit * 1000);
      if (!remove && (mode === 'at' ? !Number.isFinite(due) || due <= Date.now() : !Number.isFinite(delayMs) || delayMs < 1000)) { error = true; return; }
      await noteCommands.setTaskAlarm(task.id, remove ? null : { mode, at: mode === 'at' ? due : null, delayMs: mode !== 'at' ? delayMs : null });
      close();
    } catch { error = true; }
    finally { busy = false; }
  }
</script>

<form bind:this={form} aria-label={t('notes.alarm')} onsubmit={(event) => { event.preventDefault(); event.stopPropagation(); void save(); }}>
  <select aria-label={t('notes.alarmMode')} bind:value={mode} disabled={busy || task.checked}>
    <option value="at">{t('notes.alarmAt')}</option><option value="afterNow">{t('notes.alarmAfterNow')}</option><option value="afterPrevious" disabled={!hasPrevious}>{t('notes.alarmAfterPrevious')}</option>
  </select>
  {#if mode === 'at'}<input type="datetime-local" step="1" aria-label={t('notes.alarmAt')} bind:value={at} disabled={busy || task.checked} />
  {:else}<div class="row"><input type="number" min="1" aria-label={t('notes.alarmDelay')} bind:value={amount} disabled={busy || task.checked} /><select aria-label={t('notes.alarmUnit')} bind:value={unit} disabled={busy || task.checked}><option value={1}>{t('notes.seconds')}</option><option value={60}>{t('timer.minutes')}</option><option value={3600}>{t('notes.hours')}</option></select></div>{/if}
  <div class="row"><button disabled={busy || task.checked} type="submit">{t('common.save')}</button>{#if task.alarm}<button disabled={busy} type="button" onclick={() => save(true)}>{t('notes.removeAlarm')}</button>{/if}<button disabled={busy} type="button" onclick={close}>{t('common.cancel')}</button></div>
  {#if error}<p role="alert">{t('notes.alarmInvalid')}</p>{/if}
</form>

<style>
  form { display: flex; flex-direction: column; gap: var(--sp-3); padding: var(--sp-4); background: var(--c-surface-2); border: 1px solid var(--c-border-strong); border-radius: var(--r-control); }
  .row { display: flex; gap: var(--sp-3); }
  input, select, button { min-width: 0; background: var(--c-surface-3); color: var(--c-text-1); border: 1px solid var(--c-border-strong); border-radius: var(--r-control); padding: var(--sp-3); font: inherit; }
  input { flex: 1; width: 100%; user-select: text; }
  button { cursor: pointer; font-size: var(--fs-small); }
  p { color: var(--c-warning); font-size: var(--fs-small); margin: 0; }
</style>
