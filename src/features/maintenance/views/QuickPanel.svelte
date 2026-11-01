<script lang="ts">
  import Panel from './Panel.svelte';
  import { t } from '$lib/i18n/index.svelte';
  let { close }: { close: () => void } = $props();
</script>
<div class="panel" role="dialog" aria-label={t('maintenance.title')} tabindex="-1" onkeydown={(e) => { if (e.key === 'Escape') { e.stopPropagation(); close(); } }}><Panel /></div>
<style>.panel { max-height: var(--island-list-max-h); overflow: auto; }</style>
