<script lang="ts">
  // Full-window entry of the floating video player (docs/modules/video-player.md §3).
  import { onMount } from 'svelte';
  import Card from '$lib/components/Card.svelte';
  import Button from '$lib/components/Button.svelte';
  import { inTauri } from '$lib/ipc/commands';
  import { videoCommands } from '$lib/ipc/video';
  import type { ToolsStatus } from '$lib/ipc/bindings/ToolsStatus';
  import { t } from '$lib/i18n/index.svelte';
  import OpenPanel from '../../../features/video/views/OpenPanel.svelte';

  let tools = $state<ToolsStatus | null>(null);
  let updating = $state(false);
  let failed = $state(false);
  onMount(() => {
    if (inTauri) void videoCommands.toolsStatus().then((s) => (tools = s)).catch(() => {});
  });
  async function update() {
    updating = true;
    failed = false;
    try {
      tools = await videoCommands.installTools();
    } catch {
      failed = true;
    } finally {
      updating = false;
    }
  }
</script>

<h1>{t('main.nav.video')}</h1>

<Card>
  <OpenPanel />
</Card>

<Card title={t('video.tools.title')}>
  <p class="muted">
    {#if tools?.installed}{t('video.tools.version', { version: tools.version ?? '?' })}{:else}{t('video.tools.missing')}{/if}
  </p>
  <Button onclick={update} disabled={updating}>
    {updating ? t('video.tools.installing') : tools?.installed ? t('video.tools.update') : t('video.tools.install')}
  </Button>
  {#if failed}<p class="warn" role="alert">{t('video.error.install')}</p>{/if}
</Card>

<style>
  .muted {
    color: var(--c-text-2);
    font-size: var(--fs-small);
  }
  .warn {
    color: var(--c-warning);
    font-size: var(--fs-small);
  }
</style>
