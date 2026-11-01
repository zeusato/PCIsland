<script lang="ts">
  import type { Activity } from '$lib/ipc/bindings/Activity';
  import Package from '@lucide/svelte/icons/package';
  import { payloadOf } from '$lib/registry/payloads';
  import Slots from './Slots.svelte';
  let { activity, variant }: { activity: Activity; variant: string } = $props();
  const shelf = $derived(payloadOf(activity, 'shelf.items'));
</script>

{#if variant === 'peek'}
  <Slots items={shelf?.items ?? []} />
{:else}
  <div class="row"><Package size={16} /><span>{shelf?.count ?? 0}</span></div>
{/if}

<style>
  .row { display: flex; align-items: center; justify-content: space-between; width: 100%; padding: 0 var(--sp-5); color: var(--c-text-2); font-size: var(--fs-body-island); }
</style>
