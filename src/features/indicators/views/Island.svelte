<script lang="ts">
  // Volume / power / battery alerts and the idle screen (docs/modules/system-indicators.md §4).
  import type { Activity } from '$lib/ipc/bindings/Activity';
  import BatteryCharging from '@lucide/svelte/icons/battery-charging';
  import BatteryLow from '@lucide/svelte/icons/battery-low';
  import BatteryMedium from '@lucide/svelte/icons/battery-medium';
  import Volume2 from '@lucide/svelte/icons/volume-2';
  import VolumeX from '@lucide/svelte/icons/volume-x';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  import { t } from '$lib/i18n/index.svelte';
  import { payloadOf } from '$lib/registry/payloads';
  import IdleEyes from './IdleEyes.svelte';
  import IdleClock from './IdleClock.svelte';

  let { activity }: { activity: Activity; variant: string } = $props();
  const idle = $derived(payloadOf(activity, 'indicators.idle'));
  const volume = $derived(payloadOf(activity, 'indicators.volume'));
  const power = $derived(payloadOf(activity, 'indicators.power'));
  const low = $derived(payloadOf(activity, 'indicators.battery_low'));
</script>

{#if idle}
  {#if idle.mode === 'clock'}<IdleClock />{:else}<IdleEyes />{/if}
{:else if volume}
  <div class="row">
    {#if volume.muted}<VolumeX size={16} />{:else}<Volume2 size={16} />{/if}
    <div class="bar"><div class="fill" style:transform={`scaleX(${volume.muted ? 0 : volume.level})`}></div></div>
    <span class="tabular pct">{Math.round(volume.level * 100)}</span>
  </div>
{:else if power}
  <div class="row">
    {#if power.onAc}<BatteryCharging size={16} />{:else}<BatteryMedium size={16} />{/if}
    <span>{power.onAc ? t('indicators.charging') : t('indicators.onBattery')} · {power.percent}%</span>
  </div>
{:else if low}
  <div class="row warn" class:danger={low.percent <= 10}>
    <BatteryLow size={16} /><span>{t('indicators.batteryLow', { n: low.percent })}</span>
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
