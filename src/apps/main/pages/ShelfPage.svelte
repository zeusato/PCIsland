<script lang="ts">
  import { inTauri, shelfCommands } from '$lib/ipc/commands';
  import { live } from '$lib/stores/live.svelte';
  import { t } from '$lib/i18n/index.svelte';
  import Slots from '../../../features/shelf/views/Slots.svelte';
  $effect(() => { if (inTauri) void shelfCommands.list().then(items=>live.shelf=items); });
</script>
<h1>{t('main.nav.shelf')}</h1>
<Slots items={live.shelf} />
