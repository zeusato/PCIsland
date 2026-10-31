<script lang="ts">
  // Fallback for kinds without a dedicated view: label from i18n + actions.
  import type { Activity } from '$lib/ipc/bindings/Activity';
  import Info from '@lucide/svelte/icons/info';
  import ActionButtons from '$lib/components/ActionButtons.svelte';
  import { t } from '$lib/i18n/index.svelte';
  import { payloadOf } from '$lib/registry/payloads';

  let { activity, variant }: { activity: Activity; variant: string } = $props();
  const update = $derived(payloadOf(activity, 'app.update'));
</script>

<div class="generic" class:column={variant !== 'compact' && activity.actions.length > 0}>
  <Info size={16} />
  <span>{t(`alerts.${activity.kind}`)}{#if update} {update.version}{/if}</span>
  {#if variant !== 'compact' && activity.actions.length}<ActionButtons {activity} labels={variant === 'alert'} />{/if}
</div>

<style>
  .generic {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--sp-4);
    width: 100%;
    padding: 0 var(--sp-5);
    font-size: var(--fs-body-island);
  }
  .column {
    flex-direction: column;
  }
</style>
