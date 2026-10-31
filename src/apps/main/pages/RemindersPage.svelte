<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import AlarmClock from '@lucide/svelte/icons/alarm-clock';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Card from '$lib/components/Card.svelte';
  import Button from '$lib/components/Button.svelte';
  import { inTauri, reminderCommands } from '$lib/ipc/commands';
  import type { Reminder } from '$lib/ipc/bindings/Reminder';
  import type { ParsedPreview } from '$lib/ipc/bindings/ParsedPreview';
  import { live } from '$lib/stores/live.svelte';
  import { formatWhen } from '$lib/format';
  import { t } from '$lib/i18n/index.svelte';

  let tab = $state<'upcoming' | 'missed' | 'done'>('upcoming');
  let items = $state<Reminder[]>([]);
  let text = $state('');
  let preview = $state<ParsedPreview | null>(null);
  let manualWhen = $state('');

  async function load() {
    if (inTauri) items = await reminderCommands.list(tab);
  }
  $effect(() => {
    void tab;
    void live.reminders?.rev;
    load();
  });
  $effect(() => {
    const v = text;
    if (!v.trim()) return void (preview = null);
    const id = setTimeout(async () => (preview = await reminderCommands.parse(v)), 150);
    return () => clearTimeout(id);
  });

  async function add() {
    if (!preview?.title) return;
    const dueAt = manualWhen ? new Date(manualWhen).getTime() : preview.dueAt;
    if (!dueAt) return;
    await reminderCommands.create({ title: preview.title, note: null, dueAt, repeat: preview.repeat, leadMin: 0 });
    text = '';
    manualWhen = '';
    load();
  }
</script>

<h1>{t('main.nav.reminders')}</h1>

<Card>
  <div class="add">
    <input class="text" placeholder={t('reminders.placeholder')} bind:value={text} onkeydown={(e) => e.key === 'Enter' && add()} />
    <input type="datetime-local" bind:value={manualWhen} aria-label={t('reminders.when')} />
    <Button variant="primary" onclick={add} disabled={!preview?.title || (!preview.dueAt && !manualWhen)}>{t('common.add')}</Button>
  </div>
  <p class="preview">
    {#if preview}
      <strong>{preview.title}</strong> ·
      {manualWhen ? formatWhen(new Date(manualWhen).getTime()) : preview.dueAt ? formatWhen(preview.dueAt) : t('reminders.noTime')}
      {#if preview.repeat.kind !== 'none'} · {t(`reminders.repeat.${preview.repeat.kind}`)}{/if}
    {:else}
      {t('reminders.hint')}
    {/if}
  </p>
</Card>

<div class="tabs">
  {#each ['upcoming', 'missed', 'done'] as const as k (k)}
    <button class:active={tab === k} onclick={() => (tab = k)}>
      {t(`reminders.tab.${k}`)}{#if k === 'missed' && live.reminders?.missedCount} ({live.reminders.missedCount}){/if}
    </button>
  {/each}
</div>

<Card>
  {#each items as r (r.id)}
    <div class="item">
      <div class="grow">
        <div class="title" class:done={r.status === 'done'}>{r.title}</div>
        <div class="muted">
          {formatWhen(r.snoozedUntil ?? r.dueAt)}{#if r.repeat.kind !== 'none'} · {t(`reminders.repeat.${r.repeat.kind}`)}{/if}
        </div>
      </div>
      {#if tab === 'upcoming'}
        <Button variant="ghost" title={t('reminders.action.done')} onclick={() => reminderCommands.complete(r.id)}><Check size={14} /></Button>
        <Button variant="ghost" title={t('reminders.action.snooze10')} onclick={() => reminderCommands.snooze(r.id, 10)}><AlarmClock size={14} /></Button>
      {/if}
      <Button variant="danger" title={t('common.delete')} onclick={async () => { await reminderCommands.remove(r.id); load(); }}><Trash2 size={14} /></Button>
    </div>
  {:else}
    <p class="muted">{t('reminders.empty')}</p>
  {/each}
</Card>

<style>
  h1 {
    margin: 0 0 var(--sp-6);
    font-size: var(--fs-title);
  }
  .add {
    display: flex;
    gap: var(--sp-4);
  }
  input {
    border: 1px solid var(--c-border-strong);
    border-radius: var(--r-control);
    background: var(--c-surface-3);
    color: var(--c-text-1);
    padding: var(--sp-4) var(--sp-5);
    font: inherit;
    user-select: text;
    color-scheme: dark;
  }
  .text {
    flex: 1;
  }
  .preview {
    margin: var(--sp-4) 0 0;
    color: var(--c-accent-bright);
    font-size: var(--fs-small);
  }
  .tabs {
    display: flex;
    gap: var(--sp-2);
    margin-bottom: var(--sp-5);
  }
  .tabs button {
    border: 0;
    background: none;
    padding: var(--sp-3) var(--sp-5);
    border-radius: var(--r-pill);
    color: var(--c-text-2);
    cursor: pointer;
  }
  .tabs .active {
    background: var(--c-surface-2);
    color: var(--c-accent-bright);
  }
  .item {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-4) 0;
    border-bottom: 1px solid var(--c-border);
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .title {
    font-weight: var(--fw-medium);
  }
  .title.done {
    text-decoration: line-through;
    color: var(--c-text-3);
  }
  .muted {
    color: var(--c-text-3);
    font-size: var(--fs-small);
  }
</style>
