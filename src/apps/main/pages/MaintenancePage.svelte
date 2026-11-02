<script lang="ts">
  // Cleanup page (docs/modules/maintenance.md): the same shared controller as the island panel,
  // so a running cleanup shows its progress in both places.
  import MaintenancePanel from '../../../features/maintenance/views/Panel.svelte';
  import { t } from '$lib/i18n/index.svelte';
</script>

<h1>{t('main.nav.maintenance')}</h1>
<MaintenancePanel wide />

<style>
  h1 { margin: 0 0 var(--sp-6); font-size: var(--fs-title); }
</style>
