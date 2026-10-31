<script lang="ts">
  import type { Activity } from '$lib/ipc/bindings/Activity';
  import { now, useTicker } from '$lib/stores/clock.svelte';
  import { t } from '$lib/i18n/index.svelte';
  import { activityProgress, progressRunning } from './progress';

  let { activity }: { activity: Activity | null } = $props();
  const progress = $derived(activityProgress(activity, now()));
  $effect(() => {
    if (progressRunning(activity) && activityProgress(activity, Date.now()) !== null) return useTicker(250);
  });
</script>

<div class="track" role="progressbar" aria-label={t('island.progress')} aria-valuemin={0} aria-valuemax={100} aria-valuenow={progress === null ? undefined : Math.round(progress * 100)}>
  {#if progress !== null}<div class="fill" style:transform={`scaleX(${progress})`}></div>{/if}
</div>

<style>
  .track { width: 100%; height: 100%; background: var(--c-surface-3); }
  .fill { width: 100%; height: 100%; background: var(--c-accent); transform-origin: left center; }
</style>
