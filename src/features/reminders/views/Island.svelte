<script lang="ts">
  import type { Activity } from '$lib/ipc/bindings/Activity';
  import type { Reminder } from '$lib/ipc/bindings/Reminder';
  import Bell from '@lucide/svelte/icons/bell';
  import BellRing from '@lucide/svelte/icons/bell-ring';
  import ActionButtons from '$lib/components/ActionButtons.svelte';
  import { payload, payloadOf } from '$lib/registry/payloads';
  import { formatTime, formatWhen } from '$lib/format';
  import { t } from '$lib/i18n/index.svelte';

  let { activity, variant }: { activity: Activity; variant: 'compact' | 'peek' | 'alert' | 'bubble' } = $props();
  const r: Reminder = $derived(payload(activity, 'reminders.due'));
  const fireAt = $derived(r?.snoozedUntil ?? r?.dueAt ?? 0);
</script>

{#if activity.kind === 'reminders.missed_summary'}
  <div class="small"><Bell size={16} /><span>{t('reminders.missed', { n: payloadOf(activity, 'reminders.missed_summary')?.count ?? 0 })}</span></div>
{:else if variant === 'bubble'}
  <div class="bubble"><Bell size={14} /></div>
{:else if variant === 'compact'}
  <div class="compact"><Bell size={16} /><span class="spacer"></span><span class="tabular">{formatTime(fireAt)}</span></div>
{:else}
  <div class="peek" class:alert={variant === 'alert'}>
    <div class="head">
      {#if activity.kind === 'reminders.due'}<BellRing size={20} />{:else}<Bell size={18} />{/if}
      <span class="title">{r.title}</span>
    </div>
    <div class="when">{formatWhen(fireAt)}{#if r.note} · {r.note}{/if}</div>
    <ActionButtons {activity} labels={variant === 'alert'} />
  </div>
{/if}

<style>
  .compact,
  .small {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    width: 100%;
    padding: 0 var(--sp-5);
    color: var(--c-accent-bright);
    font-size: var(--fs-body-island);
  }
  .small {
    justify-content: center;
    color: var(--c-text-1);
  }
  .spacer {
    flex: 1;
  }
  .bubble {
    display: grid;
    place-items: center;
    color: var(--c-accent);
  }
  .peek {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--sp-4);
    padding: var(--sp-5) var(--sp-7);
    height: 100%;
    text-align: center;
  }
  .head {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    color: var(--c-accent-bright);
  }
  .title {
    color: var(--c-text-1);
    font-size: var(--fs-title-s);
    font-weight: var(--fw-semibold);
    max-width: 320px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .when {
    color: var(--c-text-2);
    font-size: var(--fs-small);
  }
</style>
