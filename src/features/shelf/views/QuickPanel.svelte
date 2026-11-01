<script lang="ts">
  import { live } from '$lib/stores/live.svelte';
  import Slots from './Slots.svelte';
  import { t } from '$lib/i18n/index.svelte';
  let { close }: { close: () => void } = $props();
</script>

<div role="dialog" aria-label={t('main.nav.shelf')} tabindex="-1" onkeydown={(event) => { if (event.key === 'Escape') { event.stopPropagation(); close(); } }}>
  <Slots items={live.shelf} />
</div>
