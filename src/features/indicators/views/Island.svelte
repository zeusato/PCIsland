<script lang="ts">
  // Volume / power / battery alerts and the idle clock (docs/modules/system-indicators.md §4).
  import type { Activity } from '$lib/ipc/bindings/Activity';
  import { BatteryCharging, BatteryLow, BatteryMedium, Volume2, VolumeX, TriangleAlert } from '@lucide/svelte';
  import { t } from '$lib/i18n/index.svelte';
  import IdleEyes from './IdleEyes.svelte';
  import IdleClock from './IdleClock.svelte';

  let { activity }: { activity: Activity; variant: string } = $props();
  const p = $derived(activity.payload as Record<string, number | boolean | null>);
</script>

{#if activity.kind === 'indicators.idle'}
  {#if (activity.payload as { mode: string }).mode === 'clock'}<IdleClock />{:else}<IdleEyes />{/if}
{:else if activity.kind === 'indicators.volume'}
  <div class="row">
    {#if p.muted}<VolumeX size={16} />{:else}<Volume2 size={16} />{/if}
    <div class="bar"><div class="fill" style:transform={`scaleX(${p.muted ? 0 : (p.level as number)})`}></div></div>
    <span class="tabular pct">{Math.round((p.level as number) * 100)}</span>
  </div>
{:else if activity.kind === 'indicators.power'}
  <div class="row">
    {#if p.onAc}<BatteryCharging size={16} />{:else}<BatteryMedium size={16} />{/if}
    <span>{p.onAc ? t('indicators.charging') : t('indicators.onBattery')} · {p.percent}%</span>
  </div>
{:else if activity.kind === 'indicators.battery_low'}
  <div class="row warn" class:danger={(p.percent as number) <= 10}>
    <BatteryLow size={16} /><span>{t('indicators.batteryLow', { n: p.percent as number })}</span>
  </div>
{:else}
  <div class="row"><TriangleAlert size={16} /><span>{t(`alerts.${activity.kind}`)}</span></div>
{/if}

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--sp-4);
    width: 100%;
    padding: 0 var(--sp-6);
    font-size: var(--fs-body-island);
  }
  .bar {
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: var(--c-surface-3);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--c-accent);
    transform-origin: 0 50%;
    transition: transform 120ms ease-out;
  }
  .pct {
    width: 24px;
    text-align: right;
    color: var(--c-text-2);
  }
  .warn {
    color: var(--c-warning);
  }
  .danger {
    color: var(--c-danger);
  }
</style>
